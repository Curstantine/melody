#![allow(dead_code)]

#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/views/settings.rs"]
mod settings;
#[path = "../src/theme/mod.rs"]
mod theme;

use std::path::Path;

use message::{SettingsTab, ViewMode, ViewTab};
use theme::ThemeConfig;

#[test]
fn test_settings_tab_variants_and_defaults() {
    let default_tab = SettingsTab::default();
    assert_eq!(default_tab, SettingsTab::Appearance);

    let tabs = [
        SettingsTab::Appearance,
        SettingsTab::Library,
        SettingsTab::Scrobbling,
    ];
    assert_ne!(tabs[0], tabs[1]);
    assert_ne!(tabs[1], tabs[2]);
    assert_ne!(tabs[0], tabs[2]);
}

#[test]
fn test_settings_views_render_without_panicking() {
    let theme_cfg = ThemeConfig::default();
    let folders = vec!["/music/ambient".to_string(), "/music/jazz".to_string()];
    let available_fonts = vec!["Default".to_string(), "Noto Sans CJK JP".to_string()];
    let db_path = Path::new("/tmp/test_library.db");

    for tab in [
        SettingsTab::Appearance,
        SettingsTab::Library,
        SettingsTab::Scrobbling,
    ] {
        let settings_view = settings::SettingsView {
            active_tab: tab,
            available_fonts: available_fonts.clone(),
            custom_font_input: String::new(),
            library_folders: folders.clone(),
            scrobble_test_status: None,
        };

        let ctx = settings::SettingsContext {
            tab_order: &[
                ViewTab::Tracks,
                ViewTab::Lyrics,
                ViewTab::Albums,
                ViewTab::Artists,
            ],
            theme_names: vec!["Default".to_string(), "Nord".to_string()],
            active_theme_name: "Default",
            library_view_mode: ViewMode::List,
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            page_size: 50,
            primary_font: Some("Noto Sans CJK JP"),
            total_tracks: 120,
            total_albums: 15,
            total_artists: 8,
            db_path,
            config_path: Path::new("/tmp/test_config.toml"),
            db_size_bytes: 2048576,
            cache_file_count: 42,
            cache_size_bytes: 524288,
            is_scanning: false,

            listenbrainz_enabled: true,
            listenbrainz_token: "test_token",
            listenbrainz_api_url: "https://api.listenbrainz.org/1/submit-listens",
            listenbrainz_status: "Ready".to_string(),
            listenbrainz_scrobbles: 12,

            lastfm_enabled: true,
            lastfm_api_key: "test_api_key",
            lastfm_api_secret: "test_secret",
            lastfm_session_key: "test_sk",
            lastfm_api_url: "https://ws.audioscrobbler.com/2.0/",
            lastfm_status: "Ready".to_string(),
            lastfm_scrobbles: 8,

            submit_now_playing: true,
            scrobble_percent: 50,
        };

        // Render each view and verify it builds valid Iced elements without panic
        let _element = settings_view.view(&theme_cfg, ctx);
    }
}
