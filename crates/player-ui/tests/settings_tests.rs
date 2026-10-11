#![allow(dead_code)]

#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/theme/mod.rs"]
mod theme;
#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/views/settings.rs"]
mod settings;

use std::path::{Path, PathBuf};
use message::{SettingsTab, ViewMode};
use settings::{SettingsViewData, view};
use theme::ThemeConfig;

#[test]
fn test_settings_tab_variants_and_defaults() {
    let default_tab = SettingsTab::default();
    assert_eq!(default_tab, SettingsTab::Appearance);

    let tabs = [SettingsTab::Appearance, SettingsTab::Library, SettingsTab::Debug];
    assert_ne!(tabs[0], tabs[1]);
    assert_ne!(tabs[1], tabs[2]);
    assert_ne!(tabs[0], tabs[2]);
}

#[test]
fn test_settings_views_render_without_panicking() {
    let theme_cfg = ThemeConfig::default();
    let folders = vec!["/music/ambient".to_string(), "/music/jazz".to_string()];
    let font_paths = vec![PathBuf::from("/usr/share/fonts/noto/NotoSansCJK-Regular.ttc")];
    let db_path = Path::new("/tmp/test_library.db");

    for tab in [SettingsTab::Appearance, SettingsTab::Library, SettingsTab::Debug] {
        let data = SettingsViewData {
            active_tab: tab,
            theme_names: vec!["Default".to_string(), "Nord".to_string()],
            active_theme_name: "Default",
            library_view_mode: ViewMode::List,
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            page_size: 50,
            primary_font: Some("Noto Sans CJK JP"),
            font_paths: &font_paths,
            library_folders: &folders,
            total_tracks: 120,
            total_albums: 15,
            total_artists: 8,
            db_path,
            db_size_bytes: 2048576,
            cache_file_count: 42,
            cache_size_bytes: 524288,
            in_memory_covers_count: 12,
            is_scanning: false,
            volume: 0.85,
            active_plugins_count: 2,
        };

        // Render each view and verify it builds valid Iced elements without panic
        let _element = view(&theme_cfg, data);
    }
}
