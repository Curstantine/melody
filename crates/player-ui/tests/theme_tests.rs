#![allow(dead_code)]

#[path = "../src/theme/mod.rs"]
mod theme;

use theme::{parse_hex, ThemeConfig, ThemeManager};

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

    assert!(names.contains(&"Catppuccin Mocha".to_string()));
    assert!(names.contains(&"Tokyo Night".to_string()));
    assert!(names.contains(&"Nord".to_string()));
    assert!(names.contains(&"Dracula".to_string()));
    assert!(names.contains(&"MusicBee Classic".to_string()));

    let switched = manager.set_theme("Tokyo Night");
    assert!(switched);
    assert_eq!(manager.active_theme_name, "Tokyo Night");
    assert_eq!(manager.current_config.name, "Tokyo Night");

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
}
