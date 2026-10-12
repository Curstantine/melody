use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::message::{AlbumSortField, ArtistSortField, SettingsTab, ViewMode, ViewTab};

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
    #[serde(default)]
    pub artist_sort_field: ArtistSortField,

    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub is_shuffle: bool,
    #[serde(default)]
    pub is_repeat: bool,

    #[serde(default)]
    pub scrobble: ScrobbleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScrobbleConfig {
    #[serde(default)]
    pub listenbrainz_enabled: bool,
    #[serde(default)]
    pub listenbrainz_token: String,
    #[serde(default = "default_listenbrainz_url")]
    pub listenbrainz_api_url: String,

    #[serde(default)]
    pub lastfm_enabled: bool,
    #[serde(default)]
    pub lastfm_api_key: String,
    #[serde(default)]
    pub lastfm_api_secret: String,
    #[serde(default)]
    pub lastfm_session_key: String,
    #[serde(default = "default_lastfm_url")]
    pub lastfm_api_url: String,

    #[serde(default = "default_true")]
    pub submit_now_playing: bool,
    #[serde(default = "default_scrobble_percent")]
    pub scrobble_percent: u32,
}

fn default_listenbrainz_url() -> String {
    player_plugins::scrobbler::listenbrainz::DEFAULT_LISTENBRAINZ_API_URL.to_string()
}

fn default_lastfm_url() -> String {
    player_plugins::scrobbler::lastfm::DEFAULT_LASTFM_API_URL.to_string()
}

fn default_true() -> bool {
    true
}

fn default_scrobble_percent() -> u32 {
    50
}

impl Default for ScrobbleConfig {
    fn default() -> Self {
        Self {
            listenbrainz_enabled: false,
            listenbrainz_token: String::new(),
            listenbrainz_api_url: default_listenbrainz_url(),
            lastfm_enabled: false,
            lastfm_api_key: String::new(),
            lastfm_api_secret: String::new(),
            lastfm_session_key: String::new(),
            lastfm_api_url: default_lastfm_url(),
            submit_now_playing: true,
            scrobble_percent: 50,
        }
    }
}

impl ScrobbleConfig {
    pub fn to_listenbrainz_config(&self) -> player_plugins::ListenBrainzConfig {
        player_plugins::ListenBrainzConfig {
            enabled: self.listenbrainz_enabled,
            user_token: self.listenbrainz_token.clone(),
            api_url: self.listenbrainz_api_url.clone(),
            submit_now_playing: self.submit_now_playing,
            scrobble_percent: self.scrobble_percent,
        }
    }

    pub fn to_lastfm_config(&self) -> player_plugins::LastFmConfig {
        player_plugins::LastFmConfig {
            enabled: self.lastfm_enabled,
            api_key: self.lastfm_api_key.clone(),
            api_secret: self.lastfm_api_secret.clone(),
            session_key: self.lastfm_session_key.clone(),
            api_url: self.lastfm_api_url.clone(),
            submit_now_playing: self.submit_now_playing,
            scrobble_percent: self.scrobble_percent,
        }
    }
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
            artist_sort_field: ArtistSortField::Artist,
            volume: default_volume(),
            is_shuffle: false,
            is_repeat: false,
            scrobble: ScrobbleConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        player_core::config_file()
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
