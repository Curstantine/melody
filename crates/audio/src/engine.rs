use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use thiserror::Error;
use tracing::{error, info, warn};

use crate::backend::{LoadedTrack, RingSink};
use crate::{commands::AudioCommand, status::AudioStatus};

/// ~1.5 seconds of stereo f32 at 48 kHz: 48_000 * 2 * 1.5 = 144_000 samples.
/// Rounded up to the next power of two for ringbuf efficiency (2^18).
const RING_BUFFER_CAPACITY: usize = 262_144;

/// Extra time to wait after the ring buffer empties at end-of-track when the
/// device has not reported its output latency yet.
const FALLBACK_OUTPUT_LATENCY: Duration = Duration::from_millis(50);

#[derive(Error, Debug)]
enum AudioEngineError {
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
    fn send(&self, cmd: AudioCommand) {
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

    pub fn set_resampling(&self, config: crate::resample::ResamplingConfig) {
        self.send(AudioCommand::SetResampling(config));
    }

    /// Switches the active audio output device (`None` = system default).
    pub fn set_output_device(&self, device: Option<String>) {
        self.send(AudioCommand::SetOutputDevice(device));
    }

    pub fn status(&self) -> &AudioStatus {
        &self.status
    }
}

/// Enumerates available audio output device names on the default CPAL host.
pub fn list_output_devices() -> Vec<String> {
    let host = cpal::default_host();
    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    let mut names: Vec<String> = devices
        .filter_map(|dev| dev.description().ok().map(|d| d.name().to_string()))
        .filter(|name| !name.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrackBoundary {
    switch_at_sample: u64,
    next_duration_ms: u64,
}

struct AudioStreamContext {
    _active_stream: Option<cpal::Stream>,
    output_latency_us: Option<Arc<AtomicU64>>,
    device_sample_rate: u32,
    device_channels: usize,
    ring_producer: Option<ringbuf::HeapProd<f32>>,
    boundary_producer: Option<ringbuf::HeapProd<TrackBoundary>>,
}

fn resolve_output_device(host: &cpal::Host, device_name: Option<&str>) -> Option<cpal::Device> {
    if let Some(target_name) = device_name {
        if let Ok(mut devices) = host.output_devices() {
            if let Some(found) = devices.find(|dev| {
                dev.description()
                    .ok()
                    .is_some_and(|d| d.name() == target_name)
            }) {
                return Some(found);
            }
        }
        warn!(
            "Configured audio output device '{}' not found, falling back to system default",
            target_name
        );
    }
    host.default_output_device()
}

fn init_cpal_output_stream(
    device_name: Option<&str>,
    status: &AudioStatus,
    flush_flag: &Arc<AtomicBool>,
) -> AudioStreamContext {
    let host = cpal::default_host();
    let device = resolve_output_device(&host, device_name);

    let (stream, device_sample_rate, device_channels, ring_producer, boundary_producer) =
        match device {
            Some(dev) => match setup_cpal_stream(&dev, status.clone(), flush_flag.clone()) {
                Ok((st, sr, ch, latency_us, prod, bound_prod)) => {
                    let active_name = dev
                        .description()
                        .ok()
                        .map(|d| d.name().to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    info!(
                        "Audio output stream initialized on '{}' ({} Hz, {} ch)",
                        active_name, sr, ch
                    );
                    (Some((st, latency_us)), sr, ch, Some(prod), Some(bound_prod))
                }
                Err(e) => {
                    warn!(
                        "Failed to setup CPAL stream: {}. Audio playback will be muted.",
                        e
                    );
                    (None, 44100, 2, None, None)
                }
            },
            None => {
                warn!("No default audio output device found. Audio playback will be muted.");
                (None, 44100, 2, None, None)
            }
        };

    let output_latency_us = stream.as_ref().map(|(_, l)| l.clone());
    let active_stream = stream.map(|(s, _)| s);

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
    device_name: Option<&str>,
    status: &AudioStatus,
    flush_flag: &Arc<AtomicBool>,
) -> &'a mut AudioStreamContext {
    ctx.get_or_insert_with(|| init_cpal_output_stream(device_name, status, flush_flag))
}

#[allow(clippy::too_many_arguments)]
fn switch_output_device(
    stream_ctx: &mut Option<AudioStreamContext>,
    selected_output_device: &mut Option<String>,
    new_device: Option<String>,
    status: &AudioStatus,
    flush_flag: &Arc<AtomicBool>,
    loaded_track: &mut Option<LoadedTrack>,
    next_track: &mut Option<LoadedTrack>,
    written_samples: &mut u64,
    resampling_cfg: &crate::resample::ResamplingConfig,
) {
    if *selected_output_device == new_device {
        return;
    }
    *selected_output_device = new_device;
    if stream_ctx.is_some() {
        *stream_ctx = None;
        flush_audio_buffer(flush_flag);
        *written_samples = 0;
        let ctx = ensure_stream(
            stream_ctx,
            selected_output_device.as_deref(),
            status,
            flush_flag,
        );
        let sr = ctx.device_sample_rate;
        let ch = ctx.device_channels;
        if let Some(track) = loaded_track.as_mut() {
            track.reconfigure_resampler(sr, ch, resampling_cfg);
            let pos = Duration::from_millis(status.position_ms());
            if let Err(e) = track.seek(pos) {
                warn!("Failed to re-seek track after device switch: {}", e);
            }
        }
        if let Some(track) = next_track.as_mut() {
            track.reconfigure_resampler(sr, ch, resampling_cfg);
        }
    }
}

fn run_audio_worker(
    cmd_rx: Receiver<AudioCommand>,
    status: AudioStatus,
    flush_flag: Arc<AtomicBool>,
) {
    info!("Starting audio worker thread");

    let mut stream_ctx: Option<AudioStreamContext> = None;
    let mut selected_output_device: Option<String> = None;
    let mut loaded_track: Option<LoadedTrack> = None;
    let mut next_track: Option<LoadedTrack> = None;
    let mut written_samples: u64 = 0;
    let mut eof_deadline: Option<Instant> = None;
    let mut resampling_cfg = crate::resample::ResamplingConfig::default();

    loop {
        // 1. Process all pending commands
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let AudioCommand::SetOutputDevice(new_device) = cmd {
                switch_output_device(
                    &mut stream_ctx,
                    &mut selected_output_device,
                    new_device,
                    &status,
                    &flush_flag,
                    &mut loaded_track,
                    &mut next_track,
                    &mut written_samples,
                    &resampling_cfg,
                );
                continue;
            }
            let is_playback_cmd = matches!(
                cmd,
                AudioCommand::Load { .. }
                    | AudioCommand::Play
                    | AudioCommand::TogglePlayPause
                    | AudioCommand::Seek(_)
                    | AudioCommand::SetNextTrack(_)
                    | AudioCommand::SetResampling(_)
            );
            let (sr, ch, prod) = if is_playback_cmd || stream_ctx.is_some() {
                let ctx = ensure_stream(
                    &mut stream_ctx,
                    selected_output_device.as_deref(),
                    &status,
                    &flush_flag,
                );
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
                &mut resampling_cfg,
            );
        }

        // 2. Decode audio if playing and loaded
        if status.is_playing() {
            let ctx = ensure_stream(
                &mut stream_ctx,
                selected_output_device.as_deref(),
                &status,
                &flush_flag,
            );
            let output_latency_us = &ctx.output_latency_us;

            let mut transition_to = None;
            if let Some(track) = loaded_track.as_mut() {
                if !track.reached_eof() {
                    eof_deadline = None;

                    let mut sink = RingSink {
                        ring_producer: &mut ctx.ring_producer,
                        flush_flag: &flush_flag,
                        written_samples: &mut written_samples,
                    };
                    if track.decode_step(&mut sink) {
                        if let Some(next) = next_track.take() {
                            transition_to = Some(next);
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
                    next.path(),
                    written_samples
                );
                if let Some(ref mut bp) = ctx.boundary_producer {
                    let _ = bp.try_push(TrackBoundary {
                        switch_at_sample: written_samples,
                        next_duration_ms: next.duration_ms(),
                    });
                }
                loaded_track = Some(next);
                eof_deadline = None;
            }
        } else {
            eof_deadline = None;

            // Not playing: block until next command or clean channel disconnection
            match cmd_rx.recv() {
                Ok(AudioCommand::SetOutputDevice(new_device)) => {
                    switch_output_device(
                        &mut stream_ctx,
                        &mut selected_output_device,
                        new_device,
                        &status,
                        &flush_flag,
                        &mut loaded_track,
                        &mut next_track,
                        &mut written_samples,
                        &resampling_cfg,
                    );
                }
                Ok(cmd) => {
                    let is_playback_cmd = matches!(
                        cmd,
                        AudioCommand::Load { .. }
                            | AudioCommand::Play
                            | AudioCommand::TogglePlayPause
                            | AudioCommand::Seek(_)
                            | AudioCommand::SetNextTrack(_)
                            | AudioCommand::SetResampling(_)
                    );
                    let (sr, ch, prod) = if is_playback_cmd || stream_ctx.is_some() {
                        let ctx = ensure_stream(
                            &mut stream_ctx,
                            selected_output_device.as_deref(),
                            &status,
                            &flush_flag,
                        );
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
                        &mut resampling_cfg,
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

pub(crate) fn push_to_ring_buffer(
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

// (producers transferred directly via setup_cpal_stream return tuple)

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

type CpalStreamSetup = (
    cpal::Stream,
    u32,
    usize,
    Arc<AtomicU64>,
    ringbuf::HeapProd<f32>,
    ringbuf::HeapProd<TrackBoundary>,
);

/// Shared, per-stream state handed to the realtime output callbacks.
struct CallbackState<'a> {
    consumer: &'a mut ringbuf::HeapCons<f32>,
    boundary_consumer: &'a mut ringbuf::HeapCons<TrackBoundary>,
    current_boundary: &'a mut Option<TrackBoundary>,
    consumed_samples: &'a mut u64,
    flush_flag: &'a AtomicBool,
    status: &'a AudioStatus,
    sample_rate: u32,
    channels: usize,
    pos_rem: &'a mut u64,
}

fn pop_samples_with_boundary(data: &mut [f32], state: &mut CallbackState) -> usize {
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

    Ok((
        stream,
        sample_rate,
        channels,
        latency_us,
        producer,
        boundary_producer,
    ))
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
    resampling_cfg: &mut crate::resample::ResamplingConfig,
) {
    match cmd {
        AudioCommand::Load { path, autoplay } => {
            info!("Loading audio file: {:?}", path);
            *next_track = None;
            match LoadedTrack::load(&path, device_sample_rate, device_channels, resampling_cfg) {
                Ok(track) => {
                    // Flush any stale audio from the previous track and wait for the consumer
                    // callback to acknowledge it before loading the new track and decoding samples.
                    flush_audio_buffer(flush_flag);
                    *written_samples = 0;

                    let duration_ms = track.duration_ms();
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
                if next_track.as_ref().map(|t| t.path()) == Some(path.as_path()) {
                    return;
                }
                info!("Preloading next track for gapless audio: {:?}", path);
                match LoadedTrack::load(&path, device_sample_rate, device_channels, resampling_cfg)
                {
                    Ok(track) => {
                        info!("Preloaded next track successfully: {:?}", track.path());
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
                track.seek_to_start();
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

                match track.seek(target_dur) {
                    Ok(_) => {
                        flush_audio_buffer(flush_flag);
                        *written_samples = 0;

                        if let Some(ref mut next) = next_track {
                            next.seek_to_start();
                        }

                        status.set_position_ms(target_dur.as_millis() as u64);
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
        AudioCommand::SetResampling(new_cfg) => {
            info!("Updating audio resampling configuration: {:?}", new_cfg);
            *resampling_cfg = new_cfg.clone();
            if let Some(track) = loaded_track.as_mut() {
                track.reconfigure_resampler(device_sample_rate, device_channels, &new_cfg);
            }
            if let Some(track) = next_track.as_mut() {
                track.reconfigure_resampler(device_sample_rate, device_channels, &new_cfg);
            }
        }
        AudioCommand::SetOutputDevice(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_flush_synchronization() {
        use ringbuf::traits::{Consumer, Producer, Split};
        use ringbuf::HeapRb;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let rb = HeapRb::<f32>::new(1024);
        let (mut producer, mut consumer) = rb.split();
        let flush_flag = Arc::new(AtomicBool::new(false));

        // Simulate old track samples in the buffer
        let old_samples = vec![1.0f32; 256];
        producer.push_slice(&old_samples);

        // Simulate worker requesting a flush
        let flush_flag_worker = flush_flag.clone();
        flush_flag_worker.store(true, Ordering::SeqCst);

        // Simulate callback clearing when it sees the flush flag
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(5));
            if flush_flag.swap(false, Ordering::SeqCst) {
                consumer.clear();
            }
            consumer
        });

        // Worker waits for flush acknowledgment
        let start = std::time::Instant::now();
        while flush_flag_worker.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
            if start.elapsed() > Duration::from_millis(100) {
                panic!("Flush timed out");
            }
        }

        // Now worker pushes the new track's samples starting at 0:00
        let new_samples = vec![0.5f32; 128];
        producer.push_slice(&new_samples);

        let mut consumer = handle.join().unwrap();
        // Consumer callback reads the new track's samples from the very beginning
        let mut read_buf = vec![0.0f32; 128];
        let read = consumer.pop_slice(&mut read_buf);
        assert_eq!(read, 128);
        assert_eq!(read_buf, new_samples);
    }

    #[test]
    fn test_track_decoding_to_eof() {
        let path = std::path::Path::new(
            "/home/curstantine/Music/Library/矢島舞依/The Un-Dead/01 Introduction.flac",
        );
        if !path.exists() {
            return;
        }
        let engine = AudioEngine::new();
        let controller = engine.controller();
        controller.load(path.to_path_buf(), true);

        std::thread::sleep(std::time::Duration::from_millis(200));
        let dur = controller.status().duration_ms();
        assert!(dur > 0, "Duration must be greater than 0");

        // Seek to 1.5 seconds before end
        controller.seek(std::time::Duration::from_millis(dur.saturating_sub(1500)));

        let start = std::time::Instant::now();
        let mut saw_ended = false;
        while start.elapsed() < std::time::Duration::from_secs(5) {
            let pos = controller.status().position_ms();
            assert!(
                pos <= dur,
                "Position ({}) must never exceed duration ({})",
                pos,
                dur
            );
            if controller.status().is_ended() {
                saw_ended = true;
                assert_eq!(pos, dur, "When ended, position must equal duration");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(saw_ended, "Track must reach ended state");

        // Test that a subsequent load command is immediately accepted and plays
        controller.load(path.to_path_buf(), true);
        let start2 = std::time::Instant::now();
        let mut second_track_played = false;
        while start2.elapsed() < std::time::Duration::from_secs(3) {
            if controller.status().is_playing() && controller.status().position_ms() > 100 {
                second_track_played = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(
            second_track_played,
            "Subsequent track load must play without hang"
        );
    }

    #[test]
    fn test_gapless_status_and_boundary_transition() {
        use ringbuf::traits::{Producer, Split};
        use ringbuf::HeapRb;
        use std::sync::atomic::AtomicBool;

        let status = AudioStatus::new();
        status.set_duration_ms(100_000);
        status.set_playing(true);

        assert!(!status.gapless_transition());
        status.set_gapless_transition(true);
        assert!(status.gapless_transition());
        assert!(status.take_gapless_transition());
        assert!(!status.gapless_transition());

        // Test sample-accurate boundary transition in pop_samples_with_boundary
        let audio_rb = HeapRb::<f32>::new(1024);
        let (mut audio_prod, mut audio_cons) = audio_rb.split();

        let boundary_rb = HeapRb::<TrackBoundary>::new(8);
        let (mut boundary_prod, mut boundary_cons) = boundary_rb.split();

        // Track 1 has 40 samples in the buffer
        let track1_samples = vec![1.0f32; 40];
        audio_prod.push_slice(&track1_samples);

        // Track 2 has 60 samples in the buffer
        let track2_samples = vec![2.0f32; 60];
        audio_prod.push_slice(&track2_samples);

        // Set boundary at sample 40, switching to Track 2 with duration 200_000 ms
        let _ = boundary_prod.try_push(TrackBoundary {
            switch_at_sample: 40,
            next_duration_ms: 200_000,
        });

        let flush_flag = AtomicBool::new(false);
        let mut current_boundary = None;
        let mut consumed_samples: u64 = 0;
        let mut pos_rem: u64 = 0;

        let mut state = CallbackState {
            consumer: &mut audio_cons,
            boundary_consumer: &mut boundary_cons,
            current_boundary: &mut current_boundary,
            consumed_samples: &mut consumed_samples,
            flush_flag: &flush_flag,
            status: &status,
            sample_rate: 44100,
            channels: 2,
            pos_rem: &mut pos_rem,
        };

        // Callback requests 100 samples (covering 40 samples of track 1 and 60 samples of track 2)
        let mut out = vec![0.0f32; 100];
        let read = pop_samples_with_boundary(&mut out, &mut state);

        assert_eq!(read, 100);
        assert_eq!(&out[..40], &track1_samples[..]);
        assert_eq!(&out[40..100], &track2_samples[..]);

        // Check that gapless transition was triggered
        assert!(status.take_gapless_transition());
        assert_eq!(status.duration_ms(), 200_000);
        assert_eq!(state.current_boundary, &None);
        assert_eq!(*state.consumed_samples, 100);
    }

    #[test]
    fn test_list_output_devices_does_not_panic() {
        let devices = list_output_devices();
        assert!(devices.iter().all(|d| !d.is_empty()));
    }
}
