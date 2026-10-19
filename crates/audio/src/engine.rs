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
    flush_flag: Arc<AtomicBool>,
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
        self.flush_flag.store(true, Ordering::SeqCst);
        self.send(AudioCommand::Stop);
    }

    pub fn seek(&self, duration: Duration) {
        self.status.set_position_ms(duration.as_millis() as u64);
        self.flush_flag.store(true, Ordering::SeqCst);
        self.send(AudioCommand::Seek(duration));
    }

    pub fn load(&self, path: PathBuf, autoplay: bool) {
        self.flush_flag.store(true, Ordering::SeqCst);
        self.send(AudioCommand::Load { path, autoplay });
    }

    pub fn set_next_track(&self, path: Option<PathBuf>) {
        self.send(AudioCommand::SetNextTrack(path));
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
        let flush_flag = Arc::new(AtomicBool::new(false));
        let controller = AudioController {
            cmd_tx,
            status: status.clone(),
            flush_flag: flush_flag.clone(),
        };

        let worker_handle = thread::Builder::new()
            .name("audio-worker".to_string())
            .spawn(move || {
                run_audio_worker(cmd_rx, status, flush_flag);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackBoundary {
    pub switch_at_sample: u64,
    pub next_duration_ms: u64,
}

struct LoadedTrack {
    path: PathBuf,
    duration_ms: u64,
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

struct AudioStreamContext {
    _active_stream: Option<cpal::Stream>,
    output_latency_us: Option<Arc<AtomicU64>>,
    device_sample_rate: u32,
    device_channels: usize,
    ring_producer: Option<ringbuf::HeapProd<f32>>,
    boundary_producer: Option<ringbuf::HeapProd<TrackBoundary>>,
}

fn init_cpal_output_stream(
    status: &AudioStatus,
    flush_flag: &Arc<AtomicBool>,
) -> AudioStreamContext {
    let host = cpal::default_host();
    let device = host.default_output_device();

    let (stream, device_sample_rate, device_channels) = match device {
        Some(dev) => match setup_cpal_stream(&dev, status.clone(), flush_flag.clone()) {
            Ok((st, sr, ch, latency_us)) => {
                info!(
                    "Audio output stream initialized successfully ({} Hz, {} ch)",
                    sr, ch
                );
                (Some((st, latency_us)), sr, ch)
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

    let output_latency_us = stream.as_ref().map(|(_, l)| l.clone());
    let active_stream = stream.map(|(s, _)| s);
    let ring_producer = WORKER_PRODUCER.lock().unwrap().take();
    let boundary_producer = WORKER_BOUNDARY_PRODUCER.lock().unwrap().take();

    AudioStreamContext {
        _active_stream: active_stream,
        output_latency_us,
        device_sample_rate,
        device_channels,
        ring_producer,
        boundary_producer,
    }
}

fn ensure_stream<'a>(
    ctx: &'a mut Option<AudioStreamContext>,
    status: &AudioStatus,
    flush_flag: &Arc<AtomicBool>,
) -> &'a mut AudioStreamContext {
    ctx.get_or_insert_with(|| init_cpal_output_stream(status, flush_flag))
}

fn run_audio_worker(
    cmd_rx: Receiver<AudioCommand>,
    status: AudioStatus,
    flush_flag: Arc<AtomicBool>,
) {
    info!("Starting audio worker thread");

    let mut stream_ctx: Option<AudioStreamContext> = None;
    let mut loaded_track: Option<LoadedTrack> = None;
    let mut next_track: Option<LoadedTrack> = None;
    let mut written_samples: u64 = 0;
    let mut eof_deadline: Option<Instant> = None;

    loop {
        // 1. Process all pending commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            let is_playback_cmd = matches!(
                cmd,
                AudioCommand::Load { .. }
                    | AudioCommand::Play
                    | AudioCommand::TogglePlayPause
                    | AudioCommand::Seek(_)
                    | AudioCommand::SetNextTrack(_)
            );
            let (sr, ch, prod) = if is_playback_cmd || stream_ctx.is_some() {
                let ctx = ensure_stream(&mut stream_ctx, &status, &flush_flag);
                (
                    ctx.device_sample_rate,
                    ctx.device_channels,
                    &mut ctx.ring_producer,
                )
            } else {
                (44100, 2, &mut None)
            };
            handle_command(
                cmd,
                &status,
                &mut loaded_track,
                &mut next_track,
                &flush_flag,
                prod,
                &mut written_samples,
                sr,
                ch,
            );
        }

        // 2. Decode audio if playing and loaded
        if status.is_playing() {
            let ctx = ensure_stream(&mut stream_ctx, &status, &flush_flag);
            let device_channels = ctx.device_channels;
            let output_latency_us = &ctx.output_latency_us;

            let mut transition_to = None;
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
                                        &mut ctx.ring_producer,
                                        device_channels,
                                        &flush_flag,
                                        &mut written_samples,
                                    );
                                }
                            }
                        }
                        None => {
                            track.reached_eof = true;
                            let _ = track.decoder.send_eof();
                            drain_decoder_frames(
                                track,
                                &mut ctx.ring_producer,
                                device_channels,
                                &flush_flag,
                                &mut written_samples,
                            );
                            flush_resampler(
                                track,
                                &mut ctx.ring_producer,
                                device_channels,
                                &flush_flag,
                                &mut written_samples,
                            );

                            if let Some(next) = next_track.take() {
                                transition_to = Some(next);
                            }
                        }
                    }
                } else if let Some(next) = next_track.take() {
                    transition_to = Some(next);
                } else {
                    let is_empty = ctx
                        .ring_producer
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

            if let Some(next) = transition_to {
                info!(
                    "Transitioning gaplessly to next track {:?} (switch at sample {})",
                    next.path, written_samples
                );
                if let Some(ref mut bp) = ctx.boundary_producer {
                    let _ = bp.try_push(TrackBoundary {
                        switch_at_sample: written_samples,
                        next_duration_ms: next.duration_ms,
                    });
                }
                loaded_track = Some(next);
                eof_deadline = None;
            }
        } else {
            eof_deadline = None;

            // Not playing: block until next command or clean channel disconnection
            match cmd_rx.recv() {
                Ok(cmd) => {
                    let is_playback_cmd = matches!(
                        cmd,
                        AudioCommand::Load { .. }
                            | AudioCommand::Play
                            | AudioCommand::TogglePlayPause
                            | AudioCommand::Seek(_)
                            | AudioCommand::SetNextTrack(_)
                    );
                    let (sr, ch, prod) = if is_playback_cmd || stream_ctx.is_some() {
                        let ctx = ensure_stream(&mut stream_ctx, &status, &flush_flag);
                        (
                            ctx.device_sample_rate,
                            ctx.device_channels,
                            &mut ctx.ring_producer,
                        )
                    } else {
                        (44100, 2, &mut None)
                    };
                    handle_command(
                        cmd,
                        &status,
                        &mut loaded_track,
                        &mut next_track,
                        &flush_flag,
                        prod,
                        &mut written_samples,
                        sr,
                        ch,
                    );
                }
                Err(_) => {
                    // Channel disconnected, audio engine shut down
                    break;
                }
            }
        }
    }
}

fn drain_decoder_frames(
    track: &mut LoadedTrack,
    ring_producer: &mut Option<ringbuf::HeapProd<f32>>,
    device_channels: usize,
    flush_flag: &Arc<AtomicBool>,
    written_samples: &mut u64,
) {
    while track
        .decoder
        .receive_frame(&mut track.decoded_frame)
        .is_ok()
    {
        if flush_flag.load(Ordering::Relaxed) {
            break;
        }

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
                            if !push_to_ring_buffer(prod, float_slice, flush_flag, written_samples)
                            {
                                break;
                            }
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
    flush_flag: &Arc<AtomicBool>,
    written_samples: &mut u64,
) {
    while let Ok(Some(_)) = track.resampler.flush(&mut track.resampled_frame) {
        if flush_flag.load(Ordering::Relaxed) {
            break;
        }

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
                if !push_to_ring_buffer(prod, float_slice, flush_flag, written_samples) {
                    break;
                }
            }
        }
    }
}

fn push_to_ring_buffer(
    prod: &mut ringbuf::HeapProd<f32>,
    samples: &[f32],
    flush_flag: &Arc<AtomicBool>,
    written_samples: &mut u64,
) -> bool {
    let mut written = 0;
    let mut stalled_since: Option<Instant> = None;
    while written < samples.len() {
        if flush_flag.load(Ordering::Relaxed) {
            return false;
        }

        let n = prod.push_slice(&samples[written..]);
        written += n;
        *written_samples += n as u64;
        if written < samples.len() {
            if n == 0 {
                let start = *stalled_since.get_or_insert_with(Instant::now);
                // If unable to push any samples for >= 500ms (buffer full and consumer stalled),
                // break to prevent hanging the worker thread indefinitely.
                if start.elapsed() >= Duration::from_millis(500) {
                    warn!(
                        "Ring buffer full and not draining for 500ms, dropping remaining samples"
                    );
                    break;
                }
            } else {
                stalled_since = None;
            }
            // Buffer full — yield briefly and retry
            thread::sleep(Duration::from_millis(8));
        }
    }
    true
}

// Global slots to transfer producers created in CPAL stream setup to worker thread
static WORKER_PRODUCER: std::sync::Mutex<Option<ringbuf::HeapProd<f32>>> =
    std::sync::Mutex::new(None);
static WORKER_BOUNDARY_PRODUCER: std::sync::Mutex<Option<ringbuf::HeapProd<TrackBoundary>>> =
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

type CpalStreamSetup = (cpal::Stream, u32, usize, Arc<AtomicU64>);

/// Shared, per-stream state handed to the realtime output callbacks.
pub struct CallbackState<'a> {
    pub consumer: &'a mut ringbuf::HeapCons<f32>,
    pub boundary_consumer: &'a mut ringbuf::HeapCons<TrackBoundary>,
    pub current_boundary: &'a mut Option<TrackBoundary>,
    pub consumed_samples: &'a mut u64,
    pub flush_flag: &'a AtomicBool,
    pub status: &'a AudioStatus,
    pub sample_rate: u32,
    pub channels: usize,
    pub pos_rem: &'a mut u64,
}

pub fn pop_samples_with_boundary(data: &mut [f32], state: &mut CallbackState) -> usize {
    let mut total_read = 0;

    while total_read < data.len() {
        if state.current_boundary.is_none() {
            *state.current_boundary = state.boundary_consumer.try_pop();
        }

        if let Some(ref mut b) = state.current_boundary {
            let target = b.switch_at_sample;
            if *state.consumed_samples >= target {
                let next_dur = b.next_duration_ms;
                *state.current_boundary = None;
                state.status.set_duration_ms(next_dur);
                state.status.set_position_ms(0);
                *state.pos_rem = 0;
                state.status.set_gapless_transition(true);
                continue;
            }

            let samples_until_switch = (target - *state.consumed_samples) as usize;
            let needed = data.len() - total_read;
            let to_read = needed.min(samples_until_switch);

            let read = state
                .consumer
                .pop_slice(&mut data[total_read..total_read + to_read]);
            if read > 0 {
                advance_position(
                    state.status,
                    read,
                    state.channels,
                    state.sample_rate,
                    state.pos_rem,
                );
                *state.consumed_samples += read as u64;
                total_read += read;
            }

            if *state.consumed_samples >= target {
                let next_dur = b.next_duration_ms;
                *state.current_boundary = None;
                state.status.set_duration_ms(next_dur);
                state.status.set_position_ms(0);
                *state.pos_rem = 0;
                state.status.set_gapless_transition(true);
            }

            if read < to_read {
                break;
            }
        } else {
            let read = state.consumer.pop_slice(&mut data[total_read..]);
            if read > 0 {
                advance_position(
                    state.status,
                    read,
                    state.channels,
                    state.sample_rate,
                    state.pos_rem,
                );
                *state.consumed_samples += read as u64;
                total_read += read;
            }
            break;
        }
    }

    total_read
}

fn setup_cpal_stream(
    device: &cpal::Device,
    status: AudioStatus,
    flush_flag: Arc<AtomicBool>,
) -> Result<CpalStreamSetup, AudioEngineError> {
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;

    let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
    let (producer, mut consumer) = rb.split();

    let boundary_rb = HeapRb::<TrackBoundary>::new(16);
    let (boundary_producer, mut boundary_consumer) = boundary_rb.split();

    *WORKER_PRODUCER.lock().unwrap() = Some(producer);
    *WORKER_BOUNDARY_PRODUCER.lock().unwrap() = Some(boundary_producer);

    let flush_cb = flush_flag.clone();
    let status_cb = status.clone();

    let latency_us = Arc::new(AtomicU64::new(0));
    let latency_cb = latency_us.clone();

    // Sub-millisecond remainder carried between callbacks (see advance_position).
    let mut pos_rem: u64 = 0;
    let mut current_boundary: Option<TrackBoundary> = None;
    let mut consumed_samples: u64 = 0;

    let err_fn = |err| error!("CPAL output stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                config.into(),
                move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                    record_output_latency(info, &latency_cb);
                    let mut state = CallbackState {
                        consumer: &mut consumer,
                        boundary_consumer: &mut boundary_consumer,
                        current_boundary: &mut current_boundary,
                        consumed_samples: &mut consumed_samples,
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
                            boundary_consumer: &mut boundary_consumer,
                            current_boundary: &mut current_boundary,
                            consumed_samples: &mut consumed_samples,
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
                            boundary_consumer: &mut boundary_consumer,
                            current_boundary: &mut current_boundary,
                            consumed_samples: &mut consumed_samples,
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

    Ok((stream, sample_rate, channels, latency_us))
}

fn audio_callback_f32(data: &mut [f32], state: &mut CallbackState) {
    if state.flush_flag.swap(false, Ordering::SeqCst) {
        state.consumer.clear();
        *state.pos_rem = 0;
        *state.consumed_samples = 0;
        state.boundary_consumer.clear();
        *state.current_boundary = None;
    }

    if !state.status.is_playing() {
        data.fill(0.0);
        return;
    }

    let read_count = pop_samples_with_boundary(data, state);
    if read_count < data.len() {
        data[read_count..].fill(0.0);
    }

    let vol = state.status.volume();
    if (vol - 1.0).abs() > 0.001 {
        for sample in &mut data[..read_count] {
            *sample *= vol;
        }
    }
}

fn audio_callback_i16(data: &mut [i16], scratch: &mut Vec<f32>, state: &mut CallbackState) {
    if state.flush_flag.swap(false, Ordering::SeqCst) {
        state.consumer.clear();
        *state.pos_rem = 0;
        *state.consumed_samples = 0;
        state.boundary_consumer.clear();
        *state.current_boundary = None;
    }

    if !state.status.is_playing() {
        data.fill(0);
        return;
    }

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let scratch_slice = &mut scratch[..data.len()];
    let read_count = pop_samples_with_boundary(scratch_slice, state);
    if read_count < data.len() {
        scratch_slice[read_count..].fill(0.0);
    }

    let vol = state.status.volume();
    for (i, sample) in data.iter_mut().enumerate() {
        let clamped = (scratch_slice[i] * vol).clamp(-1.0, 1.0);
        *sample = (clamped * i16::MAX as f32) as i16;
    }
}

fn audio_callback_u16(data: &mut [u16], scratch: &mut Vec<f32>, state: &mut CallbackState) {
    if state.flush_flag.swap(false, Ordering::SeqCst) {
        state.consumer.clear();
        *state.pos_rem = 0;
        *state.consumed_samples = 0;
        state.boundary_consumer.clear();
        *state.current_boundary = None;
    }

    if !state.status.is_playing() {
        data.fill(u16::MAX / 2);
        return;
    }

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let scratch_slice = &mut scratch[..data.len()];
    let read_count = pop_samples_with_boundary(scratch_slice, state);
    if read_count < data.len() {
        scratch_slice[read_count..].fill(0.0);
    }

    let vol = state.status.volume();
    for (i, sample) in data.iter_mut().enumerate() {
        let clamped = (scratch_slice[i] * vol).clamp(-1.0, 1.0);
        *sample = (((clamped + 1.0) * 0.5) * u16::MAX as f32) as u16;
    }
}

fn flush_audio_buffer(flush_flag: &Arc<AtomicBool>) {
    flush_flag.store(true, Ordering::SeqCst);
    let start = std::time::Instant::now();
    while flush_flag.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(1));
        if start.elapsed() > Duration::from_millis(100) {
            warn!("Audio buffer flush timed out after 100ms");
            flush_flag.store(false, Ordering::SeqCst);
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_command(
    cmd: AudioCommand,
    status: &AudioStatus,
    loaded_track: &mut Option<LoadedTrack>,
    next_track: &mut Option<LoadedTrack>,
    flush_flag: &Arc<AtomicBool>,
    _ring_producer: &mut Option<ringbuf::HeapProd<f32>>,
    written_samples: &mut u64,
    device_sample_rate: u32,
    device_channels: usize,
) {
    match cmd {
        AudioCommand::Load { path, autoplay } => {
            info!("Loading audio file: {:?}", path);
            *next_track = None;
            match load_track(&path, device_sample_rate, device_channels) {
                Ok(track) => {
                    // Flush any stale audio from the previous track and wait for the consumer
                    // callback to acknowledge it before loading the new track and decoding samples.
                    flush_audio_buffer(flush_flag);
                    *written_samples = 0;

                    let duration_ms = track.duration_ms;
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
        AudioCommand::SetNextTrack(maybe_path) => match maybe_path {
            Some(path) => {
                if next_track.as_ref().map(|t| &t.path) == Some(&path) {
                    return;
                }
                info!("Preloading next track for gapless audio: {:?}", path);
                match load_track(&path, device_sample_rate, device_channels) {
                    Ok(track) => {
                        info!("Preloaded next track successfully: {:?}", track.path);
                        *next_track = Some(track);
                    }
                    Err(e) => {
                        warn!("Failed to preload next track {:?}: {}", path, e);
                        *next_track = None;
                    }
                }
            }
            None => {
                *next_track = None;
            }
        },
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
            *next_track = None;
            if let Some(track) = loaded_track.as_mut() {
                let _ = track.input.seek(0, ..);
                track.decoder.flush();
                while let Ok(Some(_)) = track.resampler.flush(&mut track.resampled_frame) {
                    if track.resampled_frame.samples() == 0 {
                        break;
                    }
                }
                track.reached_eof = false;
            }
            flush_audio_buffer(flush_flag);
            *written_samples = 0;
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
                        while let Ok(Some(_)) = track.resampler.flush(&mut track.resampled_frame) {
                            if track.resampled_frame.samples() == 0 {
                                break;
                            }
                        }
                        track.reached_eof = false;
                        flush_audio_buffer(flush_flag);
                        *written_samples = 0;

                        if let Some(ref mut next) = next_track {
                            let _ = next.input.seek(0, ..);
                            next.decoder.flush();
                            next.reached_eof = false;
                        }

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
) -> Result<LoadedTrack, Box<dyn std::error::Error + Send + Sync>> {
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
        path: path.to_path_buf(),
        duration_ms,
        input,
        decoder,
        stream_index,
        resampler,
        reached_eof: false,
        decoded_frame: AudioFrame::empty(),
        resampled_frame: AudioFrame::empty(),
    };

    Ok(loaded)
}
