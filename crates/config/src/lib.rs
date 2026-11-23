pub mod config;
pub mod navigation;
pub mod state;

pub use config::{AppConfig, DEFAULT_ALBUM_TILE_WIDTH, DEFAULT_ARTIST_TILE_WIDTH};
pub use melody_extension_sdk::ExtensionConfigEntry;
pub use navigation::{PlaylistSection, SettingsTab, ViewTab};
pub use state::{AppState, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::sanitize_tab_order;
    use melody_extension_sdk::SettingValue;
    use model::{AlbumSortField, ArtistSortField, PlaylistSortField, ViewMode};
    use std::collections::BTreeMap;
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
        assert_eq!(cfg.layout_name, layout::DEFAULT_LAYOUT_NAME);
        assert_eq!(cfg.fonts.ui, None);
        assert_eq!(cfg.page_size, 50);
        assert_eq!(cfg.album_tile_width, 140);
        assert_eq!(cfg.artist_tile_width, 140);
        assert_eq!(cfg.fonts.lyrics, None);
        assert_eq!(cfg.fonts.ui_size, 12);
        assert_eq!(cfg.fonts.lyrics_size, 15);
        assert!(cfg.gapless);
        assert!(cfg.watch_library);
        assert!(cfg.extensions.is_empty());
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let mut extensions = BTreeMap::new();
        extensions.insert(
            "melody.listenbrainz".to_string(),
            ExtensionConfigEntry {
                enabled: true,
                settings: BTreeMap::from([
                    (
                        "user_token".to_string(),
                        SettingValue::String("test_token".to_string()),
                    ),
                    (
                        "api_url".to_string(),
                        SettingValue::String("https://custom.listenbrainz/api".to_string()),
                    ),
                    ("submit_now_playing".to_string(), SettingValue::Bool(false)),
                    ("scrobble_percent".to_string(), SettingValue::Integer(75)),
                ]),
            },
        );

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
            custom_titlebar: true,
            minimize_to_tray: true,
            page_size: 100,
            album_tile_width: 190,
            artist_tile_width: 180,
            fonts: font::FontConfig::new(
                Some("Inter".to_string()),
                Some("Noto Serif".to_string()),
                14,
                18,
            ),
            gapless: false,
            resampling: audio::ResamplingConfig::default(),
            output_device: Some("USB DAC".to_string()),
            watch_library: false,
            extensions,
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
        assert_eq!(st.repeat_mode, model::RepeatMode::Off);
        assert_eq!(st.albums_view_mode, ViewMode::Tile);
        assert_eq!(st.artists_view_mode, ViewMode::Tile);
        assert_eq!(st.playlists_view_mode, ViewMode::List);
        assert_eq!(st.album_sort_field, AlbumSortField::Title);
        assert_eq!(st.artist_sort_field, ArtistSortField::Artist);
        assert_eq!(st.playlist_sort_field, PlaylistSortField::Name);
        assert_eq!(st.window_width, WINDOW_MIN_WIDTH);
        assert_eq!(st.window_height, WINDOW_MIN_HEIGHT);
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
            repeat_mode: model::RepeatMode::All,
            albums_view_mode: ViewMode::List,
            artists_view_mode: ViewMode::List,
            playlists_view_mode: ViewMode::Tile,
            album_sort_field: AlbumSortField::Artist,
            artist_sort_field: ArtistSortField::TrackCount,
            playlist_sort_field: PlaylistSortField::TrackCount,
            window_width: 1280.0,
            window_height: 800.0,
            queue_id: Some(12),
            playing_seq: Some(5),
        };

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("melody_test_state_{}.toml", std::process::id()));

        custom.save_to_path(&temp_file);
        assert!(temp_file.exists());

        let loaded = AppState::load_from_path(&temp_file);
        assert_eq!(custom, loaded);
        assert_eq!(loaded.window_width, 1280.0);
        assert_eq!(loaded.window_height, 800.0);

        let _ = std::fs::remove_file(temp_file);
    }

    #[test]
    fn test_missing_state_returns_defaults() {
        let non_existent = PathBuf::from("/non/existent/path/melody_state.toml");
        let loaded = AppState::load_from_path(&non_existent);
        assert_eq!(loaded, AppState::default());
    }

    #[test]
    fn test_window_dimensions_deserialize() {
        let toml = r#"
window_width = 1440.0
window_height = 900.0
"#;
        let state: AppState =
            toml::from_str(toml).expect("Failed to parse state with width and height");
        assert_eq!(state.window_width, 1440.0);
        assert_eq!(state.window_height, 900.0);
    }
}
