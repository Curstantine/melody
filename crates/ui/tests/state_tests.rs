#![allow(dead_code)]

use std::path::PathBuf;

#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/state.rs"]
mod state;

use message::{AlbumSortField, ArtistSortField, PlaylistSortField, SettingsTab, ViewMode, ViewTab};
use state::AppState;

#[test]
fn test_default_app_state() {
    let st = AppState::default();
    assert_eq!(st.active_tab, ViewTab::Tracks);
    assert_eq!(st.settings_tab, SettingsTab::Appearance);
    assert_eq!(st.volume, 1.0);
    assert!(!st.is_shuffle);
    assert!(!st.is_repeat);
    assert_eq!(st.albums_view_mode, ViewMode::Tile);
    assert_eq!(st.artists_view_mode, ViewMode::Tile);
    assert_eq!(st.playlists_view_mode, ViewMode::List);
    assert_eq!(st.album_sort_field, AlbumSortField::Title);
    assert_eq!(st.artist_sort_field, ArtistSortField::Artist);
    assert_eq!(st.playlist_sort_field, PlaylistSortField::Name);
}

#[test]
fn test_state_toml_roundtrip() {
    let custom = AppState {
        active_tab: ViewTab::Albums,
        settings_tab: SettingsTab::Library,
        volume: 0.65,
        is_shuffle: true,
        is_repeat: true,
        albums_view_mode: ViewMode::List,
        artists_view_mode: ViewMode::List,
        playlists_view_mode: ViewMode::Tile,
        album_sort_field: AlbumSortField::Artist,
        artist_sort_field: ArtistSortField::TrackCount,
        playlist_sort_field: PlaylistSortField::TrackCount,
    };

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("melody_test_state_{}.toml", std::process::id()));

    custom.save_to_path(&temp_file);
    assert!(temp_file.exists());

    let loaded = AppState::load_from_path(&temp_file);
    assert_eq!(custom, loaded);

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_missing_state_returns_defaults() {
    let non_existent = PathBuf::from("/non/existent/path/melody_state.toml");
    let loaded = AppState::load_from_path(&non_existent);
    assert_eq!(loaded, AppState::default());
}

#[test]
fn test_state_deserializes_from_legacy_config() {
    let legacy_config_toml = r#"
active_tab = "Playlists"
settings_tab = "Scrobbling"
volume = 0.42
is_shuffle = true
is_repeat = false
albums_view_mode = "List"
artists_view_mode = "Tile"
playlists_view_mode = "Tile"
album_sort_field = "Year"
artist_sort_field = "AlbumCount"
playlist_sort_field = "DateAdded"
"#;
    let state: AppState = toml::from_str(legacy_config_toml)
        .expect("Failed to deserialize AppState from legacy config");
    assert_eq!(state.active_tab, ViewTab::Playlists);
    assert_eq!(state.settings_tab, SettingsTab::Scrobbling);
    assert_eq!(state.volume, 0.42);
    assert!(state.is_shuffle);
    assert!(!state.is_repeat);
    assert_eq!(state.albums_view_mode, ViewMode::List);
    assert_eq!(state.artists_view_mode, ViewMode::Tile);
    assert_eq!(state.playlists_view_mode, ViewMode::Tile);
    assert_eq!(state.album_sort_field, AlbumSortField::Year);
    assert_eq!(state.artist_sort_field, ArtistSortField::AlbumCount);
    assert_eq!(state.playlist_sort_field, PlaylistSortField::DateAdded);
}
