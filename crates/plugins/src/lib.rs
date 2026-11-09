pub mod manager;
#[cfg(target_os = "linux")]
pub mod mpris;
pub mod scrobbler;
pub mod types;
#[cfg(target_os = "windows")]
pub mod windows_media;

pub use manager::{DiscordRpcPlugin, LyricsPlugin, Plugin, PluginManager};
#[cfg(target_os = "linux")]
pub use mpris::MprisPlugin;
pub use scrobbler::{
    LastFmConfig, LastFmPlugin, ListenBrainzConfig, ListenBrainzPlugin, ScrobbleService,
};
pub use types::{PlayerAction, PluginHook, PluginMetadata, PluginResult, TrackMetadata};
#[cfg(target_os = "windows")]
pub use windows_media::WindowsMediaPlugin;
