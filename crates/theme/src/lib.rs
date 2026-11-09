use std::{collections::HashMap, fs, path::Path};

use iced::Theme;
use tracing::{info, warn};

pub use crate::{
    colors::hex_color,
    config::{ThemeConfig, DEFAULT_THEME_NAME},
    fonts::{font_from_family_name, list_font_families},
};

mod colors;
mod config;
mod fonts;
mod radius;

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

    pub fn load_all_themes(&mut self) {
        self.themes
            .insert(DEFAULT_THEME_NAME.to_string(), ThemeConfig::default());

        if let Some(bundled) = paths::bundled_themes_dir() {
            self.load_from_folder(&bundled);
        }
        self.load_from_folder(paths::user_themes_dir());

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
        if !folder.is_dir() {
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
}
