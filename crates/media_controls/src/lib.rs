#[cfg(target_os = "linux")]
mod mpris;
#[cfg(target_os = "windows")]
mod windows_media;

use std::sync::Arc;

use crossbeam_channel::Sender;
use plugin::{PlayerAction, TrackMetadata};

/// Cross-platform OS media transport controls (MPRIS on Linux, SMTC on Windows).
#[derive(Clone)]
pub struct MediaControls {
    #[cfg(target_os = "linux")]
    inner: Arc<mpris::MprisControls>,
    #[cfg(target_os = "windows")]
    inner: Arc<windows_media::WindowsMediaControls>,
}

impl MediaControls {
    pub fn new(action_tx: Sender<PlayerAction>) -> Self {
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = action_tx;
        Self {
            #[cfg(target_os = "linux")]
            inner: Arc::new(mpris::MprisControls::new(action_tx)),
            #[cfg(target_os = "windows")]
            inner: Arc::new(windows_media::WindowsMediaControls::new(action_tx)),
        }
    }

    pub fn attach_window(&self, hwnd: isize) {
        #[cfg(target_os = "windows")]
        self.inner.attach_window(hwnd);
        #[cfg(not(target_os = "windows"))]
        let _ = hwnd;
    }

    pub fn detach_window(&self) {
        #[cfg(target_os = "windows")]
        self.inner.detach_window();
    }

    pub fn on_track_change(&self, track: &TrackMetadata) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_track_change(track);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = track;
    }

    pub fn on_playback_end(&self) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_playback_end();
    }

    pub fn on_playback_status_change(&self, is_playing: bool) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_playback_status_change(is_playing);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = is_playing;
    }

    pub fn on_position_change(&self, position_ms: u64) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_position_change(position_ms);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = position_ms;
    }

    pub fn on_seeked(&self, position_ms: u64) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_seeked(position_ms);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = position_ms;
    }

    pub fn on_volume_change(&self, volume: f32) {
        #[cfg(target_os = "linux")]
        self.inner.on_volume_change(volume);
        #[cfg(not(target_os = "linux"))]
        let _ = volume;
    }

    pub fn on_repeat_change(&self, mode: model::RepeatMode) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_repeat_change(mode);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = mode;
    }

    pub fn on_shuffle_change(&self, is_shuffle: bool) {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        self.inner.on_shuffle_change(is_shuffle);
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = is_shuffle;
    }
}
