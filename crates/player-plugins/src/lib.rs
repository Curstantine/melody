pub mod manager;
pub mod types;

pub use manager::{DiscordRpcPlugin, LyricsPlugin, Plugin, PluginManager};
pub use types::{PluginHook, PluginMetadata, PluginResult, TrackMetadata};
