pub mod installer;
pub mod manager;
pub mod manifest;
pub mod runtime;
pub mod types;

pub use manager::PluginManager;
pub use manifest::{ExtensionManifest, ExtensionOrigin, ExtensionSnapshot, ExtensionStatus};
pub use melody_extension_sdk::{
    ExtensionConfigEntry, ExtensionPermission, SelectChoice, SettingKind, SettingOption,
    SettingValue, SettingsMap, ABI_VERSION,
};
pub use runtime::ExtensionActionOutcome;
pub use types::{PlayerAction, TrackMetadata};
