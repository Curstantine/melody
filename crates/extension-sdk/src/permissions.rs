use serde::{Deserialize, Serialize};

/// Capabilities an extension can request in its `extension.toml` manifest.
///
/// The Melody host inspects declared permissions before loading an extension's
/// WebAssembly module, rejects undeclared host imports, and surfaces requested
/// permissions to the user in Settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionPermission {
    PlaybackEvents,
    Lyrics,
    Settings,
    Network,
    PlayerControl,
}

impl ExtensionPermission {
    /// Human-readable short badge title for UI display.
    pub fn label(self) -> &'static str {
        match self {
            Self::PlaybackEvents => "Playback events",
            Self::Lyrics => "Lyrics provider",
            Self::Settings => "Settings UI",
            Self::Network => "Network access",
            Self::PlayerControl => "Player control",
        }
    }

    /// Concise explanation of what granting this permission allows.
    pub fn description(self) -> &'static str {
        match self {
            Self::PlaybackEvents => "Receive track metadata and playback state updates",
            Self::Lyrics => "Supply lyrics for currently playing tracks",
            Self::Settings => "Display custom options and actions in Settings",
            Self::Network => "Send outbound HTTP/HTTPS network requests",
            Self::PlayerControl => "Control playback, seek position, volume, shuffle, and repeat",
        }
    }
}
