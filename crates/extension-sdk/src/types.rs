use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::settings::{SettingOption, SettingValue, SettingsMap};

/// Track metadata exposed to sandboxed extensions (deliberately excludes local file paths).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
}

/// Playback repeat mode exposed to extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    Off,
    All,
    One,
}

/// Query passed to extensions with the `lyrics` permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyricsRequest {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub duration_ms: u64,
}

/// Player control command an extension with `player-control` permission can send to Melody.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ExtensionPlayerAction {
    Play,
    Pause,
    TogglePlayPause,
    Stop,
    Next,
    Previous,
    SeekOffsetMs { offset_ms: i64 },
    SetPositionMs { position_ms: u64 },
    SetVolume { volume: f32 },
    SetShuffle { enabled: bool },
    SetRepeat { mode: RepeatMode },
}

/// Diagnostic log level passed to `host_log`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => Self::Error,
            1 => Self::Warn,
            2 => Self::Info,
            3 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

/// Supported outbound HTTP methods for extensions with `network` permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// Outbound HTTP request sent through Melody's host HTTP bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub form: Option<Vec<(String, String)>>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// HTTP response returned from Melody's host HTTP bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Host-to-extension RPC request envelope serialized across the WASM ABI boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ExtensionRequest {
    Init {
        settings: SettingsMap,
    },
    GetSettingsSchema,
    OnSettingsChange {
        settings: SettingsMap,
    },
    OnAction {
        action_id: String,
        settings: SettingsMap,
    },
    OnTrackChange {
        track: TrackInfo,
    },
    OnPlaybackEnd,
    OnPlaybackStatusChange {
        is_playing: bool,
    },
    OnPositionChange {
        position_ms: u64,
    },
    OnSeeked {
        position_ms: u64,
    },
    OnVolumeChange {
        volume: f32,
    },
    OnRepeatChange {
        mode: RepeatMode,
    },
    OnShuffleChange {
        is_shuffle: bool,
    },
    FetchLyrics {
        request: LyricsRequest,
    },
}

/// Extension-to-host RPC response envelope serialized across the WASM ABI boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtensionResponse {
    Ok,
    Err { message: String },
    SettingsSchema { options: Vec<SettingOption> },
    ActionResult { ok: bool, message: String },
    Lyrics { lyrics: Option<String> },
}

/// Request to launch a browser-based authorization flow with a localhost callback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserAuthRequest {
    /// Authorization URL containing `{callback_url}` placeholder, which the host replaces
    /// with the URL-encoded local `http://127.0.0.1:<port>/callback` address.
    pub auth_url_template: String,
    /// Optional timeout in seconds to wait for the browser redirect (default 120s).
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Query parameters captured from the localhost browser authorization callback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserAuthResponse {
    pub query_params: BTreeMap<String, String>,
}

/// Persisted setting update emitted by an extension via `host::set_setting`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingUpdate {
    pub key: String,
    pub value: SettingValue,
}
