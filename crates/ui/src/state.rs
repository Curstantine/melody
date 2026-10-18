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
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(err) = fs::write(path, content) {
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
