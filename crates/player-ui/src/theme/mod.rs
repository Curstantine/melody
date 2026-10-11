pub mod styles;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use iced::{theme::Palette, Color, Theme};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub background: String,
    pub surface: String,
    pub surface_hover: String,
    pub border: String,
    pub primary: String,
    pub accent: String,
    pub text: String,
    pub text_muted: String,
    pub success: String,
    pub danger: String,
    #[serde(default)]
    pub player_bar_bg: Option<String>,
    #[serde(default)]
    pub player_bar_text: Option<String>,
    #[serde(default)]
    pub slider_active: Option<String>,
    #[serde(default)]
    pub slider_rail: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            background: "#161616".to_string(),
            surface: "#191919".to_string(),
            surface_hover: "#282828".to_string(),
            border: "#282828".to_string(),
            primary: "#ffffff".to_string(),
            accent: "#888888".to_string(),
            text: "#f0f0f0".to_string(),
            text_muted: "#787878".to_string(),
            success: "#ffffff".to_string(),
            danger: "#888888".to_string(),
            player_bar_bg: Some("#191919".to_string()),
            player_bar_text: Some("#f0f0f0".to_string()),
            slider_active: Some("#ffffff".to_string()),
            slider_rail: Some("#363636".to_string()),
            font_family: None,
        }
    }
}

pub fn parse_hex(hex: &str) -> Color {
    let clean = hex.trim().trim_start_matches('#');
    match clean.len() {
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0) as f32 / 255.0;
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0) as f32 / 255.0;
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0) as f32 / 255.0;
            Color::from_rgb(r, g, b)
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0) as f32 / 255.0;
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0) as f32 / 255.0;
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0) as f32 / 255.0;
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255) as f32 / 255.0;
            Color::from_rgba(r, g, b, a)
        }
        _ => Color::BLACK,
    }
}

impl ThemeConfig {
    pub fn bg_color(&self) -> Color {
        parse_hex(&self.background)
    }

    pub fn surface_color(&self) -> Color {
        parse_hex(&self.surface)
    }

    pub fn surface_hover_color(&self) -> Color {
        parse_hex(&self.surface_hover)
    }

    pub fn border_color(&self) -> Color {
        parse_hex(&self.border)
    }

    pub fn primary_color(&self) -> Color {
        parse_hex(&self.primary)
    }

    pub fn accent_color(&self) -> Color {
        parse_hex(&self.accent)
    }

    pub fn text_color(&self) -> Color {
        parse_hex(&self.text)
    }

    pub fn text_muted_color(&self) -> Color {
        parse_hex(&self.text_muted)
    }

    pub fn success_color(&self) -> Color {
        parse_hex(&self.success)
    }

    pub fn danger_color(&self) -> Color {
        parse_hex(&self.danger)
    }

    pub fn player_bar_bg_color(&self) -> Color {
        self.player_bar_bg
            .as_deref()
            .map(parse_hex)
            .unwrap_or_else(|| self.surface_color())
    }

    pub fn player_bar_text_color(&self) -> Color {
        self.player_bar_text
            .as_deref()
            .map(parse_hex)
            .unwrap_or_else(|| self.text_color())
    }

    pub fn slider_active_color(&self) -> Color {
        self.slider_active
            .as_deref()
            .map(parse_hex)
            .unwrap_or_else(|| self.primary_color())
    }

    pub fn slider_rail_color(&self) -> Color {
        self.slider_rail
            .as_deref()
            .map(parse_hex)
            .unwrap_or_else(|| self.surface_hover_color())
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

    #[allow(dead_code)]
    pub fn font(&self) -> iced::Font {
        if let Some(ref name) = self.font_family {
            iced::Font {
                family: iced::font::Family::Name(Box::leak(name.clone().into_boxed_str())),
                ..iced::Font::DEFAULT
            }
        } else {
            iced::Font::DEFAULT
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThemeManager {
    pub themes: HashMap<String, ThemeConfig>,
    pub active_theme_name: String,
    pub current_config: ThemeConfig,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut manager = Self {
            themes: HashMap::new(),
            active_theme_name: "Default".to_string(),
            current_config: ThemeConfig::default(),
        };

        manager.load_all_themes();
        manager
    }

    pub fn theme_dir() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("org", "vibecore", "vibecore") {
            proj_dirs.config_dir().join("themes")
        } else {
            PathBuf::from("themes")
        }
    }

    pub fn load_all_themes(&mut self) {
        // First register default builtin theme
        let defaults = vec![
            ThemeConfig::default(),
        ];

        for t in defaults {
            self.themes.insert(t.name.clone(), t);
        }

        // Load from disk: local themes/ folder and config_dir themes/ folder
        self.load_from_folder(Path::new("themes"));
        let cfg_dir = Self::theme_dir();
        self.load_from_folder(&cfg_dir);

        if let Some(active) = self.themes.get(&self.active_theme_name) {
            self.current_config = active.clone();
        }
    }

    fn load_from_folder(&mut self, folder: &Path) {
        if !folder.exists() {
            let _ = fs::create_dir_all(folder);
            return;
        }

        if let Ok(entries) = fs::read_dir(folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        match toml::from_str::<ThemeConfig>(&content) {
                            Ok(theme) => {
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
    }

    pub fn set_theme(&mut self, name: &str) -> bool {
        if let Some(theme) = self.themes.get(name) {
            self.active_theme_name = name.to_string();
            self.current_config = theme.clone();
            info!("Switched theme to '{}'", name);
            true
        } else {
            false
        }
    }

    pub fn theme_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.themes.keys().cloned().collect();
        names.sort();
        names
    }
}
