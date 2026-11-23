use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};

use melody_extension_sdk::{
    ExtensionPermission, SettingKind, SettingOption, SettingValue, SettingsMap, ABI_VERSION,
};
use serde::{Deserialize, Serialize};

fn default_entry() -> String {
    "extension.wasm".to_string()
}

const fn default_api_version() -> u32 {
    ABI_VERSION
}

/// Validates an extension identifier so it is safe to use as a directory name
/// and config table key.
pub fn validate_extension_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err("Extension id must be between 1 and 64 characters".to_string());
    }
    if id.starts_with('.') || id.contains("..") {
        return Err(format!(
            "Extension id '{id}' contains disallowed path sequences"
        ));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(format!(
            "Extension id '{id}' may only contain ASCII letters, digits, '.', '_', and '-'"
        ));
    }
    Ok(())
}

/// Declarative `extension.toml` manifest for a Melody WebAssembly extension.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    #[serde(default)]
    pub permissions: Vec<ExtensionPermission>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub allowed_ipc: Vec<String>,
    #[serde(default)]
    pub settings: Vec<SettingOption>,
}

impl ExtensionManifest {
    /// Parses a manifest from TOML text and normalizes deduplicated fields.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, String> {
        let mut manifest: Self =
            toml::from_str(toml_str).map_err(|e| format!("Invalid extension.toml: {e}"))?;
        manifest.normalize();
        Ok(manifest)
    }

    /// Deduplicates declared permissions while preserving their manifest order.
    pub fn normalize(&mut self) {
        let mut seen = HashSet::new();
        self.permissions.retain(|p| seen.insert(*p));
    }

    /// Returns `true` if the manifest explicitly declared `perm`.
    pub fn has_permission(&self, perm: ExtensionPermission) -> bool {
        self.permissions.contains(&perm)
    }

    /// Validates metadata, permissions, settings schema, and resolves the WASM entry file inside `ext_dir`.
    pub fn validate(&self, ext_dir: &Path) -> Result<PathBuf, String> {
        validate_extension_id(&self.id)?;

        if self.name.trim().is_empty() {
            return Err("Extension name must not be empty".to_string());
        }
        if self.version.trim().is_empty() {
            return Err("Extension version must not be empty".to_string());
        }
        if self.api_version != ABI_VERSION {
            return Err(format!(
                "Extension '{}' targets API version {}, but Melody supports version {}",
                self.id, self.api_version, ABI_VERSION
            ));
        }

        let entry_rel = Path::new(&self.entry);
        if self.entry.trim().is_empty()
            || entry_rel.components().count() == 0
            || !entry_rel
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(format!(
                "Extension entry '{}' must be a safe relative path",
                self.entry
            ));
        }

        let wasm_path = ext_dir.join(entry_rel);
        if !wasm_path.is_file() {
            return Err(format!(
                "Extension entry module '{}' not found in {}",
                self.entry,
                ext_dir.display()
            ));
        }

        validate_settings_schema(&self.settings)?;

        for pat in &self.allowed_ipc {
            crate::local_ipc::validate_ipc_pattern(pat)?;
        }
        if self.has_permission(ExtensionPermission::LocalIpc) && self.allowed_ipc.is_empty() {
            return Err(
                "Extension declaring 'local-ipc' permission must specify at least one 'allowed_ipc' pattern"
                    .to_string(),
            );
        }

        Ok(wasm_path)
    }
}

/// Validates a list of extension setting options for duplicate keys or invalid bounds.
pub fn validate_settings_schema(options: &[SettingOption]) -> Result<(), String> {
    let mut keys = HashSet::new();
    for opt in options {
        if opt.key.trim().is_empty() {
            return Err("Setting option key must not be empty".to_string());
        }
        if !keys.insert(opt.key.as_str()) {
            return Err(format!("Duplicate setting key '{}'", opt.key));
        }
        if opt.label.trim().is_empty() {
            return Err(format!("Setting '{}' must have a non-empty label", opt.key));
        }
        match &opt.kind {
            SettingKind::Number {
                default,
                min,
                max,
                step,
                ..
            } => {
                if !min.is_finite() || !max.is_finite() || !step.is_finite() || !default.is_finite()
                {
                    return Err(format!(
                        "Number setting '{}' must use finite values",
                        opt.key
                    ));
                }
                if min > max || *step <= 0.0 {
                    return Err(format!(
                        "Number setting '{}' has invalid range/step (min={min}, max={max}, step={step})",
                        opt.key
                    ));
                }
            }
            SettingKind::Select { options, .. } => {
                if options.is_empty() {
                    return Err(format!(
                        "Select setting '{}' must define at least one choice",
                        opt.key
                    ));
                }
                for choice in options {
                    if choice.value.is_empty() {
                        return Err(format!(
                            "Select setting '{}' contains a choice with an empty value",
                            opt.key
                        ));
                    }
                }
            }
            SettingKind::Action { button_label } => {
                if button_label.trim().is_empty() {
                    return Err(format!(
                        "Action setting '{}' must define a non-empty button_label",
                        opt.key
                    ));
                }
            }
            SettingKind::Toggle { .. } | SettingKind::Text { .. } => {}
        }
    }
    Ok(())
}

/// Builds a complete [`SettingsMap`] by seeding schema defaults and overlaying persisted user values.
pub fn merge_settings_with_schema(
    schema: &[SettingOption],
    saved: &BTreeMap<String, SettingValue>,
) -> SettingsMap {
    let mut map = BTreeMap::new();
    for opt in schema {
        if let Some(default_val) = opt.kind.default_value() {
            map.insert(opt.key.clone(), default_val);
        }
    }
    for (key, val) in saved {
        if let Some(opt) = schema.iter().find(|o| &o.key == key) {
            if value_matches_kind(val, &opt.kind) {
                map.insert(key.clone(), val.clone());
            }
        } else {
            map.insert(key.clone(), val.clone());
        }
    }
    SettingsMap::from_map(map)
}

fn value_matches_kind(val: &SettingValue, kind: &SettingKind) -> bool {
    match kind {
        SettingKind::Toggle { .. } => matches!(val, SettingValue::Bool(_)),
        SettingKind::Text { .. } | SettingKind::Select { .. } => {
            matches!(val, SettingValue::String(_))
        }
        SettingKind::Number { .. } => {
            matches!(val, SettingValue::Integer(_) | SettingValue::Float(_))
        }
        SettingKind::Action { .. } => false,
    }
}

/// Where an extension was discovered on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionOrigin {
    Bundled,
    User,
}

/// Current runtime lifecycle state of an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionStatus {
    Disabled,
    Running,
    Error(String),
}

/// Immutable UI-facing snapshot of a discovered extension and its active settings schema.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionSnapshot {
    pub manifest: ExtensionManifest,
    pub origin: ExtensionOrigin,
    pub dir: PathBuf,
    pub enabled: bool,
    pub status: ExtensionStatus,
    pub settings_schema: Vec<SettingOption>,
}

impl ExtensionSnapshot {
    /// Merges `saved` settings onto this extension's effective settings schema.
    pub fn merged_settings(&self, saved: &BTreeMap<String, SettingValue>) -> SettingsMap {
        merge_settings_with_schema(&self.settings_schema, saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parsing_and_settings_merge() {
        let toml = r#"
id = "com.example.demo"
name = "Demo Extension"
version = "1.2.3"
description = "Demo"
authors = ["Alice"]
permissions = ["playback-events", "network", "playback-events"]

[[settings]]
key = "enabled_extra"
label = "Extra flag"
type = "toggle"
default = true

[[settings]]
key = "threshold"
label = "Threshold"
type = "number"
default = 50
min = 10
max = 100
step = 5
unit = "%"

[[settings]]
key = "mode"
label = "Mode"
type = "select"
default = "fast"
options = [
  { value = "fast", label = "Fast" },
  { value = "accurate", label = "Accurate" }
]
"#;

        let manifest = ExtensionManifest::from_toml_str(toml).expect("parse manifest");
        assert_eq!(manifest.id, "com.example.demo");
        assert_eq!(manifest.entry, "extension.wasm");
        assert_eq!(manifest.api_version, 1);
        assert_eq!(
            manifest.permissions,
            vec![
                ExtensionPermission::PlaybackEvents,
                ExtensionPermission::Network
            ]
        );
        assert_eq!(manifest.settings.len(), 3);

        let saved = BTreeMap::from([("threshold".to_string(), SettingValue::Integer(80))]);
        let merged = merge_settings_with_schema(&manifest.settings, &saved);
        assert_eq!(merged.get_bool("enabled_extra"), Some(true));
        assert_eq!(merged.get_u32("threshold"), Some(80));
        assert_eq!(merged.get_str("mode"), Some("fast"));
    }

    #[test]
    fn test_manifest_validation_rejects_bad_ids_and_traversal() {
        assert!(validate_extension_id("").is_err());
        assert!(validate_extension_id("../evil").is_err());
        assert!(validate_extension_id(".hidden").is_err());
        assert!(validate_extension_id("bad/slash").is_err());
        assert!(validate_extension_id("good.ext_1-a").is_ok());

        let temp_dir =
            std::env::temp_dir().join(format!("melody_manifest_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("extension.wasm"), b"fake").unwrap();

        let mut manifest = ExtensionManifest::from_toml_str(
            r#"
id = "ok.ext"
name = "OK"
version = "0.1.0"
entry = "../escape.wasm"
"#,
        )
        .unwrap();
        assert!(manifest.validate(&temp_dir).is_err());

        manifest.entry = "extension.wasm".to_string();
        assert!(manifest.validate(&temp_dir).is_ok());

        manifest.api_version = 99;
        assert!(manifest.validate(&temp_dir).is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_manifest_validation_rejects_duplicate_setting_keys() {
        let manifest = ExtensionManifest::from_toml_str(
            r#"
id = "dup.keys"
name = "Dup"
version = "0.1.0"

[[settings]]
key = "dup"
label = "One"
type = "toggle"

[[settings]]
key = "dup"
label = "Two"
type = "text"
"#,
        )
        .unwrap();

        assert!(validate_settings_schema(&manifest.settings).is_err());
    }

    #[test]
    fn test_manifest_validation_enforces_allowed_ipc() {
        let temp_dir =
            std::env::temp_dir().join(format!("melody_manifest_ipc_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("extension.wasm"), b"fake").unwrap();

        let empty_ipc = ExtensionManifest::from_toml_str(
            r#"
id = "ipc.empty"
name = "IPC Empty"
version = "0.1.0"
permissions = ["local-ipc"]
"#,
        )
        .unwrap();
        assert!(empty_ipc.validate(&temp_dir).is_err());

        let wildcard_ipc = ExtensionManifest::from_toml_str(
            r#"
id = "ipc.star"
name = "IPC Star"
version = "0.1.0"
permissions = ["local-ipc"]
allowed_ipc = ["*"]
"#,
        )
        .unwrap();
        assert!(wildcard_ipc.validate(&temp_dir).is_err());

        let traversal_ipc = ExtensionManifest::from_toml_str(
            r#"
id = "ipc.trav"
name = "IPC Trav"
version = "0.1.0"
permissions = ["local-ipc"]
allowed_ipc = ["../a*"]
"#,
        )
        .unwrap();
        assert!(traversal_ipc.validate(&temp_dir).is_err());

        let valid_ipc = ExtensionManifest::from_toml_str(
            r#"
id = "ipc.ok"
name = "IPC OK"
version = "0.1.0"
permissions = ["local-ipc"]
allowed_ipc = ["app/com.example.App/ipc-*"]
"#,
        )
        .unwrap();
        assert!(valid_ipc.validate(&temp_dir).is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
