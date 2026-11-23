use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const fn default_step() -> f64 {
    1.0
}

/// Choice item for a [`SettingKind::Select`] option.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectChoice {
    pub value: String,
    pub label: String,
}

/// Declarative UI control type for an extension setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SettingKind {
    Toggle {
        #[serde(default)]
        default: bool,
    },
    Text {
        #[serde(default)]
        default: String,
        #[serde(default)]
        placeholder: Option<String>,
    },
    Number {
        default: f64,
        min: f64,
        max: f64,
        #[serde(default = "default_step")]
        step: f64,
        #[serde(default)]
        unit: Option<String>,
    },
    Select {
        default: String,
        options: Vec<SelectChoice>,
    },
    Action {
        button_label: String,
    },
}

impl SettingKind {
    /// Returns the default persisted value for input-backed setting kinds, or `None` for actions.
    pub fn default_value(&self) -> Option<SettingValue> {
        match self {
            Self::Toggle { default } => Some(SettingValue::Bool(*default)),
            Self::Text { default, .. } => Some(SettingValue::String(default.clone())),
            Self::Number { default, step, .. } => {
                if step.fract() == 0.0 && default.fract() == 0.0 {
                    Some(SettingValue::Integer(*default as i64))
                } else {
                    Some(SettingValue::Float(*default))
                }
            }
            Self::Select { default, .. } => Some(SettingValue::String(default.clone())),
            Self::Action { .. } => None,
        }
    }
}

/// Metadata and UI definition for a single extension setting or action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingOption {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(flatten)]
    pub kind: SettingKind,
}

/// Typed value stored for an extension setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl SettingValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Integer(i) => Some(*i as f64),
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::Float(f) if f.is_finite() => Some(f.round() as i64),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        self.as_i64().and_then(|v| u32::try_from(v).ok())
    }
}

/// Map of setting keys to values passed to extensions during lifecycle & action calls.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SettingsMap(pub BTreeMap<String, SettingValue>);

impl SettingsMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_map(map: BTreeMap<String, SettingValue>) -> Self {
        Self(map)
    }

    pub fn get(&self, key: &str) -> Option<&SettingValue> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: SettingValue) -> Option<SettingValue> {
        self.0.insert(key.into(), value)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(SettingValue::as_bool)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(SettingValue::as_str)
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.0.get(key).and_then(SettingValue::as_f64)
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(SettingValue::as_i64)
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.0.get(key).and_then(SettingValue::as_u32)
    }
}

/// Persisted configuration state for a single extension in Melody's `config.toml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtensionConfigEntry {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: BTreeMap<String, SettingValue>,
}
