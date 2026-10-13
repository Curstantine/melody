use rubato::audioadapter_buffers::direct::{SequentialSliceOfSlices, SequentialSliceOfVecs};
use rubato::{Fft, FixedSync, Resampler, WindowFunction};
use tracing::warn;

/// ITU-R BS.775 downmix coefficients for center and surround channels.
const CENTER_COEFF: f32 = 0.707; // -3 dB  (1 / sqrt(2))
const SURROUND_COEFF: f32 = 0.707; // -3 dB

/// Converts a single source frame (arbitrary channel count) into a
/// destination frame with `dst_channels` channels, following ITU-R BS.775.
///
/// Supported layouts:
/// - 1 (Mono)        → N: duplicate to all channels
/// - 2 (Stereo)      → 2: identity; → 1: average
/// - 3 (L C R)       → 2: L += C*k, R += C*k
/// - 4 (L R Ls Rs)   → 2: L += Ls*k, R += Rs*k
/// - 6 (5.1)         → 2: ITU-R BS.775 standard downmix
/// - 8 (7.1)         → 2: standard downmix
/// - Anything else   → copy first dst_channels, zero the rest
fn downmix_frame(src: &[f32], dst: &mut [f32]) {
    let src_ch = src.len();
    let dst_ch = dst.len();

    match (src_ch, dst_ch) {
        // ── Mono → anything: duplicate ───────────────────────────────────────
        (1, _) => dst.fill(src[0]),

        // ── Stereo → stereo (identity) ───────────────────────────────────────
        (2, 2) => {
            dst[0] = src[0];
            dst[1] = src[1];
        }

        // ── Stereo → mono ────────────────────────────────────────────────────
        (2, 1) => dst[0] = (src[0] + src[1]) * 0.5,

        // ── 3.0 (L C R) → stereo ─────────────────────────────────────────────
        (3, 2) => {
            let c = src[1] * CENTER_COEFF;
            dst[0] = (src[0] + c).clamp(-1.0, 1.0);
            dst[1] = (src[2] + c).clamp(-1.0, 1.0);
        }

        // ── 4.0 (L R Ls Rs) → stereo ─────────────────────────────────────────
        (4, 2) => {
            dst[0] = (src[0] + src[2] * SURROUND_COEFF).clamp(-1.0, 1.0);
            dst[1] = (src[1] + src[3] * SURROUND_COEFF).clamp(-1.0, 1.0);
        }

        // ── 5.1 (L R C LFE Ls Rs) → stereo ──────────────────────────────────
        (6, 2) => {
            let c = src[2] * CENTER_COEFF;
            // LFE (src[3]) is omitted in standard stereo downmix
            dst[0] = (src[0] + c + src[4] * SURROUND_COEFF).clamp(-1.0, 1.0);
            dst[1] = (src[1] + c + src[5] * SURROUND_COEFF).clamp(-1.0, 1.0);
        }

        // ── 7.1 (L R C LFE Ls Rs Lss Rss) → stereo ──────────────────────────
        (8, 2) => {
            let c = src[2] * CENTER_COEFF;
            dst[0] = (src[0] + c + (src[4] + src[6]) * SURROUND_COEFF).clamp(-1.0, 1.0);
            dst[1] = (src[1] + c + (src[5] + src[7]) * SURROUND_COEFF).clamp(-1.0, 1.0);
        }

        // ── src_ch == dst_ch (no channel conversion needed) ──────────────────
        _ if src_ch == dst_ch => dst.copy_from_slice(src),

        // ── Fallback: copy as many channels as possible, zero the rest ────────
        _ => {
            let copy = src_ch.min(dst_ch);
            dst[..copy].copy_from_slice(&src[..copy]);
            dst[copy..].fill(0.0);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AudioConverter
// ─────────────────────────────────────────────────────────────────────────────

pub struct AudioConverter {
    src_channels: usize,
    dst_channels: usize,
    src_sample_rate: u32,
    dst_sample_rate: u32,
    /// rubato FFT resampler operating on `src_channels` separate channel
    /// planes. `None` when src and dst sample rates are identical.
    resampler: Option<Fft<f32>>,
    /// Planar channel buffers fed into rubato.
    input_planes: Vec<Vec<f32>>,
    /// Planar channel buffers produced by rubato.
    output_planes: Vec<Vec<f32>>,
    /// Pre-allocated scratch buffers to avoid allocations in audio processing loops
    dst_frame_buf: Vec<f32>,
    src_frame_buf: Vec<f32>,
}

/// Size of rubato's processing block in source frames.
/// 1024 frames ≈ 23 ms at 44.1 kHz — small enough for low latency, large
/// enough for the FFT resampler to be efficient.
const RESAMPLE_CHUNK: usize = 1024;

impl AudioConverter {
    pub fn new(
        src_channels: usize,
        src_sample_rate: u32,
        dst_channels: usize,
        dst_sample_rate: u32,
    ) -> Self {
        let src_channels = src_channels.max(1);
        let dst_channels = dst_channels.max(1);

        // Build a rubato FFT resampler when sample rates differ.
        let resampler = if src_sample_rate != dst_sample_rate {
            match Fft::<f32>::new_custom(
                src_sample_rate as usize,
                dst_sample_rate as usize,
                RESAMPLE_CHUNK,
                2, // sub-chunks — 2 gives a good quality/latency trade-off
                src_channels,
                WindowFunction::BlackmanHarris2,
                FixedSync::Input,
            ) {
                Ok(r) => Some(r),
                Err(e) => {
                    warn!(
                        "Failed to create sinc resampler ({} → {} Hz): {}. \
                         Falling back to no resampling.",
                        src_sample_rate, dst_sample_rate, e
                    );
                    None
                }
            }
        } else {
            None
        };

        let output_len = if let Some(ref r) = resampler {
            r.output_frames_max()
        } else {
            RESAMPLE_CHUNK
        };

        Self {
            src_channels,
            dst_channels,
            src_sample_rate,
            dst_sample_rate,
            resampler,
            input_planes: vec![Vec::new(); src_channels],
            output_planes: vec![vec![0.0; output_len]; src_channels],
            dst_frame_buf: vec![0.0; dst_channels],
            src_frame_buf: vec![0.0; src_channels],
        }
    }

    pub fn reset(&mut self) {
        for plane in &mut self.input_planes {
            plane.clear();
        }
        for plane in &mut self.output_planes {
            plane.fill(0.0);
        }
        // Recreate the resampler to flush its internal delay line.
        if self.resampler.is_some() {
            self.resampler = Fft::<f32>::new_custom(
                self.src_sample_rate as usize,
                self.dst_sample_rate as usize,
                RESAMPLE_CHUNK,
                2,
                self.src_channels,
                WindowFunction::BlackmanHarris2,
                FixedSync::Input,
            )
            .ok();
        }
    }

    /// Converts interleaved input samples (src_channels @ src_sample_rate)
    /// into interleaved output samples (dst_channels @ dst_sample_rate) and
    /// appends them to `output`.
    pub fn convert(&mut self, input: &[f32], output: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }

        let input_frame_count = input.len() / self.src_channels;
        if input_frame_count == 0 {
            return;
        }

        // ── Fast path: identical layout ──────────────────────────────────────
        if self.src_channels == self.dst_channels && self.resampler.is_none() {
            output.extend_from_slice(input);
            return;
        }

        // ── Channel-only conversion (same rate) ──────────────────────────────
        if self.resampler.is_none() {
            let dst_ch = self.dst_channels;
            let src_ch = self.src_channels;
            let mut dst_frame = [0.0f32; 8];
            for f in 0..input_frame_count {
                let src_frame = &input[f * src_ch..(f + 1) * src_ch];
                if dst_ch <= 8 {
                    downmix_frame(src_frame, &mut dst_frame[..dst_ch]);
                    output.extend_from_slice(&dst_frame[..dst_ch]);
                } else {
                    self.dst_frame_buf.fill(0.0);
                    downmix_frame(src_frame, &mut self.dst_frame_buf);
                    output.extend_from_slice(&self.dst_frame_buf);
                }
            }
            return;
        }

        // ── De-interleave into planar buffers ────────────────────────────────
        // Append into persistent channel buffers instead of wiping them out.
        // This ensures unaligned/residual frames from variable-block codecs
        // (like FLAC) are carried over cleanly without clicks or pops.
        for f in 0..input_frame_count {
            for c in 0..self.src_channels {
                self.input_planes[c].push(input[f * self.src_channels + c]);
            }
        }

        // ── Process in RESAMPLE_CHUNK-sized blocks ───────────────────────────
        let resampler = self.resampler.as_mut().unwrap();
        let chunk = resampler.input_frames_next();
        let mut offset = 0;
        let total_buffered_frames = self.input_planes[0].len();
        let src_channels = self.src_channels;
        let dst_channels = self.dst_channels;

        let mut slices: Vec<&[f32]> = Vec::with_capacity(src_channels);

        while offset + chunk <= total_buffered_frames {
            // Slice exactly `chunk` frames from each plane
            slices.clear();
            for p in &self.input_planes {
                slices.push(&p[offset..offset + chunk]);
            }

            let input_adapter = match SequentialSliceOfSlices::new(&slices, src_channels, chunk) {
                Ok(adapter) => adapter,
                Err(e) => {
                    warn!("Resampler input adapter error: {}", e);
                    break;
                }
            };
            let out_capacity = self.output_planes.first().map(Vec::len).unwrap_or(0);
            let mut output_adapter = match SequentialSliceOfVecs::new_mut(
                &mut self.output_planes,
                src_channels,
                out_capacity,
            ) {
                Ok(adapter) => adapter,
                Err(e) => {
                    warn!("Resampler output adapter error: {}", e);
                    break;
                }
            };

            match resampler.process_into_buffer(&input_adapter, &mut output_adapter, None) {
                Ok((_, out_frames)) => {
                    // Re-interleave src_channels planes → dst_channels output
                    let mut src_frame_stack = [0.0f32; 8];
                    let mut dst_frame_stack = [0.0f32; 8];

                    for f in 0..out_frames {
                        if src_channels <= 8 && dst_channels <= 8 {
                            for (c, sample) in
                                src_frame_stack[..src_channels].iter_mut().enumerate()
                            {
                                *sample = self.output_planes[c][f];
                            }
                            downmix_frame(
                                &src_frame_stack[..src_channels],
                                &mut dst_frame_stack[..dst_channels],
                            );
                            output.extend_from_slice(&dst_frame_stack[..dst_channels]);
                        } else {
                            if self.src_frame_buf.len() < src_channels {
                                self.src_frame_buf.resize(src_channels, 0.0);
                            }
                            if self.dst_frame_buf.len() < dst_channels {
                                self.dst_frame_buf.resize(dst_channels, 0.0);
                            }
                            for (c, sample) in
                                self.src_frame_buf.iter_mut().enumerate().take(src_channels)
                            {
                                *sample = self.output_planes[c][f];
                            }
                            downmix_frame(&self.src_frame_buf, &mut self.dst_frame_buf);
                            output.extend_from_slice(&self.dst_frame_buf);
                        }
                    }
                }
                Err(e) => {
                    warn!("Resampler process error: {}", e);
                }
            }

            offset += chunk;
        }

        // Drain the consumed frames, retaining any residual frames (< chunk) for the next packet.
        if offset > 0 {
            for plane in &mut self.input_planes {
                plane.drain(0..offset);
            }
        }
    }

    /// Flushes any leftover input frames on end-of-stream by zero-padding to a full chunk.
    pub fn flush(&mut self, output: &mut Vec<f32>) {
        if self.resampler.is_none() {
            return;
        }

        let pending_frames = self.input_planes[0].len();
        if pending_frames == 0 {
            return;
        }

        let resampler = self.resampler.as_mut().unwrap();
        let chunk = resampler.input_frames_next();

        if pending_frames < chunk {
            // Pad remaining frames with silence to fill one final chunk
            for plane in &mut self.input_planes {
                plane.resize(chunk, 0.0);
            }
        }

        let mut slices: Vec<&[f32]> = Vec::with_capacity(self.src_channels);
        for p in &self.input_planes {
            slices.push(&p[0..chunk]);
        }

        let src_channels = self.src_channels;
        let dst_channels = self.dst_channels;

        let input_adapter = match SequentialSliceOfSlices::new(&slices, src_channels, chunk) {
            Ok(adapter) => adapter,
            Err(e) => {
                warn!("Resampler flush input adapter error: {}", e);
                return;
            }
        };
        let out_capacity = self.output_planes.first().map(Vec::len).unwrap_or(0);
        let mut output_adapter = match SequentialSliceOfVecs::new_mut(
            &mut self.output_planes,
            src_channels,
            out_capacity,
        ) {
            Ok(adapter) => adapter,
            Err(e) => {
                warn!("Resampler flush output adapter error: {}", e);
                return;
            }
        };

        match resampler.process_into_buffer(&input_adapter, &mut output_adapter, None) {
            Ok((_, out_frames)) => {
                let mut src_frame_stack = [0.0f32; 8];
                let mut dst_frame_stack = [0.0f32; 8];

                for f in 0..out_frames {
                    if src_channels <= 8 && dst_channels <= 8 {
                        for (c, sample) in src_frame_stack[..src_channels].iter_mut().enumerate() {
                            *sample = self.output_planes[c][f];
                        }
                        downmix_frame(
                            &src_frame_stack[..src_channels],
                            &mut dst_frame_stack[..dst_channels],
                        );
                        output.extend_from_slice(&dst_frame_stack[..dst_channels]);
                    } else {
                        if self.src_frame_buf.len() < src_channels {
                            self.src_frame_buf.resize(src_channels, 0.0);
                        }
                        if self.dst_frame_buf.len() < dst_channels {
                            self.dst_frame_buf.resize(dst_channels, 0.0);
                        }
                        for (c, sample) in
                            self.src_frame_buf.iter_mut().enumerate().take(src_channels)
                        {
                            *sample = self.output_planes[c][f];
                        }
                        downmix_frame(&self.src_frame_buf, &mut self.dst_frame_buf);
                        output.extend_from_slice(&self.dst_frame_buf);
                    }
                }
            }
            Err(e) => {
                warn!("Resampler flush error: {}", e);
            }
        }

        for plane in &mut self.input_planes {
            plane.clear();
        }
    }
}
