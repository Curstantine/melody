pub mod config;
pub mod navigation;
pub mod state;

pub use config::{
    default_tab_order, sanitize_tab_order, AppConfig, ScrobbleConfig, DEFAULT_ALBUM_TILE_WIDTH,
    DEFAULT_ARTIST_TILE_WIDTH, DEFAULT_LAYOUT_NAME, DEFAULT_NOW_PLAYING_COVER_SIZE,
    DEFAULT_PAGE_SIZE, DEFAULT_TILE_WIDTH,
};
pub use navigation::{PlaylistSection, SettingsTab, ViewTab};
pub use state::AppState;

#[cfg(test)]
mod tests {
    use super::*;
    use model::{AlbumSortField, ArtistSortField, PlaylistSortField, ViewMode};
    use std::path::PathBuf;

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
        assert_eq!(cfg.layout_name, DEFAULT_LAYOUT_NAME);
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
            resampling: audio::ResamplingConfig::default(),
            watch_library: false,
            scrobble: ScrobbleConfig {
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
        let sanitized = sanitize_tab_order(&input);
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
        let sanitized_empty = sanitize_tab_order(&empty);
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

    #[test]
    fn test_config_resampling_toml_roundtrip() {
        let cfg = AppConfig {
            resampling: audio::ResamplingConfig {
                filter: audio::ResamplingFilter::BlackmanNuttall,
                quality: audio::ResamplingQuality::Ultra,
                linear_interp: false,
            },
            ..Default::default()
        };

        let toml_str = toml::to_string(&cfg).expect("Serialize config");
        let deserialized: AppConfig = toml::from_str(&toml_str).expect("Deserialize config");
        assert_eq!(cfg.resampling, deserialized.resampling);
        assert_eq!(
            deserialized.resampling.filter,
            audio::ResamplingFilter::BlackmanNuttall
        );
        assert_eq!(
            deserialized.resampling.quality,
            audio::ResamplingQuality::Ultra
        );
        assert!(!deserialized.resampling.linear_interp);
    }

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
        assert_eq!(st.window_size, None);
        assert_eq!(st.window_size(), None);
        assert_eq!(st.queue_id, None);
        assert_eq!(st.playing_seq, None);
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
            window_size: Some((1280.0, 800.0)),
            window_width: None,
            window_height: None,
            queue_id: Some(12),
            playing_seq: Some(5),
        };

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("melody_test_state_{}.toml", std::process::id()));

        custom.save_to_path(&temp_file);
        assert!(temp_file.exists());

        let loaded = AppState::load_from_path(&temp_file);
        assert_eq!(custom, loaded);
        assert_eq!(loaded.window_size(), Some((1280.0, 800.0)));

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
        assert_eq!(state.window_size, None);
        assert_eq!(state.queue_id, None);
        assert_eq!(state.playing_seq, None);
    }

    #[test]
    fn test_window_size_fallback_from_width_height() {
        let toml = r#"
window_width = 1440.0
window_height = 900.0
"#;
        let state: AppState =
            toml::from_str(toml).expect("Failed to parse state with width and height");
        assert_eq!(state.window_size(), Some((1440.0, 900.0)));
    }

    #[test]
    fn test_queue_state_aliases() {
        let toml = r#"
active_queue_id = 42
current_seq = 7
"#;
        let state: AppState = toml::from_str(toml).expect("Failed to parse state with aliases");
        assert_eq!(state.queue_id, Some(42));
        assert_eq!(state.playing_seq, Some(7));

        let toml2 = r#"
queue_id = 99
queue_seq = 3
"#;
        let state2: AppState = toml::from_str(toml2).expect("Failed to parse state with queue_seq");
        assert_eq!(state2.queue_id, Some(99));
        assert_eq!(state2.playing_seq, Some(3));
    }
}
