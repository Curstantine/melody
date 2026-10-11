pub struct AudioConverter {
    src_channels: usize,
    dst_channels: usize,
    src_sample_rate: u32,
    dst_sample_rate: u32,
    ratio: f64, // src_sample_rate / dst_sample_rate
    phase: f64,
    last_frame: Vec<f32>,
}

impl AudioConverter {
    pub fn new(
        src_channels: usize,
        src_sample_rate: u32,
        dst_channels: usize,
        dst_sample_rate: u32,
    ) -> Self {
        let src_channels = src_channels.max(1);
        let dst_channels = dst_channels.max(1);
        let ratio = (src_sample_rate as f64) / (dst_sample_rate.max(1) as f64);

        Self {
            src_channels,
            dst_channels,
            src_sample_rate,
            dst_sample_rate,
            ratio,
            phase: 0.0,
            last_frame: vec![0.0; dst_channels],
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.last_frame.fill(0.0);
    }

    /// Converts interleaved input samples at src_channels & src_sample_rate
    /// into interleaved output samples at dst_channels & dst_sample_rate.
    pub fn convert(&mut self, input: &[f32], output: &mut Vec<f32>) {
        if input.is_empty() {
            return;
        }

        // Fast path: no channel mapping and same sample rate
        if self.src_channels == self.dst_channels && self.src_sample_rate == self.dst_sample_rate {
            output.extend_from_slice(input);
            return;
        }

        let input_frame_count = input.len() / self.src_channels;
        if input_frame_count == 0 {
            return;
        }

        // Helper to extract a single frame mapped to dst_channels
        let get_frame = |frame_idx: usize, out_frame: &mut [f32]| {
            let offset = frame_idx * self.src_channels;
            if self.src_channels == 1 && self.dst_channels == 2 {
                let sample = input[offset];
                out_frame[0] = sample;
                out_frame[1] = sample;
            } else if self.src_channels == self.dst_channels {
                for c in 0..self.dst_channels {
                    out_frame[c] = input[offset + c];
                }
            } else if self.src_channels > self.dst_channels {
                // Downmix or truncate
                for c in 0..self.dst_channels {
                    out_frame[c] = input[offset + c];
                }
            } else {
                // Less source channels than destination
                for c in 0..self.dst_channels {
                    out_frame[c] = if c < self.src_channels {
                        input[offset + c]
                    } else {
                        0.0
                    };
                }
            }
        };

        if self.src_sample_rate == self.dst_sample_rate {
            // Only channel conversion
            let mut frame = vec![0.0; self.dst_channels];
            for f in 0..input_frame_count {
                get_frame(f, &mut frame);
                output.extend_from_slice(&frame);
            }
            return;
        }

        // Resampling with linear interpolation
        let mut curr_frame = vec![0.0; self.dst_channels];
        let mut next_frame = vec![0.0; self.dst_channels];

        while self.phase < (input_frame_count as f64) {
            let idx = self.phase.floor() as usize;
            let frac = (self.phase - idx as f64) as f32;

            get_frame(idx, &mut curr_frame);

            if idx + 1 < input_frame_count {
                get_frame(idx + 1, &mut next_frame);
            } else {
                next_frame.copy_from_slice(&curr_frame);
            }

            for c in 0..self.dst_channels {
                let interpolated = curr_frame[c] * (1.0 - frac) + next_frame[c] * frac;
                output.push(interpolated);
            }

            self.phase += self.ratio;
        }

        self.phase -= input_frame_count as f64;
    }
}
