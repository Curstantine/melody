#![allow(dead_code)]

use std::path::PathBuf;

#[path = "../src/config.rs"]
mod config;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/theme/mod.rs"]
mod theme;

use config::AppConfig;
use message::ViewTab;

#[test]
fn test_default_app_config() {
    let cfg = AppConfig::default();
    assert_eq!(
        cfg.tab_order,
        vec![
            ViewTab::Tracks,
            ViewTab::Lyrics,
            ViewTab::Albums,
            ViewTab::Artists,
            ViewTab::Playlists
        ]
    );
    assert_eq!(cfg.theme_name, theme::DEFAULT_THEME_NAME);
    assert_eq!(cfg.layout_name, config::DEFAULT_LAYOUT_NAME);
    assert_eq!(cfg.primary_font, None);
    assert_eq!(cfg.page_size, 50);
    assert_eq!(cfg.tile_width, 140);
    assert_eq!(cfg.album_tile_width, 140);
    assert_eq!(cfg.artist_tile_width, 140);
    assert_eq!(cfg.now_playing_cover_size, 260);
    assert!(cfg.gapless);
    assert!(cfg.watch_library);
    assert!(!cfg.scrobble.listenbrainz_enabled);
    assert!(!cfg.scrobble.lastfm_enabled);
    assert_eq!(cfg.scrobble.scrobble_percent, 50);
}

#[test]
fn test_config_toml_roundtrip() {
    let custom = AppConfig {
        tab_order: vec![
            ViewTab::Albums,
            ViewTab::Tracks,
            ViewTab::Artists,
            ViewTab::Lyrics,
            ViewTab::Playlists,
        ],
        theme_name: "Nord".to_string(),
        layout_name: "Sidebar".to_string(),
        primary_font: Some("Inter".to_string()),
        page_size: 100,
        tile_width: 190,
        album_tile_width: 190,
        artist_tile_width: 180,
        now_playing_cover_size: 260,
        gapless: false,
        watch_library: false,
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
    let temp_file = temp_dir.join(format!("melody_test_config_{}.toml", std::process::id()));

    custom.save_to_path(&temp_file);
    assert!(temp_file.exists());

    let loaded = AppConfig::load_from_path(&temp_file);
    assert_eq!(custom, loaded);

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_config_ignores_legacy_state_fields() {
    let toml_with_state = r#"
theme_name = "Nord"
page_size = 75
active_tab = "Albums"
volume = 0.5
is_shuffle = true
is_repeat = true
"#;
    let cfg: AppConfig =
        toml::from_str(toml_with_state).expect("Failed to parse config with legacy state");
    assert_eq!(cfg.theme_name, "Nord");
    assert_eq!(cfg.page_size, 75);
}

#[test]
fn test_missing_config_returns_defaults() {
    let non_existent = PathBuf::from("/non/existent/path/melody_config.toml");
    let loaded = AppConfig::load_from_path(&non_existent);
    assert_eq!(loaded, AppConfig::default());
}

#[test]
fn test_tab_order_sanitization() {
    // Missing tabs and duplicates
    let input = vec![ViewTab::Artists, ViewTab::Tracks, ViewTab::Artists];
    let sanitized = config::sanitize_tab_order(&input);
    assert_eq!(
        sanitized,
        vec![
            ViewTab::Artists,
            ViewTab::Tracks,
            ViewTab::Lyrics,
            ViewTab::Albums,
            ViewTab::Playlists
        ]
    );

    // Empty list restores all defaults
    let empty: Vec<ViewTab> = vec![];
    let sanitized_empty = config::sanitize_tab_order(&empty);
    assert_eq!(
        sanitized_empty,
        vec![
            ViewTab::Tracks,
            ViewTab::Lyrics,
            ViewTab::Albums,
            ViewTab::Artists,
            ViewTab::Playlists
        ]
    );
}
