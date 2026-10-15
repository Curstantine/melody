use rubato::audioadapter_buffers::direct::{SequentialSliceOfSlices, SequentialSliceOfVecs};
use rubato::{Fft, FixedSync, Indexing, Resampler, ResamplerConstructionError, WindowFunction};
use tracing::warn;

/// ITU-R BS.775 downmix coefficients for center and surround channels.
const CENTER_COEFF: f32 = 0.707; // -3 dB  (1 / sqrt(2))
const SURROUND_COEFF: f32 = 0.707; // -3 dB

/// Converts a single source frame (arbitrary channel count) into a
/// destination frame with `dst_channels` channels, following ITU-R BS.775
/// downmix coefficients.
///
/// Source layouts are FFmpeg canonical channel orders (as produced by
/// `ResamplerContext` with the target layouts in `target_channel_layout`):
/// - 3 (L R C)       → 2: L += C*k, R += C*k
/// - 4 (L R Ls Rs)   → 2: L += Ls*k, R += Rs*k
/// - 5 (L R C Ls Rs) → 2: L += C*k + Ls*k, R += C*k + Rs*k (LFE absent)
/// - 6 (5.1)         → 2: ITU-R BS.775 standard downmix (LFE omitted)
/// - 8 (7.1)         → 2: standard downmix
/// - 1 (Mono)        → N: duplicate to all channels
/// - 2 (Stereo)      → 2: identity
/// - N → 1           → downmix to stereo first, then average
/// - Anything else   → copy first dst_channels, zero the rest
///
/// Unlike raw BS.775 (designed for chains with headroom), each mix is
/// normalized by its coefficient sum: our f32 pipeline has no headroom
/// above 1.0, so unnormalized sums (up to ~3.1 for 7.1) would hard-clip
/// loud material. Normalization costs ~3–8 dB of level, recoverable via
/// the player's volume control.
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

        // ── 3.0 (L R C, FFmpeg SURROUND order) → stereo ────────────────────
        (3, 2) => {
            let norm = 1.0 + CENTER_COEFF;
            dst[0] = (src[0] + src[2] * CENTER_COEFF) / norm;
            dst[1] = (src[1] + src[2] * CENTER_COEFF) / norm;
        }

        // ── 4.0 (L R Ls Rs) → stereo ─────────────────────────────────────────
        (4, 2) => {
            let norm = 1.0 + SURROUND_COEFF;
            dst[0] = (src[0] + src[2] * SURROUND_COEFF) / norm;
            dst[1] = (src[1] + src[3] * SURROUND_COEFF) / norm;
        }

        // ── 5.0 (L R C Ls Rs, no LFE) → stereo ───────────────────────────────
        (5, 2) => {
            let norm = 1.0 + CENTER_COEFF + SURROUND_COEFF;
            dst[0] = (src[0] + src[2] * CENTER_COEFF + src[3] * SURROUND_COEFF) / norm;
            dst[1] = (src[1] + src[2] * CENTER_COEFF + src[4] * SURROUND_COEFF) / norm;
        }

        // ── 5.1 (L R C LFE Ls Rs) → stereo ──────────────────────────────────
        (6, 2) => {
            let norm = 1.0 + CENTER_COEFF + SURROUND_COEFF;
            // LFE (src[3]) is omitted in standard stereo downmix
            dst[0] = (src[0] + src[2] * CENTER_COEFF + src[4] * SURROUND_COEFF) / norm;
            dst[1] = (src[1] + src[2] * CENTER_COEFF + src[5] * SURROUND_COEFF) / norm;
        }

        // ── 7.1 (L R C LFE Ls Rs Lss Rss) → stereo ──────────────────────────
        (8, 2) => {
            let norm = 1.0 + CENTER_COEFF + 2.0 * SURROUND_COEFF;
            dst[0] = (src[0] + src[2] * CENTER_COEFF + (src[4] + src[6]) * SURROUND_COEFF) / norm;
            dst[1] = (src[1] + src[2] * CENTER_COEFF + (src[5] + src[7]) * SURROUND_COEFF) / norm;
        }

        // ── N → mono: downmix to stereo first, then average ───────────────────
        // (covers stereo, 5.1, 7.1, …; avoids the old bug of copying just Left)
        (_, 1) => {
            let mut stereo = [0.0f32; 2];
            downmix_frame(src, &mut stereo);
            dst[0] = (stereo[0] + stereo[1]) * 0.5;
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

/// Outcome of [`AudioConverter::process_buffered_chunk`], telling the caller
/// whether the chunk was consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkOutcome {
    /// Chunk processed and emitted; caller should advance past it.
    Processed,
    /// rubato failed on this chunk; caller should advance past it anyway
    /// (a click beats wedging the worker on a poisoned chunk).
    Skipped,
    /// Adapter/underrun failure; caller should abort and keep the buffered data.
    Aborted,
}

/// Interleaves a single resampled planar frame into `output`, applying
/// channel downmixing. A free function over disjoint buffers so callers can
/// use it while a borrow of the resampler itself is live.
fn emit_planar_frame(
    output_planes: &[Vec<f32>],
    frame: usize,
    src_channels: usize,
    dst_channels: usize,
    src_scratch: &mut Vec<f32>,
    dst_scratch: &mut Vec<f32>,
    output: &mut Vec<f32>,
) {
    if src_channels <= 8 && dst_channels <= 8 {
        let mut src_frame = [0.0f32; 8];
        let mut dst_frame = [0.0f32; 8];
        for (c, sample) in src_frame[..src_channels].iter_mut().enumerate() {
            *sample = output_planes[c][frame];
        }
        downmix_frame(&src_frame[..src_channels], &mut dst_frame[..dst_channels]);
        output.extend_from_slice(&dst_frame[..dst_channels]);
    } else {
        if src_scratch.len() < src_channels {
            src_scratch.resize(src_channels, 0.0);
        }
        if dst_scratch.len() < dst_channels {
            dst_scratch.resize(dst_channels, 0.0);
        }
        for (c, sample) in src_scratch.iter_mut().enumerate().take(src_channels) {
            *sample = output_planes[c][frame];
        }
        downmix_frame(src_scratch, dst_scratch);
        output.extend_from_slice(&dst_scratch[..dst_channels]);
    }
}

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
    /// Leading output frames of FFT filter pre-response still to drop.
    /// An FFT resampler's first `output_delay()` output frames are not real
    /// audio; emitting them would prepend ~6–12 ms of silence/pre-ringing and
    /// shift every seek position. Re-armed by `reset()`.
    delay_trim_remaining: usize,
    /// Input frames fed into the resample path, for exact end-of-stream accounting.
    total_input_frames: u64,
    /// Output frames appended so far, excluding delay-trimmed frames.
    emitted_frames: u64,
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
    ) -> Result<Self, ResamplerConstructionError> {
        let src_channels = src_channels.max(1);
        let dst_channels = dst_channels.max(1);

        // Build a rubato FFT resampler when sample rates differ.
        // Construction failure is returned loudly: silently falling back to
        // no resampling would play audio at the wrong rate (pitch-shifted).
        let resampler = if src_sample_rate != dst_sample_rate {
            Some(Fft::<f32>::new_custom(
                src_sample_rate as usize,
                dst_sample_rate as usize,
                RESAMPLE_CHUNK,
                2, // sub-chunks — 2 gives a good quality/latency trade-off
                src_channels,
                WindowFunction::BlackmanHarris2,
                FixedSync::Input,
            )?)
        } else {
            None
        };

        let (output_len, delay_trim_remaining) = match resampler {
            Some(ref r) => (r.output_frames_max(), r.output_delay()),
            None => (RESAMPLE_CHUNK, 0),
        };

        Ok(Self {
            src_channels,
            dst_channels,
            src_sample_rate,
            dst_sample_rate,
            resampler,
            input_planes: vec![Vec::new(); src_channels],
            output_planes: vec![vec![0.0; output_len]; src_channels],
            dst_frame_buf: vec![0.0; dst_channels],
            src_frame_buf: vec![0.0; src_channels],
            delay_trim_remaining,
            total_input_frames: 0,
            emitted_frames: 0,
        })
    }

    pub fn reset(&mut self) {
        for plane in &mut self.input_planes {
            plane.clear();
        }
        // rubato's reset clears the delay line without reallocating.
        if let Some(r) = self.resampler.as_mut() {
            r.reset();
            self.delay_trim_remaining = r.output_delay();
        } else {
            self.delay_trim_remaining = 0;
        }
        self.total_input_frames = 0;
        self.emitted_frames = 0;
    }

    /// Converts interleaved input samples (src_channels @ src_sample_rate)
    /// into interleaved output samples (dst_channels @ dst_sample_rate) and
    /// appends them to `output`.
    pub fn convert(&mut self, input: &[f32], output: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }

        // Caller invariant: whole frames only. A trailing partial frame would
        // be silently dropped below; flag it loudly in debug builds.
        debug_assert_eq!(
            input.len() % self.src_channels,
            0,
            "resampler input must contain whole frames"
        );
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
        self.total_input_frames += input_frame_count as u64;

        // ── Process in RESAMPLE_CHUNK-sized blocks ───────────────────────────
        // The input chunk size is constant for fixed-input resampling; read it
        // once up front. The resampler itself is re-borrowed per iteration so
        // `emit_frames` can take `&mut self` without borrow conflicts.
        let chunk = self
            .resampler
            .as_ref()
            .map(Resampler::input_frames_next)
            .unwrap_or(RESAMPLE_CHUNK);
        let mut offset = 0;
        let total_buffered_frames = self.input_planes[0].len();

        while offset + chunk <= total_buffered_frames {
            // Process errors skip the chunk (audible click, but stalling would
            // wedge the worker); adapter errors abort and keep the residual.
            match self.process_buffered_chunk(offset, chunk, None, output) {
                ChunkOutcome::Processed | ChunkOutcome::Skipped => offset += chunk,
                ChunkOutcome::Aborted => break,
            }
        }

        // Drain the consumed frames, retaining any residual frames (< chunk) for the next packet.
        if offset > 0 {
            for plane in &mut self.input_planes {
                plane.drain(0..offset);
            }
        }
    }

    /// Interleaves `out_frames` resampled planar frames into `output`,
    /// applying channel downmixing. Drops leading frames still covered by
    /// `delay_trim_remaining` (FFT pre-response, not real audio) and counts
    /// every appended frame in `emitted_frames` for end-of-stream trimming.
    fn emit_frames(&mut self, out_frames: usize, output: &mut Vec<f32>) {
        let skip = self.delay_trim_remaining.min(out_frames);
        self.delay_trim_remaining -= skip;

        let src_channels = self.src_channels;
        let dst_channels = self.dst_channels;
        // Borrow disjoint fields directly: `output_planes` (read) and the
        // scratch buffers (write) never alias each other.
        let output_planes = &self.output_planes;
        let src_scratch = &mut self.src_frame_buf;
        let dst_scratch = &mut self.dst_frame_buf;

        for f in skip..out_frames {
            emit_planar_frame(
                output_planes,
                f,
                src_channels,
                dst_channels,
                src_scratch,
                dst_scratch,
                output,
            );
            self.emitted_frames += 1;
        }
    }

    /// Flushes end-of-stream audio: emits the buffered residual frames, pushes
    /// the resampler's internal delay line out, then trims everything to the
    /// exact output length so no leading/trailing silence is invented or lost.
    ///
    /// Protocol (all in output frames):
    /// 1. Leftover input (< 1 chunk) is processed with
    ///    `Indexing { partial_len }` — rubato treats only those frames as real
    ///    audio instead of our old manual zero-pad-as-full-chunk, which
    ///    appended up to ~23 ms of bogus silence.
    /// 2. Pure-silence chunks (`partial_len: Some(0)`) push the delay line out
    ///    so the tail of the audio is emitted rather than lost. This runs even
    ///    when nothing is pending: a stream ending exactly on a chunk boundary
    ///    still has unflushed delay.
    /// 3. The appended tail is truncated to the exact length
    ///    `total_input * dst_rate / src_rate`, cutting the silence response.
    pub fn flush(&mut self, output: &mut Vec<f32>) {
        if self.resampler.is_none() {
            return;
        }

        let appended_start = output.len();
        // Snapshot: `emitted_frames` will grow as this flush appends, so the
        // exact-length budget must be computed against the pre-flush count.
        let emitted_before_flush = self.emitted_frames;
        let dst_channels = self.dst_channels;
        let chunk = self
            .resampler
            .as_ref()
            .map(Resampler::input_frames_next)
            .unwrap_or(RESAMPLE_CHUNK);

        // ── 1. Drain any full chunks first (defensive: convert() normally
        // leaves < 1 chunk buffered) ──────────────────────────────────────────
        while self.input_planes[0].len() >= chunk {
            match self.process_buffered_chunk(0, chunk, None, output) {
                ChunkOutcome::Processed | ChunkOutcome::Skipped => {
                    for plane in &mut self.input_planes {
                        plane.drain(0..chunk);
                    }
                }
                ChunkOutcome::Aborted => break,
            }
        }

        // ── 2. Partial last chunk: pad the adapter to a full chunk but tell
        // rubato only `pending` frames are real ───────────────────────────────
        let pending = self.input_planes.first().map(Vec::len).unwrap_or(0);
        if pending > 0 {
            for plane in &mut self.input_planes {
                plane.resize(chunk, 0.0);
            }
            let indexing = Indexing {
                partial_len: Some(pending),
                ..Default::default()
            };
            self.process_buffered_chunk(0, chunk, Some(&indexing), output);
            for plane in &mut self.input_planes {
                plane.clear();
            }
        }

        // ── 3. Push the delay line out with silence ──────────────────────────
        // `output_delay()` frames of real tail audio are still inside the
        // resampler; each silence chunk releases up to `chunk_size_out` of them.
        let delay = self
            .resampler
            .as_ref()
            .map(Resampler::output_delay)
            .unwrap_or(0);
        let mut drained = 0;
        while drained < delay {
            // Reuse the (now cleared) planes as scratch: with partial_len 0
            // rubato reads no input frames, so contents are irrelevant.
            for plane in &mut self.input_planes {
                plane.resize(chunk, 0.0);
            }
            let indexing = Indexing {
                partial_len: Some(0),
                ..Default::default()
            };
            let before = self.emitted_frames;
            self.process_buffered_chunk(0, chunk, Some(&indexing), output);
            drained += (self.emitted_frames - before) as usize;
            for plane in &mut self.input_planes {
                plane.clear();
            }
            // No progress (shouldn't happen): avoid spinning forever.
            if self.emitted_frames == before {
                break;
            }
        }

        // ── 4. Trim to the exact stream length ───────────────────────────────
        // Total true audio frames, floored (never invent audio). `emitted`
        // counts post-delay-trim frames, so both sides exclude pre-response.
        let expected_total =
            self.total_input_frames * self.dst_sample_rate as u64 / self.src_sample_rate as u64;
        let appended_frames = (output.len() - appended_start) as u64 / dst_channels as u64;
        let keep_frames = expected_total
            .saturating_sub(emitted_before_flush)
            .min(appended_frames);
        output.truncate(appended_start + keep_frames as usize * dst_channels);
        self.emitted_frames += keep_frames;

        // Leave the converter in a clean, reusable state: planes are empty,
        // counters are zeroed, and the delay trim is re-armed for the next
        // stream. Flush is end-of-stream by contract, so re-arming is correct.
        self.reset();
    }

    /// Processes one full `chunk` of frames starting at `offset` in the input
    /// planes through the resampler with the given indexing, emitting the
    /// result via [`Self::emit_frames`].
    fn process_buffered_chunk(
        &mut self,
        offset: usize,
        chunk: usize,
        indexing: Option<&Indexing>,
        output: &mut Vec<f32>,
    ) -> ChunkOutcome {
        let src_channels = self.src_channels;

        let mut slices: Vec<&[f32]> = Vec::with_capacity(src_channels);
        for p in &self.input_planes {
            // Adapter requires full-chunk slices; callers pad short tails.
            if p.len() < offset + chunk {
                warn!("Resampler input underrun: plane too short, dropping chunk");
                return ChunkOutcome::Aborted;
            }
            slices.push(&p[offset..offset + chunk]);
        }

        let input_adapter = match SequentialSliceOfSlices::new(&slices, src_channels, chunk) {
            Ok(adapter) => adapter,
            Err(e) => {
                warn!("Resampler input adapter error: {}", e);
                return ChunkOutcome::Aborted;
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
                return ChunkOutcome::Aborted;
            }
        };

        // Borrow scope dance: `output_adapter` borrows `self.output_planes`
        // while the resampler borrow is live (disjoint fields, fine), then
        // both end before `emit_frames` takes `&mut self`.
        let out_frames = match self.resampler.as_mut().unwrap().process_into_buffer(
            &input_adapter,
            &mut output_adapter,
            indexing,
        ) {
            Ok((_, out_frames)) => out_frames,
            Err(e) => {
                warn!("Resampler process error: {}", e);
                return ChunkOutcome::Skipped;
            }
        };
        // NLL ends the adapter/resampler borrows here; disjoint-field access resumes.
        self.emit_frames(out_frames, output);
        ChunkOutcome::Processed
    }
}
