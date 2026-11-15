use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[cfg(all(feature = "ffmpeg", feature = "symphonia"))]
compile_error!("Features `ffmpeg` and `symphonia` are mutually exclusive in `audio`.");

#[cfg(feature = "ffmpeg")]
mod ffmpeg;
#[cfg(feature = "symphonia")]
mod symphonia;

#[cfg(feature = "ffmpeg")]
pub(crate) use self::ffmpeg::LoadedTrack;
#[cfg(feature = "symphonia")]
pub(crate) use self::symphonia::LoadedTrack;

pub(crate) struct RingSink<'a> {
    pub(crate) ring_producer: &'a mut Option<ringbuf::HeapProd<f32>>,
    pub(crate) flush_flag: &'a Arc<AtomicBool>,
    pub(crate) written_samples: &'a mut u64,
}

impl<'a> RingSink<'a> {
    pub(crate) fn is_flushing(&self) -> bool {
        self.flush_flag.load(Ordering::Relaxed)
    }

    pub(crate) fn push_interleaved(&mut self, samples: &[f32]) -> bool {
        if let Some(ref mut prod) = self.ring_producer {
            crate::engine::push_to_ring_buffer(prod, samples, self.flush_flag, self.written_samples)
        } else {
            true
        }
    }
}

#[cfg(not(any(feature = "ffmpeg", feature = "symphonia")))]
pub(crate) struct LoadedTrack {
    path: std::path::PathBuf,
}

#[cfg(not(any(feature = "ffmpeg", feature = "symphonia")))]
impl LoadedTrack {
    pub(crate) fn load(
        _path: &std::path::Path,
        _dst_sample_rate: u32,
        _dst_channels: usize,
        _resampling_cfg: &crate::resample::ResamplingConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("No audio decoder backend enabled (enable `ffmpeg` or `symphonia` feature)".into())
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub(crate) fn duration_ms(&self) -> u64 {
        0
    }

    pub(crate) fn reached_eof(&self) -> bool {
        true
    }

    pub(crate) fn seek_to_start(&mut self) {}

    pub(crate) fn seek(
        &mut self,
        _target: std::time::Duration,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    pub(crate) fn reconfigure_resampler(
        &mut self,
        _dst_sample_rate: u32,
        _dst_channels: usize,
        _new_cfg: &crate::resample::ResamplingConfig,
    ) {
    }

    pub(crate) fn decode_step(&mut self, _sink: &mut RingSink<'_>) -> bool {
        true
    }
}
