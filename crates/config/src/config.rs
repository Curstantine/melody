use std::{collections::BTreeMap, fs, path::Path};

use melody_extension_sdk::{ExtensionConfigEntry, SettingValue};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::navigation::ViewTab;

const DEFAULT_PAGE_SIZE: usize = 50;
pub const DEFAULT_ALBUM_TILE_WIDTH: u32 = 140;
pub const DEFAULT_ARTIST_TILE_WIDTH: u32 = 140;

pub(crate) fn default_tab_order() -> Vec<ViewTab> {
    vec![
        ViewTab::Tracks,
        ViewTab::Lyrics,
        ViewTab::Albums,
        ViewTab::Artists,
        ViewTab::Playlists,
    ]
}

pub(crate) fn sanitize_tab_order(order: &[ViewTab]) -> Vec<ViewTab> {
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
/// tile sizes, and per-extension settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    #[serde(default = "default_tab_order")]
    pub tab_order: Vec<ViewTab>,

    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default = "default_layout_name")]
    pub layout_name: String,
    #[serde(default)]
    pub custom_titlebar: bool,

    #[serde(default)]
    pub minimize_to_tray: bool,

    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default = "default_album_tile_width")]
    pub album_tile_width: u32,
    #[serde(default = "default_artist_tile_width")]
    pub artist_tile_width: u32,
    #[serde(flatten)]
    pub fonts: font::FontConfig,

    #[serde(default = "default_true")]
    pub gapless: bool,

    #[serde(default)]
    pub resampling: audio::ResamplingConfig,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_device: Option<String>,

    #[serde(default = "default_true")]
    pub watch_library: bool,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, ExtensionConfigEntry>,
}

fn default_true() -> bool {
    true
}

fn default_theme_name() -> String {
    theme::DEFAULT_THEME_NAME.to_string()
}

fn default_layout_name() -> String {
    layout::DEFAULT_LAYOUT_NAME.to_string()
}

fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

fn default_album_tile_width() -> u32 {
    DEFAULT_ALBUM_TILE_WIDTH
}

fn default_artist_tile_width() -> u32 {
    DEFAULT_ARTIST_TILE_WIDTH
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tab_order: default_tab_order(),
            theme_name: default_theme_name(),
            layout_name: default_layout_name(),
            custom_titlebar: false,
            minimize_to_tray: false,
            page_size: default_page_size(),
            album_tile_width: default_album_tile_width(),
            artist_tile_width: default_artist_tile_width(),
            fonts: font::FontConfig::default(),
            gapless: true,
            resampling: audio::ResamplingConfig::default(),
            output_device: None,
            watch_library: true,
            extensions: BTreeMap::new(),
        }
    }
}

/// Migrates legacy `[scrobble]` TOML fields into `extensions["melody.listenbrainz"]`
/// and `extensions["melody.lastfm"]` when `extensions` is empty.
fn migrate_legacy_scrobble(content: &str, extensions: &mut BTreeMap<String, ExtensionConfigEntry>) {
    if !extensions.is_empty() {
        return;
    }
    let Ok(root) = toml::from_str::<toml::Table>(content) else {
        return;
    };
    let Some(scrobble) = root.get("scrobble").and_then(|v| v.as_table()) else {
        return;
    };

    let submit_now_playing = scrobble
        .get("submit_now_playing")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let scrobble_percent = scrobble
        .get("scrobble_percent")
        .and_then(|v| v.as_integer())
        .unwrap_or(50);

    let lb_enabled = scrobble
        .get("listenbrainz_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let lb_token = scrobble
        .get("listenbrainz_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let lb_url = scrobble
        .get("listenbrainz_api_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if lb_enabled || !lb_token.is_empty() || !lb_url.is_empty() {
        let mut settings = BTreeMap::new();
        if !lb_token.is_empty() {
            settings.insert(
                "user_token".to_string(),
                SettingValue::String(lb_token.to_string()),
            );
        }
        if !lb_url.is_empty() {
            settings.insert(
                "api_url".to_string(),
                SettingValue::String(lb_url.to_string()),
            );
        }
        settings.insert(
            "submit_now_playing".to_string(),
            SettingValue::Bool(submit_now_playing),
        );
        settings.insert(
            "scrobble_percent".to_string(),
            SettingValue::Integer(scrobble_percent),
        );
        extensions.insert(
            "melody.listenbrainz".to_string(),
            ExtensionConfigEntry {
                enabled: lb_enabled,
                settings,
            },
        );
    }

    let lfm_enabled = scrobble
        .get("lastfm_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let lfm_key = scrobble
        .get("lastfm_api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let lfm_secret = scrobble
        .get("lastfm_api_secret")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let lfm_sk = scrobble
        .get("lastfm_session_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let lfm_url = scrobble
        .get("lastfm_api_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if lfm_enabled
        || !lfm_key.is_empty()
        || !lfm_secret.is_empty()
        || !lfm_sk.is_empty()
        || !lfm_url.is_empty()
    {
        let mut settings = BTreeMap::new();
        if !lfm_key.is_empty() {
            settings.insert(
                "api_key".to_string(),
                SettingValue::String(lfm_key.to_string()),
            );
        }
        if !lfm_secret.is_empty() {
            settings.insert(
                "api_secret".to_string(),
                SettingValue::String(lfm_secret.to_string()),
            );
        }
        if !lfm_sk.is_empty() {
            settings.insert(
                "session_key".to_string(),
                SettingValue::String(lfm_sk.to_string()),
            );
        }
        if !lfm_url.is_empty() {
            settings.insert(
                "api_url".to_string(),
                SettingValue::String(lfm_url.to_string()),
            );
        }
        settings.insert(
            "submit_now_playing".to_string(),
            SettingValue::Bool(submit_now_playing),
        );
        settings.insert(
            "scrobble_percent".to_string(),
            SettingValue::Integer(scrobble_percent),
        );
        extensions.insert(
            "melody.lastfm".to_string(),
            ExtensionConfigEntry {
                enabled: lfm_enabled,
                settings,
            },
        );
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = path::config_file();
        if path.exists() {
            return Self::load_from_path(path);
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

        Self::load_from_path(path)
    }

    pub(crate) fn load_from_path(path: &Path) -> Self {
        if !path.exists() {
            info!("No config file found at {:?}, using default settings", path);
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Self>(&content) {
                Ok(mut config) => {
                    info!("Successfully loaded settings from {:?}", path);
                    config.tab_order = sanitize_tab_order(&config.tab_order);
                    migrate_legacy_scrobble(&content, &mut config.extensions);
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

    pub fn save_to_path(&self, path: &Path) {
        match toml::to_string_pretty(self) {
            Ok(content) => {
                if let Err(err) = path::atomic_write(path, &content) {
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
