use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::message::{AlbumSortField, SettingsTab, ViewMode, ViewTab};

pub const DEFAULT_PAGE_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default)]
    pub active_tab: ViewTab,
    #[serde(default)]
    pub settings_tab: SettingsTab,

    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default)]
    pub primary_font: Option<String>,

    #[serde(default)]
    pub library_view_mode: ViewMode,
    #[serde(default)]
    pub albums_view_mode: ViewMode,
    #[serde(default)]
    pub artists_view_mode: ViewMode,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub album_sort_field: AlbumSortField,

    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub is_shuffle: bool,
    #[serde(default)]
    pub is_repeat: bool,
}

fn default_theme_name() -> String {
    "Default".to_string()
}

fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

fn default_volume() -> f32 {
    1.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_tab: ViewTab::Tracks,
            settings_tab: SettingsTab::Appearance,
            theme_name: default_theme_name(),
            primary_font: None,
            library_view_mode: ViewMode::List,
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            page_size: default_page_size(),
            album_sort_field: AlbumSortField::Title,
            volume: default_volume(),
            is_shuffle: false,
            is_repeat: false,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("org", "vibecore", "vibecore") {
            let config_dir = proj_dirs.config_dir();
            let _ = fs::create_dir_all(config_dir);
            config_dir.join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Self {
        if !path.exists() {
            info!("No config file found at {:?}, using default settings", path);
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(config) => {
                    info!("Successfully loaded settings from {:?}", path);
                    config
                }
                Err(err) => {
                    error!(
                        "Failed to parse config file {:?}: {}. Falling back to defaults",
                        path, err
                    );
                    Self::default()
                }
            },
            Err(err) => {
                error!(
                    "Failed to read config file {:?}: {}. Falling back to defaults",
                    path, err
                );
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        self.save_to_path(&path);
    }

    pub fn save_to_path(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(err) = fs::write(path, content) {
                    error!("Failed to write settings to {:?}: {}", path, err);
                } else {
                    info!("Saved settings to {:?}", path);
                }
            }
            Err(err) => {
                error!("Failed to serialize config to TOML: {}", err);
            }
        }
    }
}
