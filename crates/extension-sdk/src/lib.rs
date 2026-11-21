pub mod guest;
pub mod host;
pub mod permissions;
pub mod settings;
pub mod types;

pub const ABI_VERSION: u32 = 1;

#[doc(hidden)]
pub mod __private {
    pub use serde_json;
}

pub use guest::{dispatch_request, Extension};
pub use permissions::ExtensionPermission;
pub use settings::{
    ExtensionConfigEntry, SelectChoice, SettingKind, SettingOption, SettingValue, SettingsMap,
};
pub use types::{
    BrowserAuthRequest, BrowserAuthResponse, ExtensionPlayerAction, ExtensionRequest,
    ExtensionResponse, HttpMethod, HttpRequest, HttpResponse, LogLevel, LyricsRequest, RepeatMode,
    SettingUpdate, TrackInfo,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_request_response_roundtrip() {
        let mut settings = SettingsMap::new();
        settings.insert("enabled_extra", SettingValue::Bool(true));
        settings.insert("threshold", SettingValue::Integer(50));
        settings.insert("ratio", SettingValue::Float(0.75));
        settings.insert("token", SettingValue::String("abc".to_string()));

        let req = ExtensionRequest::Init {
            settings: settings.clone(),
        };
        let json = serde_json::to_vec(&req).expect("serialize request");
        let parsed: ExtensionRequest = serde_json::from_slice(&json).expect("deserialize request");
        assert_eq!(req, parsed);
        assert_eq!(settings.get_bool("enabled_extra"), Some(true));
        assert_eq!(settings.get_u32("threshold"), Some(50));
        assert_eq!(settings.get_f64("ratio"), Some(0.75));
        assert_eq!(settings.get_str("token"), Some("abc"));

        let resp = ExtensionResponse::SettingsSchema {
            options: vec![SettingOption {
                key: "mode".to_string(),
                label: "Mode".to_string(),
                description: Some("Select mode".to_string()),
                kind: SettingKind::Select {
                    default: "fast".to_string(),
                    options: vec![SelectChoice {
                        value: "fast".to_string(),
                        label: "Fast".to_string(),
                    }],
                },
            }],
        };
        let rjson = serde_json::to_vec(&resp).expect("serialize response");
        let rparsed: ExtensionResponse =
            serde_json::from_slice(&rjson).expect("deserialize response");
        assert_eq!(resp, rparsed);
    }

    #[test]
    fn test_permission_serialization_and_metadata() {
        let perms = vec![
            ExtensionPermission::PlaybackEvents,
            ExtensionPermission::Lyrics,
            ExtensionPermission::Settings,
            ExtensionPermission::Network,
            ExtensionPermission::PlayerControl,
        ];
        let json = serde_json::to_string(&perms).expect("serialize permissions");
        assert_eq!(
            json,
            r#"["playback-events","lyrics","settings","network","player-control"]"#
        );
        for p in perms {
            assert!(!p.label().is_empty());
            assert!(!p.description().is_empty());
        }
    }

    #[test]
    fn test_config_entry_defaults() {
        let entry = ExtensionConfigEntry {
            enabled: true,
            settings: BTreeMap::from([("scrobble_percent".to_string(), SettingValue::Integer(65))]),
        };
        let json = serde_json::to_string(&entry).expect("serialize entry");
        let decoded: ExtensionConfigEntry = serde_json::from_str(&json).expect("deserialize entry");
        assert_eq!(entry, decoded);
    }
}
