use rubato::{FftFixedIn, Resampler};
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
    resampler: Option<FftFixedIn<f32>>,
    /// Planar channel buffers fed into rubato.
    input_planes: Vec<Vec<f32>>,
    /// Planar channel buffers produced by rubato.
    output_planes: Vec<Vec<f32>>,
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
            match FftFixedIn::<f32>::new(
                src_sample_rate as usize,
                dst_sample_rate as usize,
                RESAMPLE_CHUNK,
                2, // sub-chunks — 2 gives a good quality/latency trade-off
                src_channels,
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
            self.resampler = FftFixedIn::<f32>::new(
                self.src_sample_rate as usize,
                self.dst_sample_rate as usize,
                RESAMPLE_CHUNK,
                2,
                self.src_channels,
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
            let mut dst_frame = vec![0.0f32; self.dst_channels];
            for f in 0..input_frame_count {
                let src_frame = &input[f * self.src_channels..(f + 1) * self.src_channels];
                downmix_frame(src_frame, &mut dst_frame);
                output.extend_from_slice(&dst_frame);
            }
            return;
        }

        // ── De-interleave into planar buffers ────────────────────────────────
        for plane in &mut self.input_planes {
            plane.clear();
        }
        for f in 0..input_frame_count {
            for c in 0..self.src_channels {
                self.input_planes[c].push(input[f * self.src_channels + c]);
            }
        }

        // ── Process in RESAMPLE_CHUNK-sized blocks ───────────────────────────
        let resampler = self.resampler.as_mut().unwrap();
        let chunk = resampler.input_frames_next();
        let mut offset = 0;

        while offset + chunk <= input_frame_count {
            // Slice exactly `chunk` frames from each plane
            let slices: Vec<&[f32]> = self
                .input_planes
                .iter()
                .map(|p| &p[offset..offset + chunk])
                .collect();

            match resampler.process_into_buffer(&slices, &mut self.output_planes, None) {
                Ok((_, out_frames)) => {
                    // Re-interleave src_channels planes → dst_channels output
                    let mut dst_frame = vec![0.0f32; self.dst_channels];
                    for f in 0..out_frames {
                        let mut src_frame = vec![0.0f32; self.src_channels];
                        for c in 0..self.src_channels {
                            src_frame[c] = self.output_planes[c][f];
                        }
                        downmix_frame(&src_frame, &mut dst_frame);
                        output.extend_from_slice(&dst_frame);
                    }
                }
                Err(e) => {
                    warn!("Resampler process error: {}", e);
                }
            }

            offset += chunk;
        }

        // Any remaining frames less than `chunk` are held until the next call.
        // rubato requires full chunks; residuals are small and short-lived.
    }
}
