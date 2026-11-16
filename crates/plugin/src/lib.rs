pub mod lyrics;
pub mod manager;
pub mod media_controls;
pub mod scrobbler;
pub mod types;

pub use lyrics::LyricsPlugin;
pub use manager::{Plugin, PluginManager};
#[cfg(target_os = "linux")]
pub use media_controls::MprisPlugin;
#[cfg(target_os = "windows")]
pub use media_controls::WindowsMediaPlugin;
pub use scrobbler::{
    LastFmConfig, LastFmPlugin, ListenBrainzConfig, ListenBrainzPlugin, ScrobbleService,
};
pub use types::{PlayerAction, PluginHook, PluginMetadata, PluginResult, TrackMetadata};
