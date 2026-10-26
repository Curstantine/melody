#![allow(dead_code)]

#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/views/settings.rs"]
mod settings;
#[path = "../src/views/status_bar.rs"]
mod status_bar;
#[path = "../src/theme/mod.rs"]
mod theme;

use message::{SettingsTab, ViewMode};
use theme::{ThemeConfig, DEFAULT_THEME_NAME};

#[test]
fn test_settings_tab_variants_and_defaults() {
    let default_tab = SettingsTab::default();
    assert_eq!(default_tab, SettingsTab::Appearance);

    let tabs = [
        SettingsTab::Appearance,
        SettingsTab::Playback,
        SettingsTab::Library,
        SettingsTab::Scrobbling,
    ];
    assert_ne!(tabs[0], tabs[1]);
    assert_ne!(tabs[1], tabs[2]);
    assert_ne!(tabs[2], tabs[3]);
    assert_ne!(tabs[0], tabs[3]);
}

#[test]
fn test_settings_views_render_without_panicking() {
    let theme_cfg = ThemeConfig::default();
    let folders = vec!["/music/ambient".to_string(), "/music/jazz".to_string()];
    let available_fonts = vec!["Default".to_string(), "Noto Sans CJK JP".to_string()];

    for tab in [
        SettingsTab::Appearance,
        SettingsTab::Playback,
        SettingsTab::Library,
        SettingsTab::Scrobbling,
    ] {
        let settings_view = settings::SettingsView {
            active_tab: tab,
            available_fonts: available_fonts.clone(),
            library_folders: folders.clone(),
            scrobble_test_status: None,
            fonts_loaded: true,
        };

        let theme_names_vec = vec![DEFAULT_THEME_NAME.to_string(), "Nord".to_string()];
        let layout_names_vec = vec!["Default".to_string(), "Sidebar".to_string()];
        let ctx = settings::SettingsContext {
            theme_names: &theme_names_vec,
            active_theme_name: DEFAULT_THEME_NAME,
            layout_names: &layout_names_vec,
            active_layout_name: "Default",
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            page_size: 50,
            album_tile_width: 170,
            artist_tile_width: 160,
            now_playing_cover_size: 220,
            primary_font: Some("Noto Sans CJK JP"),
            total_tracks: 120,
            total_albums: 15,
            total_artists: 8,
            db_size_bytes: 2048576,
            cache_file_count: 42,
            cache_size_bytes: 524288,
            is_scanning: false,

            listenbrainz_enabled: true,
            listenbrainz_token: "test_token",
            listenbrainz_api_url: "https://api.listenbrainz.org/1/submit-listens",

            lastfm_enabled: true,
            lastfm_api_key: "test_api_key",
            lastfm_api_secret: "test_secret",
            lastfm_session_key: "test_sk",
            lastfm_api_url: "https://ws.audioscrobbler.com/2.0/",

            submit_now_playing: true,
            scrobble_percent: 50,
            gapless: true,
            watch_library: true,
            background_mode: true,
            background_mode_timeout_secs: 60,
            volume: 1.0,
            is_shuffle: false,
            is_repeat: false,
        };

        // Render each view and verify it builds valid Iced elements without panic
        let _element = settings_view.view(&theme_cfg, ctx);
    }
}

#[test]
fn test_status_bar_indicators_render() {
    let theme_cfg = ThemeConfig::default();
    let ctx = status_bar::StatusBarContext {
        active_tab: message::ViewTab::Tracks,
        total_tracks: 100,
        search_query: "",
        album_count: 10,
        album_tracks: 100,
        album_duration_ms: 100000,
        artist_count: 5,
        artist_albums: 10,
        artist_tracks: 100,
        current_track: None,
        is_playing: false,
        playlist_count: 2,
        queue_count: 1,
        active_queue: None,
        is_scanning: true,
        scan_progress: Some((45, 120)),
        is_generating_thumbnails: false,
    };
    let _el = status_bar::StatusBar::view(&theme_cfg, ctx);

    let ctx_thumbs = status_bar::StatusBarContext {
        is_scanning: false,
        scan_progress: None,
        is_generating_thumbnails: true,
        ..ctx
    };
    let _el2 = status_bar::StatusBar::view(&theme_cfg, ctx_thumbs);
}

#[test]
fn test_watch_library_and_scan_new_messages() {
    let msg1 = message::Message::ScanLibraryForNewFiles;
    let msg2 = message::Message::ToggleWatchLibrary(true);
    let msg3 = message::Message::ToggleWatchLibrary(false);

    assert_eq!(format!("{:?}", msg1), "ScanLibraryForNewFiles");
    assert_eq!(format!("{:?}", msg2), "ToggleWatchLibrary(true)");
    assert_eq!(format!("{:?}", msg3), "ToggleWatchLibrary(false)");
}

#[test]
fn test_settings_lazy_fonts() {
    let mut view = settings::SettingsView::new(SettingsTab::Appearance, Vec::new());
    assert!(!view.fonts_loaded);
    assert_eq!(view.available_fonts, vec!["Default".to_string()]);

    view.set_available_fonts(vec!["Inter".to_string(), "Roboto".to_string()]);
    assert!(view.fonts_loaded);
    assert_eq!(
        view.available_fonts,
        vec![
            "Default".to_string(),
            "Inter".to_string(),
            "Roboto".to_string()
        ]
    );
}
