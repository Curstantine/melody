use iced::{theme::palette::Seed, Color, Theme};
use serde::{Deserialize, Serialize};

use crate::{
    colors::{hex_color, opt_hex_color, parse_hex},
    fonts::font_from_family_name,
    radius::BorderRadiusConfig,
};

pub const DEFAULT_THEME_NAME: &str = "Dark";

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

impl ThemeConfig {
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
            background: self.background,
            text: self.text,
            primary: self.primary,
            success: self.success,
            warning: self.accent,
            danger: self.danger,
        }
    }

    pub fn to_iced_theme(&self) -> Theme {
        Theme::custom(self.name.clone(), self.to_seed())
    }

    /// Recomputes the cached [`iced::Font`] from the current `font_family`.
    /// Must be called after any mutation of `font_family`.
    pub fn refresh_font(&mut self) {
        self.font = font_from_family_name(self.font_family.as_deref());
    }
}

#[cfg(test)]
mod tests {
    use iced::border::Radius;

    use super::*;

    #[test]
    fn test_border_radius_defaults_and_helpers() {
        let config = BorderRadiusConfig::default();
        assert_eq!(config.xs, Radius::from(2.0));
        assert_eq!(config.sm, Radius::from(4.0));
        assert_eq!(config.md, Radius::from(6.0));
        assert_eq!(config.lg, Radius::from(8.0));
        assert_eq!(config.xl, Radius::from(16.0));

        let theme = ThemeConfig::default();
        assert_eq!(theme.border_radius, config);
    }

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
        assert_eq!(theme.border_radius.xs, Radius::from(1.0));
        assert_eq!(theme.border_radius.sm, Radius::from(3.0));
        assert_eq!(theme.border_radius.md, Radius::from(5.0));
        assert_eq!(theme.border_radius.lg, Radius::from(7.0));
        assert_eq!(theme.border_radius.xl, Radius::from(12.0));

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
        assert_eq!(theme.border_radius.xs, Radius::from(0.0));
        assert_eq!(theme.border_radius.sm, Radius::from(2.0));
        assert_eq!(theme.border_radius.md, Radius::from(4.0));
        assert_eq!(theme.border_radius.lg, Radius::from(6.0));
        assert_eq!(theme.border_radius.xl, Radius::from(10.0));

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
        assert_eq!(theme.border_radius.xs, Radius::from(2.0)); // default
        assert_eq!(theme.border_radius.sm, Radius::from(10.0));
        assert_eq!(theme.border_radius.md, Radius::from(6.0)); // default
        assert_eq!(theme.border_radius.lg, Radius::from(20.0));
        assert_eq!(theme.border_radius.xl, Radius::from(16.0)); // default

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
        assert_eq!(theme.border_radius.xs, Radius::from(0.0));
        assert_eq!(theme.border_radius.sm, Radius::from(0.0));
        assert_eq!(theme.border_radius.md, Radius::from(0.0));
        assert_eq!(theme.border_radius.lg, Radius::from(0.0));
        assert_eq!(theme.border_radius.xl, Radius::from(0.0));
    }
}
