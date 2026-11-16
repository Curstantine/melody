use std::collections::{BTreeSet, HashSet};
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize};

const DEFAULT_LYRICS_FONT_SIZE: u16 = 15;

fn default_lyrics_font_size() -> u16 {
    DEFAULT_LYRICS_FONT_SIZE
}

fn intern_str(s: &str) -> &'static str {
    static INTERNED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut guard = INTERNED.lock().unwrap();
    let set = guard.get_or_insert_with(HashSet::new);
    if let Some(&existing) = set.get(s) {
        existing
    } else {
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        set.insert(leaked);
        leaked
    }
}

/// Resolves an optional font family name to an [`iced::Font`].
pub fn font_from_family_name(name: Option<&str>) -> iced::Font {
    if let Some(name) = name {
        let family_static = intern_str(name);
        iced::Font::new(family_static)
    } else {
        iced::Font::DEFAULT
    }
}

/// Sorted, de-duplicated family names for the font picker.
/// Uses fontdb (memory-mapped, parses name tables only), so nothing stays in RAM.
pub fn list_font_families() -> Vec<String> {
    use iced::advanced::graphics::text::cosmic_text::fontdb;

    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut families = BTreeSet::new();
    for face in db.faces() {
        if let Some((name, _)) = face.families.first() {
            families.insert(name.clone());
        }
    }

    families.into_iter().collect()
}

/// Single source of truth for persisted font preferences and their cached [`iced::Font`] handles.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FontConfig {
    #[serde(default, rename = "primary_font")]
    pub primary: Option<String>,
    #[serde(default, rename = "lyrics_font")]
    pub lyrics: Option<String>,
    #[serde(default = "default_lyrics_font_size", rename = "lyrics_font_size")]
    pub lyrics_size: u16,

    #[serde(skip)]
    primary_iced: iced::Font,
    #[serde(skip)]
    lyrics_iced: Option<iced::Font>,
}

#[derive(Deserialize)]
struct RawFontConfig {
    #[serde(default, rename = "primary_font")]
    primary: Option<String>,
    #[serde(default, rename = "lyrics_font")]
    lyrics: Option<String>,
    #[serde(default = "default_lyrics_font_size", rename = "lyrics_font_size")]
    lyrics_size: u16,
}

impl<'de> Deserialize<'de> for FontConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawFontConfig::deserialize(deserializer)?;
        Ok(Self::new(raw.primary, raw.lyrics, raw.lyrics_size))
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self::new(None, None, DEFAULT_LYRICS_FONT_SIZE)
    }
}

impl FontConfig {
    pub fn new(primary: Option<String>, lyrics: Option<String>, lyrics_size: u16) -> Self {
        let primary_iced = font_from_family_name(primary.as_deref());
        let lyrics_iced = lyrics
            .as_deref()
            .map(|name| font_from_family_name(Some(name)));
        Self {
            primary,
            lyrics,
            lyrics_size,
            primary_iced,
            lyrics_iced,
        }
    }

    pub fn set_primary(&mut self, font: Option<String>) {
        self.primary_iced = font_from_family_name(font.as_deref());
        self.primary = font;
    }

    pub fn set_lyrics(&mut self, font: Option<String>) {
        self.lyrics_iced = font
            .as_deref()
            .map(|name| font_from_family_name(Some(name)));
        self.lyrics = font;
    }

    pub fn set_lyrics_size(&mut self, size: u16) {
        self.lyrics_size = size.clamp(11, 28);
    }

    /// Cached [`iced::Font`] for general UI text.
    pub fn primary_font(&self) -> iced::Font {
        self.primary_iced
    }

    /// Cached [`iced::Font`] for lyrics, falling back to `primary_font()`.
    pub fn lyrics_font(&self) -> iced::Font {
        self.lyrics_iced.unwrap_or(self.primary_iced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_config_defaults_and_updates() {
        let mut cfg = FontConfig::default();
        assert_eq!(cfg.primary, None);
        assert_eq!(cfg.lyrics, None);
        assert_eq!(cfg.lyrics_size, DEFAULT_LYRICS_FONT_SIZE);
        assert_eq!(cfg.primary_font(), iced::Font::DEFAULT);
        assert_eq!(cfg.lyrics_font(), iced::Font::DEFAULT);

        cfg.set_primary(Some("Inter".to_string()));
        assert_eq!(cfg.primary.as_deref(), Some("Inter"));
        assert_eq!(cfg.primary_font(), font_from_family_name(Some("Inter")));
        assert_eq!(cfg.lyrics_font(), cfg.primary_font());

        cfg.set_lyrics(Some("Noto Serif".to_string()));
        assert_eq!(cfg.lyrics.as_deref(), Some("Noto Serif"));
        assert_eq!(cfg.lyrics_font(), font_from_family_name(Some("Noto Serif")));

        cfg.set_lyrics_size(50);
        assert_eq!(cfg.lyrics_size, 28);
        cfg.set_lyrics_size(4);
        assert_eq!(cfg.lyrics_size, 11);
    }
}
