pub mod albums;
pub mod artists;
pub mod library;
pub mod now_playing;
pub mod player_bar;
pub mod playlists;
pub mod settings;
pub mod status_bar;

pub use albums::AlbumsView;
pub use artists::ArtistsView;
pub use library::LibraryView;
pub use now_playing::NowPlayingView;
pub use player_bar::PlayerBar;
pub use playlists::PlaylistsView;
pub use settings::SettingsView;
pub use status_bar::{StatusBar, StatusBarContext};

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
