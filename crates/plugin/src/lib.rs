pub mod installer;
pub mod lyrics;
pub mod manager;
pub mod manifest;
pub mod media_controls;
pub mod runtime;
pub mod types;

pub use lyrics::LyricsPlugin;
pub use manager::{Plugin, PluginManager};
pub use manifest::{ExtensionManifest, ExtensionOrigin, ExtensionSnapshot, ExtensionStatus};
#[cfg(target_os = "linux")]
pub use media_controls::MprisPlugin;
#[cfg(target_os = "windows")]
pub use media_controls::WindowsMediaPlugin;
pub use melody_extension_sdk::{
    ExtensionConfigEntry, ExtensionPermission, SelectChoice, SettingKind, SettingOption,
    SettingValue, SettingsMap, ABI_VERSION,
};
pub use types::{PlayerAction, PluginMetadata, TrackMetadata};
