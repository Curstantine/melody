use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

use crate::message::ViewTab;

pub const DEFAULT_PAGE_SIZE: usize = 50;
pub const DEFAULT_TILE_WIDTH: u32 = 140;
pub const DEFAULT_ALBUM_TILE_WIDTH: u32 = 140;
pub const DEFAULT_ARTIST_TILE_WIDTH: u32 = 140;
pub const DEFAULT_NOW_PLAYING_COVER_SIZE: u32 = 260;

pub fn default_tab_order() -> Vec<ViewTab> {
    vec![
        ViewTab::Tracks,
        ViewTab::Lyrics,
        ViewTab::Albums,
        ViewTab::Artists,
        ViewTab::Playlists,
    ]
}

pub fn sanitize_tab_order(order: &[ViewTab]) -> Vec<ViewTab> {
    let standard = default_tab_order();
    let mut sanitized = Vec::with_capacity(standard.len());
    for &tab in order {
        if standard.contains(&tab) && !sanitized.contains(&tab) {
            sanitized.push(tab);
        }
    }
    for tab in standard {
        if !sanitized.contains(&tab) {
            sanitized.push(tab);
        }
    }
    sanitized
}

/// Persistent user configuration and preferences.
///
/// Unlike [`crate::state::AppState`], which stores ephemeral session and player state,
/// [`AppConfig`] stores user preferences such as theme, font selection, tab ordering,
/// tile sizes, and scrobbling credentials.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default = "default_tab_order")]
    pub tab_order: Vec<ViewTab>,

    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default = "default_layout_name")]
    pub layout_name: String,
    #[serde(default)]
    pub primary_font: Option<String>,

    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default = "default_tile_width")]
    pub tile_width: u32,
    #[serde(default = "default_album_tile_width")]
    pub album_tile_width: u32,
    #[serde(default = "default_artist_tile_width")]
    pub artist_tile_width: u32,
    #[serde(default = "default_now_playing_cover_size")]
    pub now_playing_cover_size: u32,

    #[serde(default = "default_true")]
    pub gapless: bool,

    #[serde(default = "default_true")]
    pub background_mode: bool,

    #[serde(default = "default_background_mode_timeout_secs")]
    pub background_mode_timeout_secs: u64,

    #[serde(default = "default_true")]
    pub watch_library: bool,

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
    plugins::scrobbler::listenbrainz::DEFAULT_LISTENBRAINZ_API_URL.to_string()
}

fn default_lastfm_url() -> String {
    plugins::scrobbler::lastfm::DEFAULT_LASTFM_API_URL.to_string()
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
    pub fn to_listenbrainz_config(&self) -> plugins::ListenBrainzConfig {
        plugins::ListenBrainzConfig {
            enabled: self.listenbrainz_enabled,
            user_token: self.listenbrainz_token.clone(),
            api_url: self.listenbrainz_api_url.clone(),
            submit_now_playing: self.submit_now_playing,
            scrobble_percent: self.scrobble_percent,
        }
    }

    pub fn to_lastfm_config(&self) -> plugins::LastFmConfig {
        plugins::LastFmConfig {
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

pub const DEFAULT_LAYOUT_NAME: &str = "Default";

fn default_theme_name() -> String {
    crate::theme::DEFAULT_THEME_NAME.to_string()
}

fn default_layout_name() -> String {
    DEFAULT_LAYOUT_NAME.to_string()
}

pub const DEFAULT_BACKGROUND_MODE_TIMEOUT_SECS: u64 = 60;

fn default_background_mode_timeout_secs() -> u64 {
    DEFAULT_BACKGROUND_MODE_TIMEOUT_SECS
}

fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

fn default_tile_width() -> u32 {
    DEFAULT_TILE_WIDTH
}

fn default_album_tile_width() -> u32 {
    DEFAULT_ALBUM_TILE_WIDTH
}

fn default_artist_tile_width() -> u32 {
    DEFAULT_ARTIST_TILE_WIDTH
}

fn default_now_playing_cover_size() -> u32 {
    DEFAULT_NOW_PLAYING_COVER_SIZE
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tab_order: default_tab_order(),
            theme_name: default_theme_name(),
            layout_name: default_layout_name(),
            primary_font: None,
            page_size: default_page_size(),
            tile_width: default_tile_width(),
            album_tile_width: default_album_tile_width(),
            artist_tile_width: default_artist_tile_width(),
            now_playing_cover_size: default_now_playing_cover_size(),
            gapless: true,
            background_mode: true,
            background_mode_timeout_secs: DEFAULT_BACKGROUND_MODE_TIMEOUT_SECS,
            watch_library: true,
            scrobble: ScrobbleConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        core_engine::config_file()
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            return Self::load_from_path(&path);
        }

        // Check system-wide XDG configuration directories as fallback
        if let Ok(dirs) = std::env::var("XDG_CONFIG_DIRS") {
            for dir in dirs.split(':') {
                let sys_cfg = Path::new(dir).join("melody").join("config.toml");
                if sys_cfg.exists() {
                    info!("Found system configuration at {:?}", sys_cfg);
                    return Self::load_from_path(&sys_cfg);
                }
            }
        }
        for fallback in &[
            Path::new("/etc/xdg/melody/config.toml"),
            Path::new("/etc/melody/config.toml"),
        ] {
            if fallback.exists() {
                info!("Found system configuration at {:?}", fallback);
                return Self::load_from_path(fallback);
            }
        }

        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Self {
        if !path.exists() {
            info!("No config file found at {:?}, using default settings", path);
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(mut config) => {
                    info!("Successfully loaded settings from {:?}", path);
                    config.tab_order = sanitize_tab_order(&config.tab_order);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ViewTab;
    use crate::theme;
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
        assert!(cfg.background_mode);
        assert_eq!(cfg.background_mode_timeout_secs, 60);
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
            background_mode: false,
            background_mode_timeout_secs: 120,
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
}
