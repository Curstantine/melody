pub mod albums;
pub mod artists;
pub mod library;
pub mod now_playing;
pub mod player_bar;
pub mod playlists;
pub mod settings;
pub mod status_bar;
pub mod titlebar;

pub use albums::AlbumsView;
pub use artists::ArtistsView;
pub use library::LibraryView;
pub use now_playing::NowPlayingView;
pub use player_bar::PlayerBar;
pub use playlists::PlaylistsView;
pub use settings::SettingsView;
pub use status_bar::{StatusBar, StatusBarContext};
pub use titlebar::{resize_handles_overlay, window_controls_row, TitlebarView};

use std::sync::atomic::{AtomicU32, Ordering};

/// Thread-safe, lock-free scroll offset tracking using an atomic bit representation of `f32`.
/// Allows view components to record scroll positions inside `on_scroll` callbacks
/// without emitting messages that trigger application updates and re-renders.
#[derive(Debug, Default)]
pub struct ScrollOffset(AtomicU32);

impl ScrollOffset {
    pub const fn new(offset: f32) -> Self {
        Self(AtomicU32::new(offset.to_bits()))
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, offset: f32) {
        self.0.store(offset.to_bits(), Ordering::Relaxed);
    }
}

impl Clone for ScrollOffset {
    fn clone(&self) -> Self {
        Self::new(self.get())
    }
}

impl PartialEq for ScrollOffset {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

/// Computes a clamped dimension for a collapsible element given `available` space,
/// subtracting `reserved` space and returning `None` if the remainder is below `min`.
pub fn collapsible_size(available: f32, reserved: f32, min: f32, max: f32) -> Option<f32> {
    let remaining = available - reserved;
    (remaining >= min).then(|| remaining.min(max))
}

#[cfg(test)]
mod tests {
    use super::collapsible_size;

    #[test]
    fn test_collapsible_size_scales_and_hides() {
        let reserved = 376.0;
        let min = 120.0;
        let max = 340.0;

        assert_eq!(collapsible_size(800.0, reserved, min, max), Some(max));
        assert_eq!(
            collapsible_size(reserved + max, reserved, min, max),
            Some(max)
        );
        assert_eq!(collapsible_size(600.0, reserved, min, max), Some(224.0));
        assert_eq!(collapsible_size(500.0, reserved, min, max), Some(124.0));
        assert_eq!(
            collapsible_size(reserved + min, reserved, min, max),
            Some(min)
        );
        assert_eq!(
            collapsible_size(reserved + min - 1.0, reserved, min, max),
            None
        );
        assert_eq!(collapsible_size(250.0, reserved, min, max), None);
    }
}
