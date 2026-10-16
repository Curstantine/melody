use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ffmpeg_next::format::context::Input;
use ffmpeg_next::media::Type;
use ffmpeg_next::software::resampling::context::Context as ResamplerContext;
use ffmpeg_next::util::channel_layout::ChannelLayout;
use ffmpeg_next::util::format::sample::Type as SampleType;
use ffmpeg_next::util::format::Sample;
use ffmpeg_next::util::frame::Audio as AudioFrame;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use thiserror::Error;
use tracing::{error, info, warn};

use crate::commands::AudioCommand;
use crate::status::AudioStatus;

/// ~1.5 seconds of stereo f32 at 48 kHz: 48_000 * 2 * 1.5 = 144_000 samples.
/// Rounded up to the next power of two for ringbuf efficiency (2^18).
const RING_BUFFER_CAPACITY: usize = 262_144;

/// Extra time to wait after the ring buffer empties at end-of-track when the
/// device has not reported its output latency yet.
const FALLBACK_OUTPUT_LATENCY: Duration = Duration::from_millis(50);

#[derive(Error, Debug)]
pub enum AudioEngineError {
    #[error("No audio output device available")]
    NoDevice,
    #[error("CPAL device error: {0}")]
    CpalDevice(#[from] cpal::Error),
    #[error("CPAL stream error: {0}")]
    CpalStream(cpal::Error),
    #[error("CPAL play error: {0}")]
    CpalPlay(cpal::Error),
}

#[derive(Clone)]
pub struct AudioController {
    cmd_tx: Sender<AudioCommand>,
    status: AudioStatus,
}

impl AudioController {
    pub fn send(&self, cmd: AudioCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn play(&self) {
        self.send(AudioCommand::Play);
    }

    pub fn pause(&self) {
        self.send(AudioCommand::Pause);
    }

    pub fn toggle_play_pause(&self) {
        self.send(AudioCommand::TogglePlayPause);
    }

    pub fn stop(&self) {
        self.send(AudioCommand::Stop);
    }

    pub fn seek(&self, duration: Duration) {
        self.send(AudioCommand::Seek(duration));
    }

    pub fn load(&self, path: PathBuf, autoplay: bool) {
        self.send(AudioCommand::Load { path, autoplay });
    }

    pub fn set_volume(&self, volume: f32) {
        self.send(AudioCommand::SetVolume(volume));
    }

    pub fn status(&self) -> &AudioStatus {
        &self.status
    }
}

pub struct AudioEngine {
    controller: AudioController,
    _worker_handle: Option<JoinHandle<()>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = unbounded::<AudioCommand>();
        let status = AudioStatus::new();
        let controller = AudioController {
            cmd_tx,
            status: status.clone(),
        };

        let worker_handle = thread::Builder::new()
            .name("audio-worker".to_string())
            .spawn(move || {
                run_audio_worker(cmd_rx, status);
            })
            .ok();

        Self {
            controller,
            _worker_handle: worker_handle,
        }
    }

    pub fn controller(&self) -> AudioController {
        self.controller.clone()
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn init_ffmpeg() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = ffmpeg_next::init();
    });
}

struct LoadedTrack {
    input: Input,
    decoder: ffmpeg_next::codec::decoder::Audio,
    stream_index: usize,
    resampler: ResamplerContext,
    reached_eof: bool,
    decoded_frame: AudioFrame,
    resampled_frame: AudioFrame,
}

fn target_channel_layout(channels: usize) -> ChannelLayout {
    match channels {
        1 => ChannelLayout::MONO,
        2 => ChannelLayout::STEREO,
        3 => ChannelLayout::SURROUND,
        4 => ChannelLayout::QUAD,
        5 => ChannelLayout::_5POINT0,
        6 => ChannelLayout::_5POINT1,
        8 => ChannelLayout::_7POINT1,
        n => ChannelLayout::default(n as i32),
    }
}

fn run_audio_worker(cmd_rx: Receiver<AudioCommand>, status: AudioStatus) {
    info!("Starting audio worker thread");

    // Initialize CPAL device and output stream
    let host = cpal::default_host();
    let device = host.default_output_device();

    let (stream, device_sample_rate, device_channels) = match device {
        Some(dev) => match setup_cpal_stream(&dev, status.clone()) {
            Ok((st, sr, ch, flush_flag, latency_us)) => {
                info!(
                    "Audio output stream initialized successfully ({} Hz, {} ch)",
                    sr, ch
                );
                (Some((st, flush_flag, latency_us)), sr, ch)
            }
            Err(e) => {
                warn!(
                    "Failed to setup CPAL stream: {}. Audio playback will be muted.",
                    e
                );
                (None, 44100, 2)
            }
        },
        None => {
            warn!("No default audio output device found. Audio playback will be muted.");
            (None, 44100, 2)
        }
    };

    let flush_flag = stream.as_ref().map(|(_, f, _)| f.clone());
    let output_latency_us = stream.as_ref().map(|(_, _, l)| l.clone());
    let _active_stream = stream.map(|(s, _, _)| s);

    let mut loaded_track: Option<LoadedTrack> = None;

    // When the decoder has hit EOF and the ring buffer has emptied, the last
    // callback buffer and the device's output latency are still in flight.
    // This holds the instant at which playback has genuinely finished.
    let mut eof_deadline: Option<Instant> = None;

    // SPSC ring buffer for decoding worker -> cpal callback
    // Note: The producer is held by the worker
    let mut ring_producer = WORKER_PRODUCER.lock().unwrap().take();

    loop {
        // 1. Process all pending commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            handle_command(
                cmd,
                &status,
                &mut loaded_track,
                &flush_flag,
                &mut ring_producer,
                device_sample_rate,
                device_channels,
            );
        }

        // 2. Decode audio if playing and loaded
        if status.is_playing() {
            if let Some(track) = loaded_track.as_mut() {
                if !track.reached_eof {
                    eof_deadline = None;

                    // Try to read next packet from input
                    match track.input.packets().next() {
                        Some((stream, packet)) => {
                            if stream.index() == track.stream_index {
                                if let Err(e) = track.decoder.send_packet(&packet) {
                                    warn!("Audio decoder send_packet error: {}", e);
                                } else {
                                    drain_decoder_frames(
                                        track,
                                        &mut ring_producer,
                                        device_channels,
                                    );
                                }
                            }
                        }
                        None => {
                            // Demuxer reached end of packets, send EOF to flush decoder
                            track.reached_eof = true;
                            let _ = track.decoder.send_eof();
                            drain_decoder_frames(track, &mut ring_producer, device_channels);
                            flush_resampler(track, &mut ring_producer, device_channels);
                        }
                    }
                } else {
                    // Reached EOF: wait until ring buffer is completely drained.
                    let is_empty = ring_producer
                        .as_ref()
                        .map(|p| p.occupied_len() == 0)
                        .unwrap_or(true);

                    if is_empty {
                        // The ring buffer is empty, but the last chunk the callback
                        // popped is still being played out by the device. Wait for
                        // the reported output latency before declaring the track done.
                        let now = Instant::now();
                        let deadline = *eof_deadline.get_or_insert_with(|| {
                            let reported = output_latency_us
                                .as_ref()
                                .map(|l| Duration::from_micros(l.load(Ordering::Relaxed)))
                                .unwrap_or(Duration::ZERO);
                            let latency = if reported.is_zero() {
                                FALLBACK_OUTPUT_LATENCY
                            } else {
                                reported
                            };
                            now + latency
                        });

                        if now >= deadline {
                            eof_deadline = None;
                            if status.duration_ms() > 0 {
                                status.set_position_ms(status.duration_ms());
                            }
                            status.set_playing(false);
                            status.set_ended(true);
                            info!("Track finished playing");
                        } else {
                            thread::sleep(Duration::from_millis(5));
                        }
                    } else {
                        eof_deadline = None;
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            } else {
                status.set_playing(false);
                thread::sleep(Duration::from_millis(20));
            }
        } else {
            eof_deadline = None;

            // Not playing: block or wait briefly for next command
            if let Ok(cmd) = cmd_rx.recv_timeout(Duration::from_millis(50)) {
                handle_command(
                    cmd,
                    &status,
                    &mut loaded_track,
                    &flush_flag,
                    &mut ring_producer,
                    device_sample_rate,
                    device_channels,
                );
            }
        }
    }
}

fn drain_decoder_frames(
    track: &mut LoadedTrack,
    ring_producer: &mut Option<ringbuf::HeapProd<f32>>,
    device_channels: usize,
) {
    while track
        .decoder
        .receive_frame(&mut track.decoded_frame)
        .is_ok()
    {
        // Resample decoded frame to destination format/rate/channels
        match track
            .resampler
            .run(&track.decoded_frame, &mut track.resampled_frame)
        {
            Ok(_) => {
                let samples_count = track.resampled_frame.samples();
                if samples_count > 0 {
                    let total_floats = samples_count * device_channels;
                    let data_bytes = track.resampled_frame.data(0);
                    let byte_count = total_floats * std::mem::size_of::<f32>();
                    if data_bytes.len() >= byte_count {
                        let float_slice: &[f32] = unsafe {
                            std::slice::from_raw_parts(
                                data_bytes.as_ptr() as *const f32,
                                total_floats,
                            )
                        };

                        if let Some(ref mut prod) = ring_producer {
                            push_to_ring_buffer(prod, float_slice);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Resampling error: {}", e);
            }
        }
    }
}

fn flush_resampler(
    track: &mut LoadedTrack,
    ring_producer: &mut Option<ringbuf::HeapProd<f32>>,
    device_channels: usize,
) {
    while let Ok(Some(_)) = track.resampler.flush(&mut track.resampled_frame) {
        let samples_count = track.resampled_frame.samples();
        if samples_count == 0 {
            break;
        }
        let total_floats = samples_count * device_channels;
        let data_bytes = track.resampled_frame.data(0);
        let byte_count = total_floats * std::mem::size_of::<f32>();
        if data_bytes.len() >= byte_count {
            let float_slice: &[f32] = unsafe {
                std::slice::from_raw_parts(data_bytes.as_ptr() as *const f32, total_floats)
            };

            if let Some(ref mut prod) = ring_producer {
                push_to_ring_buffer(prod, float_slice);
            }
        }
    }
}

fn push_to_ring_buffer(prod: &mut ringbuf::HeapProd<f32>, samples: &[f32]) {
    let mut written = 0;
    let mut retries = 0;
    while written < samples.len() {
        let n = prod.push_slice(&samples[written..]);
        written += n;
        if written < samples.len() {
            if n == 0 {
                retries += 1;
                // If unable to push any samples for ~500ms (buffer full and consumer stalled),
                // break to prevent hanging the worker thread indefinitely.
                if retries > 500 {
                    warn!(
                        "Ring buffer full and not draining for 500ms, dropping remaining samples"
                    );
                    break;
                }
            } else {
                retries = 0;
            }
            // Buffer full — yield briefly and retry
            thread::sleep(Duration::from_millis(1));
        }
    }
}

// Global slot to transfer producer created in CPAL stream setup to worker thread
static WORKER_PRODUCER: std::sync::Mutex<Option<ringbuf::HeapProd<f32>>> =
    std::sync::Mutex::new(None);

/// Records how far in the future the buffer handed to the callback will actually
/// be heard (playback timestamp minus callback timestamp).
fn record_output_latency(info: &cpal::OutputCallbackInfo, latency_us: &AtomicU64) {
    let ts = info.timestamp();
    let latency = ts.playback.duration_since(ts.callback);
    latency_us.store(latency.as_micros() as u64, Ordering::Relaxed);
}

/// Advances the playback position by the number of frames just consumed.
///
/// Integer division per callback would truncate away a fraction of a
/// millisecond every time (≈0.67 ms per ~10.67 ms callback at 48 kHz), so the
/// remainder, expressed in "frame-milliseconds" (ms * sample_rate), is carried
/// forward in `rem` and the position never drifts.
fn advance_position(
    status: &AudioStatus,
    read_count: usize,
    channels: usize,
    sample_rate: u32,
    rem: &mut u64,
) {
    if read_count > 0 && channels > 0 && sample_rate > 0 {
        let frames_played = (read_count / channels) as u64;
        let rate = sample_rate as u64;
        let total = *rem + frames_played * 1000;
        let elapsed_ms = total / rate;
        *rem = total % rate;
        if elapsed_ms > 0 {
            let dur = status.duration_ms();
            let new_pos = status.position_ms() + elapsed_ms;
            let clamped = if dur > 0 { new_pos.min(dur) } else { new_pos };
            status.set_position_ms(clamped);
        }
    }
}

type CpalStreamSetup = (cpal::Stream, u32, usize, Arc<AtomicBool>, Arc<AtomicU64>);

/// Shared, per-stream state handed to the realtime output callbacks.
struct CallbackState<'a> {
    consumer: &'a mut ringbuf::HeapCons<f32>,
    flush_flag: &'a AtomicBool,
    status: &'a AudioStatus,
    sample_rate: u32,
    channels: usize,
    pos_rem: &'a mut u64,
}

fn setup_cpal_stream(
    device: &cpal::Device,
    status: AudioStatus,
) -> Result<CpalStreamSetup, AudioEngineError> {
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;

    let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
    let (producer, mut consumer) = rb.split();

    *WORKER_PRODUCER.lock().unwrap() = Some(producer);

    let flush_flag = Arc::new(AtomicBool::new(false));
    let flush_cb = flush_flag.clone();
    let status_cb = status.clone();

    let latency_us = Arc::new(AtomicU64::new(0));
    let latency_cb = latency_us.clone();

    // Sub-millisecond remainder carried between callbacks (see advance_position).
    let mut pos_rem: u64 = 0;

    let err_fn = |err| error!("CPAL output stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                config.into(),
                move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                    record_output_latency(info, &latency_cb);
                    let mut state = CallbackState {
                        consumer: &mut consumer,
                        flush_flag: &flush_cb,
                        status: &status_cb,
                        sample_rate,
                        channels,
                        pos_rem: &mut pos_rem,
                    };
                    audio_callback_f32(data, &mut state);
                },
                err_fn,
                None,
            )
            .map_err(AudioEngineError::CpalStream)?,
        cpal::SampleFormat::I16 => {
            let mut scratch = Vec::<f32>::new();
            device
                .build_output_stream(
                    config.into(),
                    move |data: &mut [i16], info: &cpal::OutputCallbackInfo| {
                        record_output_latency(info, &latency_cb);
                        let mut state = CallbackState {
                            consumer: &mut consumer,
                            flush_flag: &flush_cb,
                            status: &status_cb,
                            sample_rate,
                            channels,
                            pos_rem: &mut pos_rem,
                        };
                        audio_callback_i16(data, &mut scratch, &mut state);
                    },
                    err_fn,
                    None,
                )
                .map_err(AudioEngineError::CpalStream)?
        }
        cpal::SampleFormat::U16 => {
            let mut scratch = Vec::<f32>::new();
            device
                .build_output_stream(
                    config.into(),
                    move |data: &mut [u16], info: &cpal::OutputCallbackInfo| {
                        record_output_latency(info, &latency_cb);
                        let mut state = CallbackState {
                            consumer: &mut consumer,
                            flush_flag: &flush_cb,
                            status: &status_cb,
                            sample_rate,
                            channels,
                            pos_rem: &mut pos_rem,
                        };
                        audio_callback_u16(data, &mut scratch, &mut state);
                    },
                    err_fn,
                    None,
                )
                .map_err(AudioEngineError::CpalStream)?
        }
        _ => return Err(AudioEngineError::NoDevice),
    };

    stream.play().map_err(AudioEngineError::CpalPlay)?;

    Ok((stream, sample_rate, channels, flush_flag, latency_us))
}

fn audio_callback_f32(data: &mut [f32], state: &mut CallbackState) {
    if state.flush_flag.swap(false, Ordering::SeqCst) {
        state.consumer.clear();
    }

    if !state.status.is_playing() {
        data.fill(0.0);
        return;
    }

    let read_count = state.consumer.pop_slice(data);
    if read_count < data.len() {
        data[read_count..].fill(0.0);
    }

    let vol = state.status.volume();
    if (vol - 1.0).abs() > 0.001 {
        for sample in &mut data[..read_count] {
            *sample *= vol;
        }
    }

    advance_position(
        state.status,
        read_count,
        state.channels,
        state.sample_rate,
        state.pos_rem,
    );
}

fn audio_callback_i16(data: &mut [i16], scratch: &mut Vec<f32>, state: &mut CallbackState) {
    if state.flush_flag.swap(false, Ordering::SeqCst) {
        state.consumer.clear();
    }

    if !state.status.is_playing() {
        data.fill(0);
        return;
    }

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let scratch_slice = &mut scratch[..data.len()];
    let read_count = state.consumer.pop_slice(scratch_slice);
    if read_count < data.len() {
        scratch_slice[read_count..].fill(0.0);
    }

    let vol = state.status.volume();
    for (i, sample) in data.iter_mut().enumerate() {
        let clamped = (scratch_slice[i] * vol).clamp(-1.0, 1.0);
        *sample = (clamped * i16::MAX as f32) as i16;
    }

    advance_position(
        state.status,
        read_count,
        state.channels,
        state.sample_rate,
        state.pos_rem,
    );
}

fn audio_callback_u16(data: &mut [u16], scratch: &mut Vec<f32>, state: &mut CallbackState) {
    if state.flush_flag.swap(false, Ordering::SeqCst) {
        state.consumer.clear();
    }

    if !state.status.is_playing() {
        data.fill(u16::MAX / 2);
        return;
    }

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let scratch_slice = &mut scratch[..data.len()];
    let read_count = state.consumer.pop_slice(scratch_slice);
    if read_count < data.len() {
        scratch_slice[read_count..].fill(0.0);
    }

    let vol = state.status.volume();
    for (i, sample) in data.iter_mut().enumerate() {
        let clamped = (scratch_slice[i] * vol).clamp(-1.0, 1.0);
        *sample = (((clamped + 1.0) * 0.5) * u16::MAX as f32) as u16;
    }

    advance_position(
        state.status,
        read_count,
        state.channels,
        state.sample_rate,
        state.pos_rem,
    );
}

fn flush_audio_buffer(flush_flag: &Option<Arc<AtomicBool>>) {
    if let Some(ref f) = flush_flag {
        f.store(true, Ordering::SeqCst);
        let start = std::time::Instant::now();
        while f.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1));
            if start.elapsed() > Duration::from_millis(100) {
                warn!("Audio buffer flush timed out after 100ms");
                f.store(false, Ordering::SeqCst);
                break;
            }
        }
    }
}

fn handle_command(
    cmd: AudioCommand,
    status: &AudioStatus,
    loaded_track: &mut Option<LoadedTrack>,
    flush_flag: &Option<Arc<AtomicBool>>,
    _ring_producer: &mut Option<ringbuf::HeapProd<f32>>,
    device_sample_rate: u32,
    device_channels: usize,
) {
    match cmd {
        AudioCommand::Load { path, autoplay } => {
            info!("Loading audio file: {:?}", path);
            match load_track(&path, device_sample_rate, device_channels) {
                Ok((track, duration_ms)) => {
                    // Flush any stale audio from the previous track and wait for the consumer
                    // callback to acknowledge it before loading the new track and decoding samples.
                    flush_audio_buffer(flush_flag);

                    *loaded_track = Some(track);
                    status.set_duration_ms(duration_ms);
                    status.set_position_ms(0);
                    status.set_loaded(true);
                    status.set_ended(false);
                    status.set_playing(autoplay);
                    info!("Track loaded successfully. Autoplay: {}", autoplay);
                }
                Err(e) => {
                    error!("Failed to load audio file {:?}: {}", path, e);
                    status.set_loaded(false);
                    status.set_playing(false);
                }
            }
        }
        AudioCommand::Play => {
            if status.is_loaded() {
                status.set_playing(true);
            }
        }
        AudioCommand::Pause => {
            status.set_playing(false);
        }
        AudioCommand::TogglePlayPause => {
            if status.is_loaded() {
                status.set_playing(!status.is_playing());
            }
        }
        AudioCommand::Stop => {
            status.set_playing(false);
            status.set_position_ms(0);
            if let Some(track) = loaded_track.as_mut() {
                let _ = track.input.seek(0, ..);
                track.decoder.flush();
                track.reached_eof = false;
            }
            flush_audio_buffer(flush_flag);
        }
        AudioCommand::Seek(duration) => {
            if let Some(track) = loaded_track.as_mut() {
                let dur_ms = status.duration_ms();
                let target_dur = if dur_ms > 0 {
                    duration.min(Duration::from_millis(dur_ms))
                } else {
                    duration
                };
                // ffmpeg-next input.seek passes stream_index = -1 to avformat_seek_file,
                // which expects timestamp in AV_TIME_BASE units (microseconds).
                let seek_micros = target_dur.as_micros() as i64;

                match track.input.seek(seek_micros, ..) {
                    Ok(_) => {
                        track.decoder.flush();
                        track.reached_eof = false;
                        flush_audio_buffer(flush_flag);
                        status.set_position_ms(target_dur.as_millis() as u64);
                        info!("Seeked to {:?}", target_dur);
                    }
                    Err(e) => {
                        warn!("Failed to seek: {}", e);
                    }
                }
            }
        }
        AudioCommand::SetVolume(vol) => {
            status.set_volume(vol);
        }
    }
}

fn load_track(
    path: &Path,
    dst_sample_rate: u32,
    dst_channels: usize,
) -> Result<(LoadedTrack, u64), Box<dyn std::error::Error + Send + Sync>> {
    init_ffmpeg();

    let input = ffmpeg_next::format::input(path)?;
    let audio_stream = input
        .streams()
        .best(Type::Audio)
        .ok_or("No audio stream found in file")?;

    let stream_index = audio_stream.index();
    let time_base = audio_stream.time_base();

    // Duration calculation
    let duration_ms = if audio_stream.duration() > 0 {
        let tb = f64::from(time_base.numerator()) / f64::from(time_base.denominator());
        (audio_stream.duration() as f64 * tb * 1000.0) as u64
    } else if input.duration() > 0 {
        (input.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE) * 1000.0) as u64
    } else {
        0
    };

    let context_decoder =
        ffmpeg_next::codec::context::Context::from_parameters(audio_stream.parameters())?;
    let decoder = context_decoder.decoder().audio()?;

    let src_format = decoder.format();
    let src_channel_layout = if decoder.channel_layout().is_empty() {
        let ch = if decoder.channels() > 0 {
            decoder.channels() as usize
        } else {
            2
        };
        target_channel_layout(ch)
    } else {
        decoder.channel_layout()
    };
    let src_rate = if decoder.rate() > 0 {
        decoder.rate()
    } else {
        44100
    };

    let dst_format = Sample::F32(SampleType::Packed);
    let dst_channel_layout = target_channel_layout(dst_channels);

    let resampler = ResamplerContext::get(
        src_format,
        src_channel_layout,
        src_rate,
        dst_format,
        dst_channel_layout,
        dst_sample_rate,
    )?;

    let loaded = LoadedTrack {
        input,
        decoder,
        stream_index,
        resampler,
        reached_eof: false,
        decoded_frame: AudioFrame::empty(),
        resampled_frame: AudioFrame::empty(),
    };

    Ok((loaded, duration_ms))
}
