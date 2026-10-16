pub mod schema;
pub mod view;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use tracing::{error, info, warn};

pub use schema::{
    parse_kdl_layout, Alignment, ContainerStyle, LayoutComponent, LayoutNode, LayoutParseError,
    LayoutTemplate, Orientation, SizeConstraint,
};

pub const DEFAULT_LAYOUT_NAME: &str = "Default";

#[derive(Debug, Clone)]
pub struct LayoutManager {
    pub layouts: HashMap<String, LayoutTemplate>,
    pub active_layout_name: String,
    pub current_template: LayoutTemplate,
    cached_layout_names: Vec<String>,
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutManager {
    pub fn new() -> Self {
        let default_template = LayoutTemplate::default();
        let mut manager = Self {
            layouts: HashMap::new(),
            active_layout_name: DEFAULT_LAYOUT_NAME.to_string(),
            current_template: default_template.clone(),
            cached_layout_names: Vec::new(),
        };

        manager.load_all_layouts();
        manager
    }

    pub fn layout_dir() -> PathBuf {
        core_engine::user_layouts_dir()
    }

    pub fn load_all_layouts(&mut self) {
        // 1. Register builtin default layout
        let default_template = LayoutTemplate::default();
        self.layouts
            .insert(default_template.name.clone(), default_template.clone());

        // 2. Load from disk: local layouts/ folder and config_dir layouts/ folder
        for candidate in &[
            Path::new("layouts"),
            Path::new("../layouts"),
            Path::new("../../layouts"),
        ] {
            self.load_from_folder(candidate);
        }
        let cfg_dir = Self::layout_dir();
        self.load_from_folder(&cfg_dir);

        // 3. Refresh active layout template
        if let Some(active) = self.layouts.get(&self.active_layout_name) {
            self.current_template = active.clone();
        } else {
            self.active_layout_name = DEFAULT_LAYOUT_NAME.to_string();
            self.current_template = default_template;
        }

        self.refresh_cached_names();
        info!(
            "Loaded {} layouts; active layout: '{}'",
            self.layouts.len(),
            self.active_layout_name
        );
    }

    fn load_from_folder(&mut self, folder: &Path) {
        if !folder.is_dir() {
            return;
        }

        let entries = match fs::read_dir(folder) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read layout directory {:?}: {e}", folder);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("kdl") {
                match fs::read_to_string(&path) {
                    Ok(content) => match parse_kdl_layout(&content) {
                        Ok(mut template) => {
                            // If name is default "Custom" and filename has a better name, use file stem
                            if template.name == "Custom" {
                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                    template.name = stem.to_string();
                                }
                            }
                            info!("Loaded layout '{}' from {:?}", template.name, path);
                            self.layouts.insert(template.name.clone(), template);
                        }
                        Err(e) => {
                            error!("Failed to parse layout file {:?}: {e}", path);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read layout file {:?}: {e}", path);
                    }
                }
            }
        }
    }

    pub fn select_layout(&mut self, name: &str) -> bool {
        if let Some(template) = self.layouts.get(name) {
            self.active_layout_name = name.to_string();
            self.current_template = template.clone();
            info!("Switched to layout: '{name}'");
            true
        } else {
            warn!(
                "Layout '{name}' not found; remaining on '{}'",
                self.active_layout_name
            );
            false
        }
    }

    pub fn layout_names(&self) -> &[String] {
        &self.cached_layout_names
    }

    fn refresh_cached_names(&mut self) {
        let mut names: Vec<String> = self.layouts.keys().cloned().collect();
        names.sort();
        if let Some(pos) = names.iter().position(|n| n == DEFAULT_LAYOUT_NAME) {
            let default_name = names.remove(pos);
            names.insert(0, default_name);
        }
        self.cached_layout_names = names;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_manager_defaults() {
        let manager = LayoutManager::new();
        assert_eq!(manager.active_layout_name, DEFAULT_LAYOUT_NAME);
        assert!(manager.layouts.contains_key(DEFAULT_LAYOUT_NAME));
        assert!(manager
            .layout_names()
            .contains(&DEFAULT_LAYOUT_NAME.to_string()));
    }

    #[test]
    fn test_layout_manager_selection() {
        let mut manager = LayoutManager::new();
        assert!(!manager.select_layout("NonExistentLayout"));
        assert_eq!(manager.active_layout_name, DEFAULT_LAYOUT_NAME);

        if manager.layouts.contains_key("Sidebar") {
            assert!(manager.select_layout("Sidebar"));
            assert_eq!(manager.active_layout_name, "Sidebar");
        }
    }

    #[test]
    fn test_layout_manager_bundled_discovery() {
        let manager = LayoutManager::new();
        let names = manager.layout_names();
        assert!(names.contains(&"Default".to_string()));
        assert!(names.contains(&"Sidebar".to_string()));
        assert!(names.contains(&"Compact".to_string()));
    }
}
