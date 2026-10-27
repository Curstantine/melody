use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

use crate::message::{
    AlbumSortField, ArtistSortField, PlaylistSortField, SettingsTab, ViewMode, ViewTab,
};

fn default_volume() -> f32 {
    1.0
}

/// Persistent runtime state of the application and player.
///
/// Unlike [`crate::config::AppConfig`], which holds user preferences and system settings,
/// [`AppState`] tracks transient session state such as active view tabs, volume, shuffle/repeat
/// flags, and catalog sort and view modes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppState {
    #[serde(default)]
    pub active_tab: ViewTab,
    #[serde(default)]
    pub settings_tab: SettingsTab,

    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub is_shuffle: bool,
    #[serde(default)]
    pub is_repeat: bool,

    #[serde(default)]
    pub albums_view_mode: ViewMode,
    #[serde(default)]
    pub artists_view_mode: ViewMode,
    #[serde(default)]
    pub playlists_view_mode: ViewMode,
    #[serde(default)]
    pub album_sort_field: AlbumSortField,
    #[serde(default)]
    pub artist_sort_field: ArtistSortField,
    #[serde(default)]
    pub playlist_sort_field: PlaylistSortField,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_size: Option<(f32, f32)>,
    #[serde(default, skip_serializing)]
    pub window_width: Option<f32>,
    #[serde(default, skip_serializing)]
    pub window_height: Option<f32>,

    #[serde(
        default,
        alias = "active_queue_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub queue_id: Option<u64>,
    #[serde(
        default,
        alias = "queue_seq",
        alias = "current_seq",
        alias = "active_queue_seq",
        skip_serializing_if = "Option::is_none"
    )]
    pub playing_seq: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_tab: ViewTab::Tracks,
            settings_tab: SettingsTab::Appearance,
            volume: default_volume(),
            is_shuffle: false,
            is_repeat: false,
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            playlists_view_mode: ViewMode::List,
            album_sort_field: AlbumSortField::Title,
            artist_sort_field: ArtistSortField::Artist,
            playlist_sort_field: PlaylistSortField::Name,
            window_size: None,
            window_width: None,
            window_height: None,
            queue_id: None,
            playing_seq: None,
        }
    }
}

impl AppState {
    pub fn window_size(&self) -> Option<(f32, f32)> {
        if let Some(size) = self.window_size {
            Some(size)
        } else if let (Some(w), Some(h)) = (self.window_width, self.window_height) {
            Some((w, h))
        } else {
            None
        }
    }
    pub fn state_path() -> PathBuf {
        core_engine::state_file()
    }

    pub fn load() -> Self {
        let path = Self::state_path();
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            // Backward-compatibility: if state.toml does not exist, attempt to load legacy
            // state properties previously stored in config.toml before falling back to defaults.
            let legacy_config = core_engine::config_file();
            if legacy_config.exists() {
                if let Ok(content) = fs::read_to_string(&legacy_config) {
                    if let Ok(state) = toml::from_str::<Self>(&content) {
                        info!(
                            "Migrated runtime state from legacy config at {:?}",
                            legacy_config
                        );
                        return state;
                    }
                }
            }
            info!("No state file found at {:?}, using default state", path);
            Self::default()
        }
    }

    pub fn load_from_path(path: &Path) -> Self {
        if !path.exists() {
            info!("No state file found at {:?}, using default state", path);
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(state) => {
                    info!("Successfully loaded state from {:?}", path);
                    state
                }
                Err(err) => {
                    error!(
                        "Failed to parse state file {:?}: {}. Falling back to defaults",
                        path, err
                    );
                    Self::default()
                }
            },
            Err(err) => {
                error!(
                    "Failed to read state file {:?}: {}. Falling back to defaults",
                    path, err
                );
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        let path = Self::state_path();
        self.save_to_path(&path);
    }

    pub fn save_to_path(&self, path: &Path) {
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(err) = core_engine::atomic_write(path, &content) {
                    error!("Failed to write state to {:?}: {}", path, err);
                } else {
                    info!("Saved state to {:?}", path);
                }
            }
            Err(err) => {
                error!("Failed to serialize state to TOML: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{
        AlbumSortField, ArtistSortField, PlaylistSortField, SettingsTab, ViewMode, ViewTab,
    };
    use std::path::PathBuf;

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
