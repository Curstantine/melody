use std::path::PathBuf;

use iced::widget::{row, scrollable, Widget};
use iced::{Element, Length};

use crate::message::Message;
use component::{self, styles};
use config::SettingsTab;
use model::ViewMode;
use model::{SortDirection, SortField};
use theme::ThemeConfig;

mod appearance;
mod debug;
mod library;
mod playback;
mod scrobbling;
mod sidebar;
mod updates;
// ─────────────────────────────────────────────────────────────────────────────
// Spacing & Padding Constants
// ─────────────────────────────────────────────────────────────────────────────

pub const PAGE_SPACING: f32 = 18.0;
pub const PAGE_PADDING: [f32; 2] = [18.0, 24.0];
pub const PAGE_BOTTOM_SPACER: f32 = 20.0;

pub const SIDEBAR_WIDTH: f32 = 220.0;
pub const SIDEBAR_PADDING: [f32; 2] = [8.0, 12.0];
pub const SIDEBAR_HEADER_SPACING: f32 = 2.0;
pub const SIDEBAR_ITEM_SPACING: f32 = 1.0;

pub const CARD_SPACING: f32 = 4.0;
pub const SECTION_SPACING: f32 = 12.0;
pub const FORM_FIELD_SPACING: f32 = 4.0;
pub const CHIP_ROW_SPACING: f32 = 4.0;

pub const BUTTON_PADDING_SM: [f32; 2] = [4.0, 8.0];
pub const BUTTON_PADDING_MD: [f32; 2] = [6.0, 12.0];
pub const INPUT_PADDING: [f32; 2] = [5.0, 8.0];
pub const BANNER_PADDING: [f32; 2] = [6.0, 10.0];
pub const ROW_ITEM_PADDING: [f32; 2] = [5.0, 8.0];
pub const EMPTY_STATE_PADDING: f32 = 20.0;
pub const PREVIEW_BOX_PADDING: f32 = 12.0;

pub const SPACE_SM: f32 = 4.0;
pub const SPACE_MD: f32 = 6.0;
pub const SPACE_LG: f32 = 10.0;

pub const TOGGLER_SIZE: f32 = 14.0;
pub const SETTING_DROPDOWN_WIDTH: f32 = 250.0;
pub const SETTING_SLIDER_WIDTH: f32 = 160.0;

pub struct SettingsContext<'a> {
    pub theme_names: &'a [String],
    pub active_theme_name: &'a str,
    pub layout_names: &'a [String],
    pub active_layout_name: &'a str,
    pub custom_titlebar: bool,
    pub minimize_to_tray: bool,
    pub albums_view_mode: ViewMode,
    pub artists_view_mode: ViewMode,
    pub page_size: usize,
    pub album_tile_width: u32,
    pub artist_tile_width: u32,
    pub lyrics_font: Option<&'a str>,
    pub lyrics_font_size: u16,
    pub primary_font: Option<&'a str>,
    pub total_tracks: usize,
    pub total_albums: usize,
    pub total_artists: usize,
    pub db_size_bytes: u64,
    pub cache_file_count: usize,
    pub cache_size_bytes: u64,
    pub is_scanning: bool,

    pub listenbrainz_enabled: bool,
    pub listenbrainz_token: &'a str,
    pub listenbrainz_api_url: &'a str,

    pub lastfm_enabled: bool,
    pub lastfm_api_key: &'a str,
    pub lastfm_api_secret: &'a str,
    pub lastfm_session_key: &'a str,
    pub lastfm_api_url: &'a str,

    pub submit_now_playing: bool,
    pub scrobble_percent: u32,
    pub gapless: bool,
    pub resampling: audio::ResamplingConfig,
    pub watch_library: bool,
    pub volume: f32,
    pub is_shuffle: bool,
    pub is_repeat: bool,
}

/// Interactive sample state for the Debug settings page.
#[derive(Debug, Clone)]
pub struct DebugSampleState {
    pub toggler: bool,
    pub slider_val: f32,
    pub input_val: String,
    pub selected_option: String,
    pub active_chip: usize,
    pub view_mode: ViewMode,
    pub sort_field: SortField,
    pub sort_dir: SortDirection,
}

impl Default for DebugSampleState {
    fn default() -> Self {
        Self {
            toggler: true,
            slider_val: 0.65,
            input_val: String::new(),
            selected_option: "Option A".to_string(),
            active_chip: 0,
            view_mode: ViewMode::List,
            sort_field: SortField::Title,
            sort_dir: SortDirection::Ascending,
        }
    }
}

/// Self-contained state and view component for the Settings preferences screen.
pub struct SettingsView {
    pub active_tab: SettingsTab,
    pub available_fonts: Vec<String>,
    pub library_folders: Vec<String>,
    pub scrobble_test_status: Option<(plugin::scrobbler::ScrobbleService, Result<String, String>)>,
    pub fonts_loaded: bool,
    pub update_status: updater::UpdateStatus,
    pub show_debug_subpage: bool,
    pub debug_state: DebugSampleState,
}

impl SettingsView {
    pub fn new(active_tab: SettingsTab, mut available_fonts: Vec<String>) -> Self {
        let fonts_loaded = !available_fonts.is_empty();
        if !available_fonts.iter().any(|f| f == "Default") {
            available_fonts.insert(0, "Default".to_string());
        }
        Self {
            active_tab,
            available_fonts,
            library_folders: Vec::new(),
            scrobble_test_status: None,
            fonts_loaded,
            update_status: updater::UpdateStatus::Idle,
            show_debug_subpage: false,
            debug_state: DebugSampleState::default(),
        }
    }

    pub fn set_available_fonts(&mut self, mut fonts: Vec<String>) {
        if !fonts.iter().any(|f| f == "Default") {
            fonts.insert(0, "Default".to_string());
        }
        self.available_fonts = fonts;
        self.fonts_loaded = true;
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ctx: SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let sidebar = self.view_sidebar(theme_cfg, font);

        let content = match self.active_tab {
            SettingsTab::Appearance if self.show_debug_subpage => self.view_debug(theme_cfg, font),
            SettingsTab::Appearance => self.view_appearance(theme_cfg, font, &ctx),
            SettingsTab::Playback => self.view_playback(theme_cfg, font, &ctx),
            SettingsTab::Library => self.view_library(theme_cfg, font, &ctx),
            SettingsTab::Scrobbling => self.view_scrobbling(theme_cfg, font, &ctx),
        };

        let main_panel = scrollable(content)
            .id(iced::widget::Id::new("settings_scroll"))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg));

        row![sidebar, main_panel]
            .width(Length::Fill)
            .height(Length::Fill)
            .boxed()
    }
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::ViewMode;
    use theme::{ThemeConfig, DEFAULT_THEME_NAME};

    #[test]
    fn test_settings_tab_variants_and_defaults() {
        let default_tab = SettingsTab::default();
        assert_eq!(default_tab, SettingsTab::Appearance);

        let tabs = [
            SettingsTab::Appearance,
            SettingsTab::Playback,
            SettingsTab::Library,
            SettingsTab::Scrobbling,
        ];
        assert_ne!(tabs[0], tabs[1]);
        assert_ne!(tabs[1], tabs[2]);
        assert_ne!(tabs[2], tabs[3]);
    }

    #[test]
    fn test_settings_views_render_without_panicking() {
        let theme_cfg = ThemeConfig::default();
        let folders = vec!["/music/ambient".to_string(), "/music/jazz".to_string()];
        let available_fonts = vec!["Default".to_string(), "Noto Sans CJK JP".to_string()];

        let tabs = vec![
            (SettingsTab::Appearance, false),
            (SettingsTab::Appearance, true),
            (SettingsTab::Playback, false),
            (SettingsTab::Library, false),
            (SettingsTab::Scrobbling, false),
        ];

        for (tab, show_debug_subpage) in tabs {
            let settings_view = SettingsView {
                active_tab: tab,
                available_fonts: available_fonts.clone(),
                library_folders: folders.clone(),
                scrobble_test_status: None,
                fonts_loaded: true,
                update_status: updater::UpdateStatus::Idle,
                show_debug_subpage,
                debug_state: DebugSampleState::default(),
            };

            let theme_names_vec = vec![DEFAULT_THEME_NAME.to_string(), "Nord".to_string()];
            let layout_names_vec = vec!["Default".to_string(), "Sidebar".to_string()];
            let ctx = SettingsContext {
                theme_names: &theme_names_vec,
                active_theme_name: DEFAULT_THEME_NAME,
                layout_names: &layout_names_vec,
                active_layout_name: "Default",
                custom_titlebar: false,
                minimize_to_tray: false,
                albums_view_mode: ViewMode::Tile,
                artists_view_mode: ViewMode::Tile,
                page_size: 50,
                album_tile_width: 170,
                artist_tile_width: 160,
                lyrics_font: Some("Noto Serif"),
                lyrics_font_size: 16,
                primary_font: Some("Noto Sans CJK JP"),
                total_tracks: 120,
                total_albums: 15,
                total_artists: 8,
                db_size_bytes: 2048576,
                cache_file_count: 42,
                cache_size_bytes: 524288,
                is_scanning: false,

                listenbrainz_enabled: true,
                listenbrainz_token: "test_token",
                listenbrainz_api_url: "https://api.listenbrainz.org/1/submit-listens",

                lastfm_enabled: true,
                lastfm_api_key: "test_api_key",
                lastfm_api_secret: "test_secret",
                lastfm_session_key: "test_sk",
                lastfm_api_url: "https://ws.audioscrobbler.com/2.0/",

                submit_now_playing: true,
                scrobble_percent: 50,
                gapless: true,
                resampling: audio::ResamplingConfig::default(),
                watch_library: true,
                volume: 1.0,
                is_shuffle: false,
                is_repeat: false,
            };

            // Render each view and verify it builds valid Iced elements without panic
            let _element = settings_view.view(&theme_cfg, iced::Font::DEFAULT, ctx);
        }
    }

    #[test]
    fn test_settings_lazy_fonts() {
        let mut view = SettingsView::new(SettingsTab::Appearance, Vec::new());
        assert!(!view.fonts_loaded);
        assert_eq!(view.available_fonts, vec!["Default".to_string()]);

        view.set_available_fonts(vec!["Inter".to_string(), "Roboto".to_string()]);
        assert!(view.fonts_loaded);
        assert_eq!(
            view.available_fonts,
            vec![
                "Default".to_string(),
                "Inter".to_string(),
                "Roboto".to_string()
            ]
        );
    }

    #[test]
    fn test_playback_settings_with_custom_resampling() {
        let theme_cfg = ThemeConfig::default();
        let settings_view = SettingsView {
            active_tab: SettingsTab::Playback,
            available_fonts: vec!["Default".to_string()],
            library_folders: Vec::new(),
            scrobble_test_status: None,
            fonts_loaded: true,
            update_status: updater::UpdateStatus::Idle,
            show_debug_subpage: false,
            debug_state: DebugSampleState::default(),
        };
        let theme_names_vec = vec![DEFAULT_THEME_NAME.to_string()];
        let layout_names_vec = vec!["Default".to_string()];

        let custom_resampling = audio::ResamplingConfig {
            filter: audio::ResamplingFilter::BlackmanNuttall,
            quality: audio::ResamplingQuality::Ultra,
            linear_interp: false,
        };

        let ctx = SettingsContext {
            theme_names: &theme_names_vec,
            active_theme_name: DEFAULT_THEME_NAME,
            layout_names: &layout_names_vec,
            active_layout_name: "Default",
            custom_titlebar: false,
            minimize_to_tray: false,
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            page_size: 50,
            album_tile_width: 170,
            artist_tile_width: 160,
            lyrics_font: None,
            lyrics_font_size: 15,
            primary_font: None,
            total_tracks: 10,
            total_albums: 2,
            total_artists: 1,
            db_size_bytes: 1024,
            cache_file_count: 0,
            cache_size_bytes: 0,
            is_scanning: false,
            listenbrainz_enabled: false,
            listenbrainz_token: "",
            listenbrainz_api_url: "",
            lastfm_enabled: false,
            lastfm_api_key: "",
            lastfm_api_secret: "",
            lastfm_session_key: "",
            lastfm_api_url: "",
            submit_now_playing: false,
            scrobble_percent: 50,
            gapless: true,
            resampling: custom_resampling,
            watch_library: false,
            volume: 1.0,
            is_shuffle: false,
            is_repeat: false,
        };

        let _element = settings_view.view(&theme_cfg, iced::Font::DEFAULT, ctx);
    }

    #[test]
    fn test_settings_sidebar_update_status_rendering() {
        let theme_cfg = ThemeConfig::default();
        let mut settings_view = SettingsView::new(SettingsTab::Appearance, Vec::new());

        // Test Available status
        let update_info = updater::UpdateInfo {
            version: "v9.9.9".to_string(),
            release_url: env!("CARGO_PKG_REPOSITORY").to_string(),
        };
        settings_view.update_status = updater::UpdateStatus::Available(update_info.clone());
        drop(settings_view.view_sidebar(&theme_cfg, iced::Font::DEFAULT));

        // Test Downloading status
        settings_view.update_status = updater::UpdateStatus::Downloading {
            info: update_info.clone(),
            percent: 45,
        };
        drop(settings_view.view_sidebar(&theme_cfg, iced::Font::DEFAULT));

        // Test ReadyToRestart status
        settings_view.update_status = updater::UpdateStatus::ReadyToRestart { info: update_info };
        drop(settings_view.view_sidebar(&theme_cfg, iced::Font::DEFAULT));

        // Test Checking status
        settings_view.update_status = updater::UpdateStatus::Checking;
        drop(settings_view.view_sidebar(&theme_cfg, iced::Font::DEFAULT));

        // Test UpToDate status
        settings_view.update_status = updater::UpdateStatus::UpToDate;
        drop(settings_view.view_sidebar(&theme_cfg, iced::Font::DEFAULT));

        // Test Failed status
        settings_view.update_status = updater::UpdateStatus::Failed("Network error".to_string());
        drop(settings_view.view_sidebar(&theme_cfg, iced::Font::DEFAULT));
    }
}
