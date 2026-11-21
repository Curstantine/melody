use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub file_path: String,
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
    SetRepeat(model::RepeatMode),
    OpenUri(String),
    Raise,
    Quit,
}
