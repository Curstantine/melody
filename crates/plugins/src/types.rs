use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub file_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginHook {
    OnTrackChange,
    OnPlaybackEnd,
    OnVolumeChange,
    OnPlaybackStatusChange,
    OnPositionChange,
    OnScrobble,
    OnNowPlaying,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerAction {
    Play,
    Pause,
    TogglePlayPause,
    Stop,
    Next,
    Previous,
    Seek(std::time::Duration),
    SetPosition(std::time::Duration),
    SetVolume(f32),
    SetShuffle(bool),
    SetRepeat(bool),
    OpenUri(String),
    Raise,
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl PluginResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
            data: None,
        }
    }

    pub fn ok_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn err<S: Into<String>>(msg: S) -> Self {
        Self {
            success: false,
            message: Some(msg.into()),
            data: None,
        }
    }
}
