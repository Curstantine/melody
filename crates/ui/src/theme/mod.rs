pub mod styles;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use iced::{theme::palette::Seed, Color, Theme};
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
        s.parse::<Color>()
            .map_err(|_| D::Error::custom(format!("invalid hex color: {s}")))
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
            Some(s) => s
                .parse::<Color>()
                .map(Some)
                .map_err(|_| D::Error::custom(format!("invalid hex color: {s}"))),
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

use iced::border::Radius;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserializer, Serializer};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadiusConfig {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

pub fn default_radius_xs() -> f32 {
    2.0
}
pub fn default_radius_sm() -> f32 {
    4.0
}
pub fn default_radius_md() -> f32 {
    6.0
}
pub fn default_radius_lg() -> f32 {
    8.0
}
pub fn default_radius_xl() -> f32 {
    16.0
}

impl Default for BorderRadiusConfig {
    fn default() -> Self {
        Self {
            xs: default_radius_xs(),
            sm: default_radius_sm(),
            md: default_radius_md(),
            lg: default_radius_lg(),
            xl: default_radius_xl(),
        }
    }
}

impl BorderRadiusConfig {
    pub fn xs(&self) -> Radius {
        Radius::from(self.xs)
    }

    pub fn sm(&self) -> Radius {
        Radius::from(self.sm)
    }

    pub fn md(&self) -> Radius {
        Radius::from(self.md)
    }

    pub fn lg(&self) -> Radius {
        Radius::from(self.lg)
    }

    pub fn xl(&self) -> Radius {
        Radius::from(self.xl)
    }
}

impl Serialize for BorderRadiusConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("BorderRadiusConfig", 5)?;
        s.serialize_field("xs", &self.xs)?;
        s.serialize_field("sm", &self.sm)?;
        s.serialize_field("md", &self.md)?;
        s.serialize_field("lg", &self.lg)?;
        s.serialize_field("xl", &self.xl)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for BorderRadiusConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BorderRadiusVisitor;

        impl<'de> Visitor<'de> for BorderRadiusVisitor {
            type Value = BorderRadiusConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or a border radius map with xs, sm, md, lg, xl")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_f64(v as f64)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_f64(v as f64)
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let r = v as f32;
                if r <= 0.0 {
                    Ok(BorderRadiusConfig {
                        xs: 0.0,
                        sm: 0.0,
                        md: 0.0,
                        lg: 0.0,
                        xl: 0.0,
                    })
                } else {
                    Ok(BorderRadiusConfig {
                        xs: (r * 0.5).round(),
                        sm: r,
                        md: (r * 1.5).round(),
                        lg: (r * 2.0).round(),
                        xl: (r * 4.0).round(),
                    })
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut xs = None;
                let mut sm = None;
                let mut md = None;
                let mut lg = None;
                let mut xl = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "xs" => xs = Some(map.next_value::<f32>()?),
                        "sm" => sm = Some(map.next_value::<f32>()?),
                        "md" => md = Some(map.next_value::<f32>()?),
                        "lg" => lg = Some(map.next_value::<f32>()?),
                        "xl" => xl = Some(map.next_value::<f32>()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(BorderRadiusConfig {
                    xs: xs.unwrap_or_else(default_radius_xs),
                    sm: sm.unwrap_or_else(default_radius_sm),
                    md: md.unwrap_or_else(default_radius_md),
                    lg: lg.unwrap_or_else(default_radius_lg),
                    xl: xl.unwrap_or_else(default_radius_xl),
                })
            }
        }

        deserializer.deserialize_any(BorderRadiusVisitor)
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
    #[serde(default, alias = "radius")]
    pub border_radius: BorderRadiusConfig,
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
            border_radius: BorderRadiusConfig::default(),
            font_family: None,
            font: iced::Font::DEFAULT,
        }
    }
}

pub fn parse_hex(hex: &str) -> Color {
    hex.trim().parse().unwrap_or(Color::BLACK)
}

impl ThemeConfig {
    pub fn radius_xs(&self) -> Radius {
        self.border_radius.xs()
    }

    pub fn radius_sm(&self) -> Radius {
        self.border_radius.sm()
    }

    pub fn radius_md(&self) -> Radius {
        self.border_radius.md()
    }

    pub fn radius_lg(&self) -> Radius {
        self.border_radius.lg()
    }

    pub fn radius_xl(&self) -> Radius {
        self.border_radius.xl()
    }
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
        self.slider_rail.unwrap_or(self.surface)
    }

    pub fn to_seed(&self) -> Seed {
        Seed {
            background: self.bg_color(),
            text: self.text_color(),
            primary: self.primary_color(),
            success: self.success_color(),
            warning: self.accent_color(),
            danger: self.danger_color(),
        }
    }

    pub fn to_iced_theme(&self) -> Theme {
        Theme::custom(self.name.clone(), self.to_seed())
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
        iced::Font::new(family_static)
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
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                self.load_from_folder(&exe_dir.join("themes"));
                self.load_from_folder(&exe_dir.join("../themes"));
                self.load_from_folder(&exe_dir.join("../share/melody/themes"));
            }
        }
        let cfg_dir = Self::theme_dir();
        self.load_from_folder(&cfg_dir);
        if let Ok(xdg_dirs) = std::env::var("XDG_CONFIG_DIRS") {
            for dir in xdg_dirs.split(':') {
                self.load_from_folder(&Path::new(dir).join("melody").join("themes"));
            }
        }
        self.load_from_folder(Path::new("/etc/xdg/melody/themes"));
        self.load_from_folder(Path::new("/etc/melody/themes"));

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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::border::Radius;

    #[test]
    fn test_hex_color_parsing() {
        let black = parse_hex("#000000");
        assert_eq!(black.r, 0.0);
        assert_eq!(black.g, 0.0);
        assert_eq!(black.b, 0.0);
        assert_eq!(black.a, 1.0);

        let white = parse_hex("#ffffff");
        assert_eq!(white.r, 1.0);
        assert_eq!(white.g, 1.0);
        assert_eq!(white.b, 1.0);

        let red_with_alpha = parse_hex("#ff000080");
        assert_eq!(red_with_alpha.r, 1.0);
        assert_eq!(red_with_alpha.g, 0.0);
        assert_eq!(red_with_alpha.b, 0.0);
        assert!((red_with_alpha.a - 0.501).abs() < 0.01);
    }

    #[test]
    fn test_theme_manager() {
        let mut manager = ThemeManager::new();
        let names = manager.theme_names();

        assert!(names.contains(&DEFAULT_THEME_NAME.to_string()));
        assert_eq!(manager.active_theme_name, DEFAULT_THEME_NAME);
        assert_eq!(manager.current_config.name, DEFAULT_THEME_NAME);

        let switched = manager.set_theme(DEFAULT_THEME_NAME);
        assert!(switched);
        assert_eq!(manager.active_theme_name, DEFAULT_THEME_NAME);
        assert_eq!(manager.current_config.name, DEFAULT_THEME_NAME);

        let invalid = manager.set_theme("NonExistentTheme");
        assert!(!invalid);
    }

    #[test]
    fn test_theme_toml_roundtrip() {
        let original = ThemeConfig::default();
        let toml_str = toml::to_string(&original).expect("Serialization failed");
        let deserialized: ThemeConfig = toml::from_str(&toml_str).expect("Deserialization failed");

        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.background, deserialized.background);
        assert_eq!(original.primary, deserialized.primary);
        assert_eq!(original.border_radius, deserialized.border_radius);
    }

    #[test]
    fn test_border_radius_defaults_and_helpers() {
        let config = BorderRadiusConfig::default();
        assert_eq!(config.xs, 2.0);
        assert_eq!(config.sm, 4.0);
        assert_eq!(config.md, 6.0);
        assert_eq!(config.lg, 8.0);
        assert_eq!(config.xl, 16.0);

        assert_eq!(config.xs(), Radius::from(2.0));
        assert_eq!(config.sm(), Radius::from(4.0));
        assert_eq!(config.md(), Radius::from(6.0));
        assert_eq!(config.lg(), Radius::from(8.0));
        assert_eq!(config.xl(), Radius::from(16.0));

        let theme = ThemeConfig::default();
        assert_eq!(theme.radius_xs(), Radius::from(2.0));
        assert_eq!(theme.radius_sm(), Radius::from(4.0));
        assert_eq!(theme.radius_md(), Radius::from(6.0));
        assert_eq!(theme.radius_lg(), Radius::from(8.0));
        assert_eq!(theme.radius_xl(), Radius::from(16.0));
    }

    #[test]
    fn test_border_radius_deserialization_variants() {
        // 1. Explicit [border_radius] table
        let toml_detailed = r##"
name = "Custom Radius"
background = "#161616"
surface = "#191919"
surface_hover = "#282828"
border = "#282828"
primary = "#ffffff"
accent = "#888888"
text = "#f0f0f0"
text_muted = "#787878"
success = "#ffffff"
danger = "#888888"

[border_radius]
xs = 1.0
sm = 3.0
md = 5.0
lg = 7.0
xl = 12.0
"##;
        let theme: ThemeConfig = toml::from_str(toml_detailed).unwrap();
        assert_eq!(theme.border_radius.xs, 1.0);
        assert_eq!(theme.border_radius.sm, 3.0);
        assert_eq!(theme.border_radius.md, 5.0);
        assert_eq!(theme.border_radius.lg, 7.0);
        assert_eq!(theme.border_radius.xl, 12.0);

        // 2. Aliased [radius] table
        let toml_aliased = r##"
name = "Aliased Radius"
background = "#161616"
surface = "#191919"
surface_hover = "#282828"
border = "#282828"
primary = "#ffffff"
accent = "#888888"
text = "#f0f0f0"
text_muted = "#787878"
success = "#ffffff"
danger = "#888888"

[radius]
xs = 0.0
sm = 2.0
md = 4.0
lg = 6.0
xl = 10.0
"##;
        let theme: ThemeConfig = toml::from_str(toml_aliased).unwrap();
        assert_eq!(theme.border_radius.xs, 0.0);
        assert_eq!(theme.border_radius.sm, 2.0);
        assert_eq!(theme.border_radius.md, 4.0);
        assert_eq!(theme.border_radius.lg, 6.0);
        assert_eq!(theme.border_radius.xl, 10.0);

        // 3. Partial [border_radius] (defaults for omitted fields)
        let toml_partial = r##"
name = "Partial Radius"
background = "#161616"
surface = "#191919"
surface_hover = "#282828"
border = "#282828"
primary = "#ffffff"
accent = "#888888"
text = "#f0f0f0"
text_muted = "#787878"
success = "#ffffff"
danger = "#888888"

[border_radius]
sm = 10.0
lg = 20.0
"##;
        let theme: ThemeConfig = toml::from_str(toml_partial).unwrap();
        assert_eq!(theme.border_radius.xs, 2.0); // default
        assert_eq!(theme.border_radius.sm, 10.0);
        assert_eq!(theme.border_radius.md, 6.0); // default
        assert_eq!(theme.border_radius.lg, 20.0);
        assert_eq!(theme.border_radius.xl, 16.0); // default

        // 4. Uniform zero radius (e.g. brutalist / sharp square theme)
        let toml_zero = r##"
name = "Square Theme"
background = "#161616"
surface = "#191919"
surface_hover = "#282828"
border = "#282828"
primary = "#ffffff"
accent = "#888888"
text = "#f0f0f0"
text_muted = "#787878"
success = "#ffffff"
danger = "#888888"
border_radius = 0.0
"##;
        let theme: ThemeConfig = toml::from_str(toml_zero).unwrap();
        assert_eq!(theme.border_radius.xs, 0.0);
        assert_eq!(theme.border_radius.sm, 0.0);
        assert_eq!(theme.border_radius.md, 0.0);
        assert_eq!(theme.border_radius.lg, 0.0);
        assert_eq!(theme.border_radius.xl, 0.0);
    }
}
