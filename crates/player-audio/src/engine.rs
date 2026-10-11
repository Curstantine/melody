use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use thiserror::Error;
use tracing::{error, info, warn};

use crate::commands::AudioCommand;
use crate::resampler::AudioConverter;
use crate::status::AudioStatus;

/// 3 seconds of stereo f32 at 48 kHz: 48_000 * 2 * 3 = 288_000 samples.
/// Rounded up to the next power of two for ringbuf efficiency.
const RING_BUFFER_CAPACITY: usize = 524_288;

#[derive(Error, Debug)]
pub enum AudioEngineError {
    #[error("No audio output device available")]
    NoDevice,
    #[error("CPAL device error: {0}")]
    CpalDevice(#[from] cpal::DefaultStreamConfigError),
    #[error("CPAL stream error: {0}")]
    CpalStream(#[from] cpal::BuildStreamError),
    #[error("CPAL play error: {0}")]
    CpalPlay(#[from] cpal::PlayStreamError),
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

struct LoadedTrack {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_buf: Option<SampleBuffer<f32>>,
    converter: AudioConverter,
    reached_eof: bool,
}

fn run_audio_worker(cmd_rx: Receiver<AudioCommand>, status: AudioStatus) {
    info!("Starting audio worker thread");

    // Initialize CPAL device and output stream
    let host = cpal::default_host();
    let device = host.default_output_device();

    let (stream, device_sample_rate, device_channels) = match device {
        Some(dev) => match setup_cpal_stream(&dev, status.clone()) {
            Ok((st, sr, ch, flush_flag)) => {
                info!(
                    "Audio output stream initialized successfully ({} Hz, {} ch)",
                    sr, ch
                );
                (Some((st, flush_flag)), sr, ch)
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

    let flush_flag = stream.as_ref().map(|(_, f)| f.clone());
    let _active_stream = stream.map(|(s, _)| s);

    let mut loaded_track: Option<LoadedTrack> = None;
    let mut temp_converted = Vec::new();

    // SPSC ring buffer for decoding worker -> cpal callback
    // (CPAL stream consumes from this if initialized, or mock drained)
    // Note: The producer is held by the worker
    let mut ring_producer = if let Some(producer) = WORKER_PRODUCER.lock().unwrap().take() {
        Some(producer)
    } else {
        None
    };

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
                    match track.format.next_packet() {
                        Ok(packet) => {
                            if packet.track_id() == track.track_id {
                                match track.decoder.decode(&packet) {
                                    Ok(decoded) => {
                                        if track.sample_buf.is_none() {
                                            track.sample_buf = Some(SampleBuffer::new(
                                                decoded.capacity() as u64,
                                                *decoded.spec(),
                                            ));
                                        }

                                        if let Some(ref mut sbuf) = track.sample_buf {
                                            sbuf.copy_interleaved_ref(decoded);
                                            temp_converted.clear();
                                            track
                                                .converter
                                                .convert(sbuf.samples(), &mut temp_converted);

                                            if let Some(ref mut prod) = ring_producer {
                                                // Backpressure: keep retrying until all converted
                                                // samples are written into the ring buffer.
                                                // This prevents the sample-dropping that caused
                                                // clicks and pops.
                                                let mut written = 0;
                                                while written < temp_converted.len() {
                                                    let n =
                                                        prod.push_slice(&temp_converted[written..]);
                                                    written += n;
                                                    if written < temp_converted.len() {
                                                        // Buffer still full — yield briefly and retry.
                                                        thread::sleep(Duration::from_millis(1));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(SymphoniaError::DecodeError(err)) => {
                                        warn!("Audio decode error: {}", err);
                                    }
                                    Err(e) => {
                                        warn!("Unexpected audio error: {}", e);
                                    }
                                }
                            }
                        }
                        Err(SymphoniaError::IoError(ref e))
                            if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                        {
                            track.reached_eof = true;
                        }
                        Err(SymphoniaError::ResetRequired) => {
                            track.decoder.reset();
                        }
                        Err(e) => {
                            warn!("Format next_packet error: {}", e);
                            track.reached_eof = true;
                        }
                    }
                } else {
                    // Reached EOF: wait until ring buffer is completely drained.
                    let is_empty = ring_producer
                        .as_ref()
                        .map(|p| p.occupied_len() == 0)
                        .unwrap_or(true);

                    if is_empty {
                        status.set_playing(false);
                        status.set_ended(true);
                        info!("Track finished playing");
                    } else {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            } else {
                status.set_playing(false);
                thread::sleep(Duration::from_millis(20));
            }
        } else {
            // Not playing: block or wait briefly for next command
            match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => {
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
                Err(_) => {}
            }
        }
    }
}

// Global slot to transfer producer created in CPAL stream setup to worker thread
static WORKER_PRODUCER: std::sync::Mutex<Option<ringbuf::HeapProd<f32>>> =
    std::sync::Mutex::new(None);

fn setup_cpal_stream(
    device: &cpal::Device,
    status: AudioStatus,
) -> Result<(cpal::Stream, u32, usize, Arc<AtomicBool>), AudioEngineError> {
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
    let (producer, mut consumer) = rb.split();

    *WORKER_PRODUCER.lock().unwrap() = Some(producer);

    let flush_flag = Arc::new(AtomicBool::new(false));
    let flush_cb = flush_flag.clone();
    let status_cb = status.clone();

    let err_fn = |err| error!("CPAL output stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                audio_callback_f32(
                    data,
                    &mut consumer,
                    &flush_cb,
                    &status_cb,
                    sample_rate,
                    channels,
                );
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                audio_callback_i16(
                    data,
                    &mut consumer,
                    &flush_cb,
                    &status_cb,
                    sample_rate,
                    channels,
                );
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.into(),
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                audio_callback_u16(
                    data,
                    &mut consumer,
                    &flush_cb,
                    &status_cb,
                    sample_rate,
                    channels,
                );
            },
            err_fn,
            None,
        )?,
        _ => return Err(AudioEngineError::NoDevice),
    };

    stream.play()?;

    Ok((stream, sample_rate, channels, flush_flag))
}

fn audio_callback_f32(
    data: &mut [f32],
    consumer: &mut ringbuf::HeapCons<f32>,
    flush_flag: &AtomicBool,
    status: &AudioStatus,
    sample_rate: u32,
    channels: usize,
) {
    if flush_flag.swap(false, Ordering::SeqCst) {
        consumer.clear();
    }

    if !status.is_playing() {
        data.fill(0.0);
        return;
    }

    let read_count = consumer.pop_slice(data);
    if read_count < data.len() {
        data[read_count..].fill(0.0);
    }

    let vol = status.volume();
    if (vol - 1.0).abs() > 0.001 {
        for sample in &mut data[..read_count] {
            *sample *= vol;
        }
    }

    if read_count > 0 && channels > 0 && sample_rate > 0 {
        let frames_played = read_count / channels;
        let elapsed_ms = (frames_played as u64 * 1000) / (sample_rate as u64);
        status.set_position_ms(status.position_ms() + elapsed_ms);
    }
}

fn audio_callback_i16(
    data: &mut [i16],
    consumer: &mut ringbuf::HeapCons<f32>,
    flush_flag: &AtomicBool,
    status: &AudioStatus,
    sample_rate: u32,
    channels: usize,
) {
    if flush_flag.swap(false, Ordering::SeqCst) {
        consumer.clear();
    }

    if !status.is_playing() {
        data.fill(0);
        return;
    }

    // Bulk-read f32 samples into a scratch buffer, then convert in one pass —
    // much cheaper than calling try_pop() per sample in the realtime callback.
    let mut scratch = vec![0.0f32; data.len()];
    let read_count = consumer.pop_slice(&mut scratch);
    if read_count < data.len() {
        scratch[read_count..].fill(0.0);
    }

    let vol = status.volume();
    for (i, sample) in data.iter_mut().enumerate() {
        let clamped = (scratch[i] * vol).clamp(-1.0, 1.0);
        *sample = (clamped * i16::MAX as f32) as i16;
    }

    if read_count > 0 && channels > 0 && sample_rate > 0 {
        let frames_played = read_count / channels;
        let elapsed_ms = (frames_played as u64 * 1000) / (sample_rate as u64);
        status.set_position_ms(status.position_ms() + elapsed_ms);
    }
}

fn audio_callback_u16(
    data: &mut [u16],
    consumer: &mut ringbuf::HeapCons<f32>,
    flush_flag: &AtomicBool,
    status: &AudioStatus,
    sample_rate: u32,
    channels: usize,
) {
    if flush_flag.swap(false, Ordering::SeqCst) {
        consumer.clear();
    }

    if !status.is_playing() {
        data.fill(u16::MAX / 2);
        return;
    }

    // Bulk-read f32 samples into a scratch buffer, then convert in one pass.
    let mut scratch = vec![0.0f32; data.len()];
    let read_count = consumer.pop_slice(&mut scratch);
    if read_count < data.len() {
        scratch[read_count..].fill(0.0);
    }

    let vol = status.volume();
    for (i, sample) in data.iter_mut().enumerate() {
        let clamped = (scratch[i] * vol).clamp(-1.0, 1.0);
        *sample = (((clamped + 1.0) * 0.5) * u16::MAX as f32) as u16;
    }

    if read_count > 0 && channels > 0 && sample_rate > 0 {
        let frames_played = read_count / channels;
        let elapsed_ms = (frames_played as u64 * 1000) / (sample_rate as u64);
        status.set_position_ms(status.position_ms() + elapsed_ms);
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
                    *loaded_track = Some(track);
                    status.set_duration_ms(duration_ms);
                    status.set_position_ms(0);
                    status.set_loaded(true);
                    status.set_ended(false);

                    // Signal the CPAL callback to clear its consumer-side buffer.
                    // This is atomic — the callback will drain stale samples from the
                    // previous track before the decode loop feeds in new ones.
                    if let Some(ref f) = flush_flag {
                        f.store(true, Ordering::SeqCst);
                    }

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
                let _ = track.format.seek(
                    SeekMode::Coarse,
                    SeekTo::Time {
                        time: Time::from(0.0),
                        track_id: Some(track.track_id),
                    },
                );
                track.decoder.reset();
                track.converter.reset();
                track.reached_eof = false;
            }
            if let Some(ref f) = flush_flag {
                f.store(true, Ordering::SeqCst);
            }
        }
        AudioCommand::Seek(duration) => {
            if let Some(track) = loaded_track.as_mut() {
                let seek_secs = duration.as_secs_f64();
                match track.format.seek(
                    SeekMode::Coarse,
                    SeekTo::Time {
                        time: Time::from(seek_secs),
                        track_id: Some(track.track_id),
                    },
                ) {
                    Ok(_) => {
                        track.decoder.reset();
                        track.converter.reset();
                        track.reached_eof = false;
                        if let Some(ref f) = flush_flag {
                            f.store(true, Ordering::SeqCst);
                        }
                        status.set_position_ms(duration.as_millis() as u64);
                        info!("Seeked to {:?}", duration);
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
    path: &PathBuf,
    dst_sample_rate: u32,
    dst_channels: usize,
) -> Result<(LoadedTrack, u64), SymphoniaError> {
    let file = File::open(path).map_err(|e| SymphoniaError::IoError(e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(SymphoniaError::Unsupported("No audio track found in file"))?;

    let track_id = track.id;
    let src_sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let src_channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

    let mut duration_ms = 0u64;
    if let Some(n_frames) = track.codec_params.n_frames {
        if src_sample_rate > 0 {
            duration_ms = (n_frames * 1000) / (src_sample_rate as u64);
        }
    } else if let Some(tb) = track.codec_params.time_base {
        if let Some(n_frames) = track.codec_params.n_frames {
            let time = tb.calc_time(n_frames);
            duration_ms = (time.seconds * 1000) + ((time.frac * 1000.0) as u64);
        }
    }

    let decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let converter =
        AudioConverter::new(src_channels, src_sample_rate, dst_channels, dst_sample_rate);

    let loaded = LoadedTrack {
        format,
        decoder,
        track_id,
        sample_buf: None,
        converter,
        reached_eof: false,
    };

    Ok((loaded, duration_ms))
}
