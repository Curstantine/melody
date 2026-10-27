use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
struct AudioStatusInner {
    position_ms: AtomicU64,
    duration_ms: AtomicU64,
    is_playing: AtomicBool,
    is_loaded: AtomicBool,
    volume_raw: AtomicU32,
    is_ended: AtomicBool,
    gapless_transition: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct AudioStatus {
    inner: Arc<AudioStatusInner>,
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStatus {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AudioStatusInner {
                position_ms: AtomicU64::new(0),
                duration_ms: AtomicU64::new(0),
                is_playing: AtomicBool::new(false),
                is_loaded: AtomicBool::new(false),
                volume_raw: AtomicU32::new(1000), // Default 1.0 volume (1000 / 1000)
                is_ended: AtomicBool::new(false),
                gapless_transition: AtomicBool::new(false),
            }),
        }
    }

    #[inline]
    pub fn position_ms(&self) -> u64 {
        self.inner.position_ms.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_position_ms(&self, ms: u64) {
        self.inner.position_ms.store(ms, Ordering::Relaxed);
    }

    #[inline]
    pub fn duration_ms(&self) -> u64 {
        self.inner.duration_ms.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_duration_ms(&self, ms: u64) {
        self.inner.duration_ms.store(ms, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_playing(&self) -> bool {
        self.inner.is_playing.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_playing(&self, playing: bool) {
        self.inner.is_playing.store(playing, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_loaded(&self) -> bool {
        self.inner.is_loaded.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_loaded(&self, loaded: bool) {
        self.inner.is_loaded.store(loaded, Ordering::Relaxed);
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        (self.inner.volume_raw.load(Ordering::Relaxed) as f32) / 1000.0
    }

    #[inline]
    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 2.0);
        let raw = (clamped * 1000.0).round() as u32;
        self.inner.volume_raw.store(raw, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_ended(&self) -> bool {
        self.inner.is_ended.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_ended(&self, ended: bool) {
        self.inner.is_ended.store(ended, Ordering::Relaxed);
    }

    #[inline]
    pub fn take_ended(&self) -> bool {
        self.inner.is_ended.swap(false, Ordering::Relaxed)
    }

    #[inline]
    pub fn gapless_transition(&self) -> bool {
        self.inner.gapless_transition.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_gapless_transition(&self, val: bool) {
        self.inner.gapless_transition.store(val, Ordering::Release);
    }

    #[inline]
    pub fn take_gapless_transition(&self) -> bool {
        self.inner.gapless_transition.swap(false, Ordering::AcqRel)
    }
}
