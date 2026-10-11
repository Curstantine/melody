#[cfg(target_os = "linux")]
pub mod mpris;
pub mod manager;
pub mod types;

pub use manager::{DiscordRpcPlugin, LyricsPlugin, Plugin, PluginManager};
#[cfg(target_os = "linux")]
pub use mpris::MprisPlugin;
pub use types::{PlayerAction, PluginHook, PluginMetadata, PluginResult, TrackMetadata};
