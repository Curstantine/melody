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
