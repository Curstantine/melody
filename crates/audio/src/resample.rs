use serde::{Deserialize, Serialize};
use std::fmt;

/// Sinc resampling filter algorithm used for sample rate conversion in `libswresample`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResamplingFilter {
    /// Kaiser windowed sinc filter (libswresample default).
    #[default]
    Kaiser,
    /// Blackman-Nuttall windowed sinc filter (>98 dB stopband attenuation).
    BlackmanNuttall,
    /// Cubic spline polynomial interpolation (lightweight, non-sinc).
    Cubic,
    /// SoX Resampler library engine (when enabled in FFmpeg).
    Soxr,
}

impl ResamplingFilter {
    pub const ALL: [ResamplingFilter; 4] = [
        ResamplingFilter::Kaiser,
        ResamplingFilter::BlackmanNuttall,
        ResamplingFilter::Cubic,
        ResamplingFilter::Soxr,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Kaiser => "Kaiser Windowed Sinc (Default)",
            Self::BlackmanNuttall => "Blackman-Nuttall Windowed Sinc",
            Self::Cubic => "Cubic Polynomial (Fast)",
            Self::Soxr => "SoX Resampler (soxr)",
        }
    }
}

impl fmt::Display for ResamplingFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Sinc filter length / precision controlling transition band steepness and aliasing rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResamplingQuality {
    /// 16 filter taps / 16-bit precision.
    Fast,
    /// 32 filter taps / 20-bit precision (libswresample default).
    #[default]
    Standard,
    /// 64 filter taps / 28-bit precision.
    High,
    /// 128 filter taps / 32-bit precision.
    Ultra,
}

impl ResamplingQuality {
    pub const ALL: [ResamplingQuality; 4] = [
        ResamplingQuality::Fast,
        ResamplingQuality::Standard,
        ResamplingQuality::High,
        ResamplingQuality::Ultra,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Fast => "Fast (16 taps)",
            Self::Standard => "Standard (32 taps - Default)",
            Self::High => "High Quality (64 taps)",
            Self::Ultra => "Audiophile / Ultra (128 taps)",
        }
    }
}

impl fmt::Display for ResamplingQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

fn default_linear_interp() -> bool {
    true
}

/// Configuration preferences for `libswresample` audio sample rate conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResamplingConfig {
    #[serde(default)]
    pub filter: ResamplingFilter,
    #[serde(default)]
    pub quality: ResamplingQuality,
    #[serde(default = "default_linear_interp")]
    pub linear_interp: bool,
}

impl Default for ResamplingConfig {
    fn default() -> Self {
        Self {
            filter: ResamplingFilter::default(),
            quality: ResamplingQuality::default(),
            linear_interp: default_linear_interp(),
        }
    }
}

impl ResamplingConfig {
    /// Builds an FFmpeg AVOptions dictionary to pass into `ResamplerContext::get_with`.
    pub fn to_dictionary(&self) -> ffmpeg_next::Dictionary<'static> {
        let mut dict = ffmpeg_next::Dictionary::new();
        match self.filter {
            ResamplingFilter::Soxr => {
                dict.set("resampler", "soxr");
                let precision = match self.quality {
                    ResamplingQuality::Fast => "16",
                    ResamplingQuality::Standard => "20",
                    ResamplingQuality::High => "28",
                    ResamplingQuality::Ultra => "32",
                };
                dict.set("precision", precision);
            }
            swr_filter => {
                dict.set("resampler", "swr");
                let filter_name = match swr_filter {
                    ResamplingFilter::Kaiser => "kaiser",
                    ResamplingFilter::BlackmanNuttall => "blackman_nuttall",
                    ResamplingFilter::Cubic => "cubic",
                    ResamplingFilter::Soxr => unreachable!(),
                };
                dict.set("filter_type", filter_name);

                let filter_size = match self.quality {
                    ResamplingQuality::Fast => "16",
                    ResamplingQuality::Standard => "32",
                    ResamplingQuality::High => "64",
                    ResamplingQuality::Ultra => "128",
                };
                dict.set("filter_size", filter_size);
                dict.set("linear_interp", if self.linear_interp { "1" } else { "0" });
            }
        }
        dict
    }
}
