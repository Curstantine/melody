use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AudioStatus {
    position_ms: Arc<AtomicU64>,
    duration_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
    is_loaded: Arc<AtomicBool>,
    volume_raw: Arc<AtomicU32>,
    is_ended: Arc<AtomicBool>,
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioStatus {
    pub fn new() -> Self {
        Self {
            position_ms: Arc::new(AtomicU64::new(0)),
            duration_ms: Arc::new(AtomicU64::new(0)),
            is_playing: Arc::new(AtomicBool::new(false)),
            is_loaded: Arc::new(AtomicBool::new(false)),
            volume_raw: Arc::new(AtomicU32::new(1000)), // Default 1.0 volume (1000 / 1000)
            is_ended: Arc::new(AtomicBool::new(false)),
        }
    }

    #[inline]
    pub fn position_ms(&self) -> u64 {
        self.position_ms.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_position_ms(&self, ms: u64) {
        self.position_ms.store(ms, Ordering::Relaxed);
    }

    #[inline]
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_duration_ms(&self, ms: u64) {
        self.duration_ms.store(ms, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_playing(&self, playing: bool) {
        self.is_playing.store(playing, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_loaded(&self) -> bool {
        self.is_loaded.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_loaded(&self, loaded: bool) {
        self.is_loaded.store(loaded, Ordering::Relaxed);
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        (self.volume_raw.load(Ordering::Relaxed) as f32) / 1000.0
    }

    #[inline]
    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 2.0);
        let raw = (clamped * 1000.0).round() as u32;
        self.volume_raw.store(raw, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_ended(&self) -> bool {
        self.is_ended.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_ended(&self, ended: bool) {
        self.is_ended.store(ended, Ordering::Relaxed);
    }

    #[inline]
    pub fn take_ended(&self) -> bool {
        self.is_ended.swap(false, Ordering::Relaxed)
    }
}
