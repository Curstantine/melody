#![allow(dead_code)]

#[path = "../src/theme/mod.rs"]
mod theme;

use iced::border::Radius;
use theme::{parse_hex, BorderRadiusConfig, ThemeConfig, ThemeManager, DEFAULT_THEME_NAME};

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
