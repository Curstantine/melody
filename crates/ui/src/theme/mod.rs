pub mod styles;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use iced::{theme::Palette, Color, Theme};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

pub const DEFAULT_THEME_NAME: &str = "Dark";

mod hex_color {
    use iced::Color;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Color::parse(&s).ok_or_else(|| D::Error::custom(format!("invalid hex color: {s}")))
    }

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let [r, g, b, a] = color.into_rgba8();
        let hex = if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        };
        serializer.serialize_str(&hex)
    }
}

mod opt_hex_color {
    use iced::Color;
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => Color::parse(&s)
                .map(Some)
                .ok_or_else(|| D::Error::custom(format!("invalid hex color: {s}"))),
            None => Ok(None),
        }
    }

    pub fn serialize<S>(color: &Option<Color>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match color {
            Some(c) => super::hex_color::serialize(c, serializer),
            None => serializer.serialize_none(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    #[serde(with = "hex_color")]
    pub background: Color,
    #[serde(with = "hex_color")]
    pub surface: Color,
    #[serde(with = "hex_color")]
    pub surface_hover: Color,
    #[serde(with = "hex_color")]
    pub border: Color,
    #[serde(with = "hex_color")]
    pub primary: Color,
    #[serde(with = "hex_color")]
    pub accent: Color,
    #[serde(with = "hex_color")]
    pub text: Color,
    #[serde(with = "hex_color")]
    pub text_muted: Color,
    #[serde(with = "hex_color")]
    pub success: Color,
    #[serde(with = "hex_color")]
    pub danger: Color,
    #[serde(default, with = "opt_hex_color")]
    pub player_bar_bg: Option<Color>,
    #[serde(default, with = "opt_hex_color")]
    pub player_bar_text: Option<Color>,
    #[serde(default, with = "opt_hex_color")]
    pub slider_active: Option<Color>,
    #[serde(default, with = "opt_hex_color")]
    pub slider_rail: Option<Color>,
    #[serde(default)]
    pub font_family: Option<String>,
    /// Cached resolved font. Kept in sync with `font_family` via
    /// [`ThemeConfig::refresh_font`]; never serialized. Caching it here removes
    /// a global mutex + hash lookup from every text widget on every frame.
    #[serde(skip)]
    pub font: iced::Font,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_THEME_NAME.to_string(),
            background: parse_hex("#161616"),
            surface: parse_hex("#191919"),
            surface_hover: parse_hex("#282828"),
            border: parse_hex("#282828"),
            primary: parse_hex("#ffffff"),
            accent: parse_hex("#888888"),
            text: parse_hex("#f0f0f0"),
            text_muted: parse_hex("#787878"),
            success: parse_hex("#ffffff"),
            danger: parse_hex("#888888"),
            player_bar_bg: Some(parse_hex("#191919")),
            player_bar_text: Some(parse_hex("#f0f0f0")),
            slider_active: Some(parse_hex("#ffffff")),
            slider_rail: Some(parse_hex("#363636")),
            font_family: None,
            font: iced::Font::DEFAULT,
        }
    }
}

pub fn parse_hex(hex: &str) -> Color {
    Color::parse(hex.trim()).unwrap_or(Color::BLACK)
}

impl ThemeConfig {
    pub fn bg_color(&self) -> Color {
        self.background
    }

    pub fn surface_color(&self) -> Color {
        self.surface
    }

    pub fn surface_hover_color(&self) -> Color {
        self.surface_hover
    }

    pub fn border_color(&self) -> Color {
        self.border
    }

    pub fn primary_color(&self) -> Color {
        self.primary
    }

    pub fn accent_color(&self) -> Color {
        self.accent
    }

    pub fn text_color(&self) -> Color {
        self.text
    }

    pub fn text_muted_color(&self) -> Color {
        self.text_muted
    }

    pub fn success_color(&self) -> Color {
        self.success
    }

    pub fn danger_color(&self) -> Color {
        self.danger
    }

    pub fn player_bar_bg_color(&self) -> Color {
        self.player_bar_bg.unwrap_or(self.surface)
    }

    pub fn player_bar_text_color(&self) -> Color {
        self.player_bar_text.unwrap_or(self.text)
    }

    pub fn slider_active_color(&self) -> Color {
        self.slider_active.unwrap_or(self.primary)
    }

    pub fn slider_rail_color(&self) -> Color {
        self.slider_rail.unwrap_or(self.surface_hover)
    }

    pub fn to_palette(&self) -> Palette {
        Palette {
            background: self.bg_color(),
            text: self.text_color(),
            primary: self.primary_color(),
            success: self.success_color(),
            danger: self.danger_color(),
        }
    }

    pub fn to_iced_theme(&self) -> Theme {
        Theme::custom(self.name.clone(), self.to_palette())
    }

    pub fn font(&self) -> iced::Font {
        self.font
    }

    /// Recomputes the cached [`iced::Font`] from the current `font_family`.
    /// Must be called after any mutation of `font_family`.
    pub fn refresh_font(&mut self) {
        self.font = font_from_family_name(self.font_family.as_deref());
    }
}

fn intern_str(s: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::Mutex;

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

pub fn font_from_family_name(name: Option<&str>) -> iced::Font {
    if let Some(name) = name {
        let family_static = intern_str(name);
        iced::Font::with_name(family_static)
    } else {
        iced::Font::DEFAULT
    }
}

#[derive(Debug, Clone)]
pub struct ThemeManager {
    pub themes: HashMap<String, ThemeConfig>,
    pub active_theme_name: String,
    pub current_config: ThemeConfig,
    cached_theme_names: Vec<String>,
    /// Pre-built iced theme for the active config. Rebuilt only on theme/font
    /// changes so `App::theme()` does not allocate on every frame.
    cached_theme: Theme,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        let default_config = ThemeConfig::default();
        let mut manager = Self {
            themes: HashMap::new(),
            active_theme_name: DEFAULT_THEME_NAME.to_string(),
            current_config: default_config.clone(),
            cached_theme_names: Vec::new(),
            cached_theme: default_config.to_iced_theme(),
        };

        manager.load_all_themes();
        manager
    }

    pub fn theme_dir() -> PathBuf {
        core_engine::user_themes_dir()
    }

    pub fn load_all_themes(&mut self) {
        // First register default builtin theme
        let defaults = vec![ThemeConfig::default()];

        for t in defaults {
            self.themes.insert(t.name.clone(), t);
        }

        // Load from disk: local themes/ folder and config_dir themes/ folder
        self.load_from_folder(Path::new("themes"));
        let cfg_dir = Self::theme_dir();
        self.load_from_folder(&cfg_dir);

        let saved_font = self.current_config.font_family.clone();
        if let Some(active) = self.themes.get(&self.active_theme_name) {
            self.current_config = active.clone();
            if saved_font.is_some() {
                self.current_config.font_family = saved_font;
            }
        }
        self.current_config.refresh_font();
        self.cached_theme = self.current_config.to_iced_theme();

        let mut names: Vec<String> = self.themes.keys().cloned().collect();
        names.sort();
        self.cached_theme_names = names;
    }

    fn load_from_folder(&mut self, folder: &Path) {
        if !folder.exists() {
            let _ = fs::create_dir_all(folder);
            return;
        }

        for entry in walkdir::WalkDir::new(folder)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(path) {
                    match toml::from_str::<ThemeConfig>(&content) {
                        Ok(mut theme) => {
                            theme.refresh_font();
                            info!("Loaded theme '{}' from {:?}", theme.name, path);
                            self.themes.insert(theme.name.clone(), theme);
                        }
                        Err(e) => {
                            warn!("Failed to parse theme file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    pub fn set_theme(&mut self, name: &str) -> bool {
        if let Some(theme) = self.themes.get(name) {
            let saved_font = self.current_config.font_family.clone();
            self.active_theme_name = name.to_string();
            self.current_config = theme.clone();
            if saved_font.is_some() {
                self.current_config.font_family = saved_font;
            }
            self.current_config.refresh_font();
            self.cached_theme = self.current_config.to_iced_theme();
            info!("Switched theme to '{}'", name);
            true
        } else {
            false
        }
    }

    pub fn set_font(&mut self, font_name: Option<String>) {
        info!("Setting active theme font family to: {:?}", font_name);
        self.current_config.font_family = font_name;
        self.current_config.refresh_font();
        self.cached_theme = self.current_config.to_iced_theme();
    }

    /// Returns the cached iced theme. Cheap (`Arc` clone) and allocation-free
    /// compared to rebuilding via `to_iced_theme` every frame.
    pub fn theme(&self) -> Theme {
        self.cached_theme.clone()
    }

    pub fn theme_names(&self) -> &[String] {
        &self.cached_theme_names
    }
}
