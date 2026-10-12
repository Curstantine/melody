#![allow(dead_code)]

use std::path::PathBuf;

#[path = "../src/config.rs"]
mod config;
#[path = "../src/message.rs"]
mod message;

use config::AppConfig;
use message::{AlbumSortField, ArtistSortField, SettingsTab, ViewMode, ViewTab};

#[test]
fn test_default_app_config() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.active_tab, ViewTab::Tracks);
    assert_eq!(cfg.settings_tab, SettingsTab::Appearance);
    assert_eq!(cfg.theme_name, "Default");
    assert_eq!(cfg.primary_font, None);
    assert_eq!(cfg.library_view_mode, ViewMode::List);
    assert_eq!(cfg.albums_view_mode, ViewMode::Tile);
    assert_eq!(cfg.artists_view_mode, ViewMode::Tile);
    assert_eq!(cfg.page_size, 50);
    assert_eq!(cfg.album_sort_field, AlbumSortField::Title);
    assert_eq!(cfg.artist_sort_field, ArtistSortField::Artist);
    assert_eq!(cfg.volume, 1.0);
    assert!(!cfg.is_shuffle);
    assert!(!cfg.is_repeat);
    assert!(!cfg.scrobble.listenbrainz_enabled);
    assert!(!cfg.scrobble.lastfm_enabled);
    assert_eq!(cfg.scrobble.scrobble_percent, 50);
}

#[test]
fn test_config_toml_roundtrip() {
    let custom = AppConfig {
        active_tab: ViewTab::Albums,
        settings_tab: SettingsTab::Library,
        theme_name: "Nord".to_string(),
        primary_font: Some("Inter".to_string()),
        library_view_mode: ViewMode::Tile,
        albums_view_mode: ViewMode::List,
        artists_view_mode: ViewMode::List,
        page_size: 100,
        album_sort_field: AlbumSortField::Artist,
        artist_sort_field: ArtistSortField::TrackCount,
        volume: 0.65,
        is_shuffle: true,
        is_repeat: true,
        scrobble: config::ScrobbleConfig {
            listenbrainz_enabled: true,
            listenbrainz_token: "test_token".to_string(),
            listenbrainz_api_url: "https://custom.listenbrainz/api".to_string(),
            lastfm_enabled: true,
            lastfm_api_key: "api_key".to_string(),
            lastfm_api_secret: "secret".to_string(),
            lastfm_session_key: "sk".to_string(),
            lastfm_api_url: "https://custom.lastfm/api".to_string(),
            submit_now_playing: false,
            scrobble_percent: 75,
        },
    };

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("vibecore_test_config_{}.toml", std::process::id()));

    custom.save_to_path(&temp_file);
    assert!(temp_file.exists());

    let loaded = AppConfig::load_from_path(&temp_file);
    assert_eq!(custom, loaded);

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_missing_config_returns_defaults() {
    let non_existent = PathBuf::from("/non/existent/path/vibecore_config.toml");
    let loaded = AppConfig::load_from_path(&non_existent);
    assert_eq!(loaded, AppConfig::default());
}
