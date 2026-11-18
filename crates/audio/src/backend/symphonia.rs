use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use symphonia::core::{
    audio::{SampleBuffer, SignalSpec},
    codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};
use tracing::{info, warn};

use crate::backend::RingSink;
use crate::resample::{ResamplingConfig, ResamplingFilter, ResamplingQuality};

const RESAMPLE_CHUNK_SIZE: usize = 1024;

fn build_rubato_resampler(
    src_rate: u32,
    dst_rate: u32,
    channels: usize,
    cfg: &ResamplingConfig,
) -> Option<SincFixedIn<f32>> {
    if src_rate == dst_rate || src_rate == 0 || dst_rate == 0 || channels == 0 {
        return None;
    }

    let sinc_len = match cfg.quality {
        ResamplingQuality::Fast => 16,
        ResamplingQuality::Standard => 32,
        ResamplingQuality::High => 64,
        ResamplingQuality::Ultra => 128,
    };

    let window = match cfg.filter {
        ResamplingFilter::Kaiser => WindowFunction::BlackmanHarris2,
        ResamplingFilter::BlackmanNuttall => WindowFunction::Blackman2,
        ResamplingFilter::Cubic => WindowFunction::Hann,
        ResamplingFilter::Soxr => WindowFunction::BlackmanHarris2,
    };

    let interpolation = if cfg.filter == ResamplingFilter::Cubic {
        SincInterpolationType::Cubic
    } else if cfg.linear_interp {
        SincInterpolationType::Linear
    } else {
        SincInterpolationType::Cubic
    };

    let params = SincInterpolationParameters {
        sinc_len,
        f_cutoff: 0.95,
        oversampling_factor: 128,
        interpolation,
        window,
    };

    match SincFixedIn::<f32>::new(
        dst_rate as f64 / src_rate as f64,
        2.0,
        params,
        RESAMPLE_CHUNK_SIZE,
        channels,
    ) {
        Ok(r) => Some(r),
        Err(e) => {
            warn!("Failed to create rubato resampler: {}", e);
            None
        }
    }
}

pub(crate) struct LoadedTrack {
    path: PathBuf,
    duration_ms: u64,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    src_rate: u32,
    src_channels: usize,
    dst_channels: usize,
    resampler: Option<SincFixedIn<f32>>,
    planar_in: Vec<Vec<f32>>,
    planar_read_pos: usize,
    resample_out: Vec<Vec<f32>>,
    interleaved_scratch: Vec<f32>,
    sample_buf: Option<SampleBuffer<f32>>,
    sample_buf_spec: Option<SignalSpec>,
    reached_eof: bool,
}

impl LoadedTrack {
    pub(crate) fn load(
        path: &Path,
        dst_sample_rate: u32,
        dst_channels: usize,
        resampling_cfg: &ResamplingConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path)?;
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
            .ok_or("No supported audio track found")?;

        let track_id = track.id;
        let params = track.codec_params.clone();

        let src_rate = params.sample_rate.unwrap_or(44100);
        let src_channels = params.channels.map(|c| c.count()).unwrap_or(2);

        let duration_ms = params
            .n_frames
            .and_then(|n_frames| {
                if let Some(tb) = params.time_base {
                    let t = tb.calc_time(n_frames);
                    Some(t.seconds * 1000 + (t.frac * 1000.0).round() as u64)
                } else if src_rate > 0 {
                    Some((n_frames * 1000) / src_rate as u64)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions::default())?;

        let resampler =
            build_rubato_resampler(src_rate, dst_sample_rate, dst_channels, resampling_cfg);
        let resample_out = resampler
            .as_ref()
            .map(|r| r.output_buffer_allocate(true))
            .unwrap_or_default();
        let planar_in = vec![Vec::with_capacity(RESAMPLE_CHUNK_SIZE * 2); dst_channels];

        Ok(Self {
            path: path.to_path_buf(),
            duration_ms,
            format,
            decoder,
            track_id,
            src_rate,
            src_channels,
            dst_channels,
            resampler,
            planar_in,
            planar_read_pos: 0,
            resample_out,
            interleaved_scratch: Vec::new(),
            sample_buf: None,
            sample_buf_spec: None,
            reached_eof: false,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    pub(crate) fn reached_eof(&self) -> bool {
        self.reached_eof
    }

    pub(crate) fn seek_to_start(&mut self) {
        let _ = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::from(0.0),
                track_id: Some(self.track_id),
            },
        );
        self.decoder.reset();
        for ch in &mut self.planar_in {
            ch.clear();
        }
        self.planar_read_pos = 0;
        if let Some(ref mut r) = self.resampler {
            r.reset();
        }
        self.reached_eof = false;
    }

    pub(crate) fn seek(
        &mut self,
        target: Duration,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let secs = target.as_secs();
        let frac = (target.subsec_nanos() as f64) / 1_000_000_000.0;
        self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::new(secs, frac),
                track_id: Some(self.track_id),
            },
        )?;
        self.decoder.reset();
        for ch in &mut self.planar_in {
            ch.clear();
        }
        self.planar_read_pos = 0;
        if let Some(ref mut r) = self.resampler {
            r.reset();
        }
        self.reached_eof = false;
        info!("Seeked to {:?}", target);
        Ok(())
    }

    pub(crate) fn reconfigure_resampler(
        &mut self,
        dst_sample_rate: u32,
        dst_channels: usize,
        new_cfg: &ResamplingConfig,
    ) {
        self.dst_channels = dst_channels;
        self.planar_in = vec![Vec::with_capacity(RESAMPLE_CHUNK_SIZE * 2); dst_channels];
        self.planar_read_pos = 0;
        self.resampler =
            build_rubato_resampler(self.src_rate, dst_sample_rate, dst_channels, new_cfg);
        self.resample_out = self
            .resampler
            .as_ref()
            .map(|r| r.output_buffer_allocate(true))
            .unwrap_or_default();
    }

    fn process_sample_buf(&mut self, frames: usize, src_ch: usize, sink: &mut RingSink<'_>) {
        let Some(buf) = self.sample_buf.as_ref() else {
            return;
        };
        let interleaved = buf.samples();

        if self.resampler.is_none() {
            // Fast path: no sample rate conversion needed
            if src_ch == self.dst_channels {
                sink.push_interleaved(interleaved);
            } else {
                self.interleaved_scratch.clear();
                self.interleaved_scratch.reserve(frames * self.dst_channels);
                for frame in interleaved.chunks_exact(src_ch) {
                    for ch_idx in 0..self.dst_channels {
                        let sample = if src_ch == 1 {
                            frame[0]
                        } else if ch_idx < src_ch {
                            frame[ch_idx]
                        } else {
                            0.0
                        };
                        self.interleaved_scratch.push(sample);
                    }
                }
                sink.push_interleaved(&self.interleaved_scratch);
            }
            return;
        }

        // Accumulate into planar channel buffers for rubato
        for frame in interleaved.chunks_exact(src_ch) {
            for (ch_idx, ch_buf) in self.planar_in.iter_mut().enumerate() {
                let sample = if src_ch == 1 {
                    frame[0]
                } else if ch_idx < src_ch {
                    frame[ch_idx]
                } else {
                    0.0
                };
                ch_buf.push(sample);
            }
        }

        self.drain_resampler(sink);
    }

    fn drain_resampler(&mut self, sink: &mut RingSink<'_>) {
        let Some(ref mut resampler) = self.resampler else {
            return;
        };

        loop {
            let needed = resampler.input_frames_next();
            let available = self
                .planar_in
                .first()
                .map(|c| c.len().saturating_sub(self.planar_read_pos))
                .unwrap_or(0);
            if available < needed {
                break;
            }
            if sink.is_flushing() {
                break;
            }

            let start = self.planar_read_pos;
            let end = start + needed;
            self.planar_read_pos = end;

            let mut in_slices: [&[f32]; 8] = [&[]; 8];
            let res = if self.dst_channels <= 8 {
                for (i, ch) in self.planar_in.iter().enumerate() {
                    in_slices[i] = &ch[start..end];
                }
                resampler.process_into_buffer(
                    &in_slices[..self.dst_channels],
                    &mut self.resample_out,
                    None,
                )
            } else {
                let slices: Vec<&[f32]> = self.planar_in.iter().map(|c| &c[start..end]).collect();
                resampler.process_into_buffer(&slices, &mut self.resample_out, None)
            };

            match res {
                Ok((_, out_frames)) => {
                    if out_frames > 0 {
                        self.interleaved_scratch.clear();
                        self.interleaved_scratch
                            .reserve(out_frames * self.dst_channels);
                        for i in 0..out_frames {
                            for ch in &self.resample_out {
                                self.interleaved_scratch.push(ch[i]);
                            }
                        }
                        if !sink.push_interleaved(&self.interleaved_scratch) {
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Rubato resampling error: {}", e);
                    break;
                }
            }
        }

        // Compact consumed samples once at the end of the packet drain
        if self.planar_read_pos > 0 {
            let consumed = self.planar_read_pos;
            for ch in &mut self.planar_in {
                ch.drain(..consumed);
            }
            self.planar_read_pos = 0;
        }
    }

    fn flush_resampler(&mut self, sink: &mut RingSink<'_>) {
        let Some(ref mut resampler) = self.resampler else {
            return;
        };
        let remaining_len = self
            .planar_in
            .first()
            .map(|c| c.len().saturating_sub(self.planar_read_pos))
            .unwrap_or(0);
        if remaining_len == 0 {
            return;
        }
        let start = self.planar_read_pos;
        let slices: Vec<&[f32]> = self.planar_in.iter().map(|c| &c[start..]).collect();
        let result = resampler.process_partial(Some(&slices), None);
        for ch in &mut self.planar_in {
            ch.clear();
        }
        self.planar_read_pos = 0;
        match result {
            Ok(out_planar) => {
                let out_frames = out_planar.first().map(|c| c.len()).unwrap_or(0);
                if out_frames > 0 {
                    self.interleaved_scratch.clear();
                    self.interleaved_scratch
                        .reserve(out_frames * self.dst_channels);
                    for i in 0..out_frames {
                        for ch in &out_planar {
                            self.interleaved_scratch.push(ch[i]);
                        }
                    }
                    sink.push_interleaved(&self.interleaved_scratch);
                }
            }
            Err(e) => {
                warn!("Rubato flush error: {}", e);
            }
        }
    }

    pub(crate) fn decode_step(&mut self, sink: &mut RingSink<'_>) -> bool {
        match self.format.next_packet() {
            Ok(packet) => {
                if packet.track_id() == self.track_id {
                    match self.decoder.decode(&packet) {
                        Ok(decoded) => {
                            let spec: SignalSpec = *decoded.spec();
                            let frames = decoded.frames();
                            if frames > 0 {
                                let src_ch = spec.channels.count().max(1);
                                self.src_channels = src_ch;

                                let need_new_buf = match self.sample_buf.as_ref() {
                                    Some(buf) => {
                                        buf.capacity() < frames * src_ch
                                            || self.sample_buf_spec != Some(spec)
                                    }
                                    None => true,
                                };
                                if need_new_buf {
                                    self.sample_buf = Some(SampleBuffer::<f32>::new(
                                        decoded.capacity() as u64,
                                        spec,
                                    ));
                                    self.sample_buf_spec = Some(spec);
                                }

                                let buf = self.sample_buf.as_mut().expect("sample_buf initialized");
                                buf.copy_interleaved_ref(decoded);
                                self.process_sample_buf(frames, src_ch, sink);
                            }
                        }
                        Err(SymphoniaError::DecodeError(err)) => {
                            warn!("Symphonia decode error (skipping packet): {}", err);
                        }
                        Err(e) => {
                            warn!("Symphonia fatal decode error: {}", e);
                        }
                    }
                }
                false
            }
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                self.reached_eof = true;
                self.flush_resampler(sink);
                true
            }
            Err(SymphoniaError::ResetRequired) => {
                self.decoder.reset();
                false
            }
            Err(e) => {
                warn!("Symphonia packet read error: {}", e);
                self.reached_eof = true;
                self.flush_resampler(sink);
                true
            }
        }
    }
}
