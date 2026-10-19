use std::path::PathBuf;

use iced::border::Radius;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, slider, text_input, toggler, Space,
    Widget,
};
use iced::{Alignment, Border, Color, Element, Length};

use crate::components::{self, border_hori, text};
use crate::icons;
use crate::message::{Message, SettingsTab, ViewMode};
use crate::theme::{styles, ThemeConfig};

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
pub const SETTING_LABEL_WIDTH: f32 = 160.0;

pub struct SettingsContext<'a> {
    pub theme_names: &'a [String],
    pub active_theme_name: &'a str,
    pub layout_names: &'a [String],
    pub active_layout_name: &'a str,
    pub albums_view_mode: ViewMode,
    pub artists_view_mode: ViewMode,
    pub page_size: usize,
    pub album_tile_width: u32,
    pub artist_tile_width: u32,
    pub now_playing_cover_size: u32,
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
    pub watch_library: bool,
    pub volume: f32,
    pub is_shuffle: bool,
    pub is_repeat: bool,
}

/// Self-contained state and view component for the Settings preferences screen.
pub struct SettingsView {
    pub active_tab: SettingsTab,
    pub available_fonts: Vec<String>,
    pub library_folders: Vec<String>,
    pub scrobble_test_status: Option<(plugins::scrobbler::ScrobbleService, Result<String, String>)>,
    pub fonts_loaded: bool,
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
        }
    }

    pub fn set_available_fonts(&mut self, mut fonts: Vec<String>) {
        if !fonts.iter().any(|f| f == "Default") {
            fonts.insert(0, "Default".to_string());
        }
        self.available_fonts = fonts;
        self.fonts_loaded = true;
    }

    pub fn select_tab(&mut self, tab: SettingsTab) {
        self.active_tab = tab;
    }

    pub fn set_library_folders(&mut self, folders: Vec<String>) {
        self.library_folders = folders;
    }

    pub fn set_scrobble_test_status(
        &mut self,
        status: Option<(plugins::scrobbler::ScrobbleService, Result<String, String>)>,
    ) {
        self.scrobble_test_status = status;
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let sidebar = self.view_sidebar(theme_cfg);

        let content = match self.active_tab {
            SettingsTab::Appearance => self.view_appearance(theme_cfg, &ctx),
            SettingsTab::Playback => self.view_playback(theme_cfg, &ctx),
            SettingsTab::Library => self.view_library(theme_cfg, &ctx),
            SettingsTab::Scrobbling => self.view_scrobbling(theme_cfg, &ctx),
        };

        let main_panel = scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg));

        row![sidebar, main_panel]
            .width(Length::Fill)
            .height(Length::Fill)
            .boxed()
    }

    fn view_sidebar<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let sidebar_title = column![
            text(theme_cfg, "Preferences")
                .size(11)
                .color(theme_cfg.primary_color()),
            text(theme_cfg, "System configuration")
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(SIDEBAR_HEADER_SPACING)
        .padding(SIDEBAR_PADDING);

        let nav_item = |tab: SettingsTab,
                        icon_svg: &'static str,
                        title: &'static str,
                        subtitle: &'static str| {
            let is_active = self.active_tab == tab;
            let color = if is_active {
                theme_cfg.primary_color()
            } else {
                theme_cfg.text_color()
            };

            button(
                row![
                    icons::icon(icon_svg, 16.0, color),
                    Space::new().width(SPACE_LG),
                    column![
                        text(theme_cfg, title).size(13).color(color),
                        text(theme_cfg, subtitle)
                            .size(10)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .spacing(SIDEBAR_ITEM_SPACING),
                ]
                .align_y(Alignment::Center),
            )
            .padding(SIDEBAR_PADDING)
            .width(Length::Fill)
            .style(styles::settings_nav_button(theme_cfg, is_active))
            .on_press(Message::SelectSettingsTab(tab))
        };

        container(row![
            column![
                sidebar_title,
                nav_item(
                    SettingsTab::Appearance,
                    icons::PALETTE_SVG,
                    "Appearance",
                    "Theme, views, fonts",
                ),
                nav_item(
                    SettingsTab::Playback,
                    icons::PLAY_CIRCLE_SVG,
                    "Playback",
                    "Audio engine & transitions",
                ),
                nav_item(
                    SettingsTab::Library,
                    icons::DATABASE_SVG,
                    "Library",
                    "Folders, indexes & cache",
                ),
                nav_item(
                    SettingsTab::Scrobbling,
                    icons::RADIO_SVG,
                    "Scrobbling",
                    "ListenBrainz & Last.fm",
                )
            ]
            .width(SIDEBAR_WIDTH),
            border_hori(1.0, theme_cfg.border_color())
        ])
        .width(Length::Fixed(SIDEBAR_WIDTH + 1.0))
        .height(Length::Fill)
        .style(styles::sidebar_container(theme_cfg))
        .boxed()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 1. Appearance Section
    // ─────────────────────────────────────────────────────────────────────────────

    fn view_appearance<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(theme_cfg, "Appearance & interface", Some("Customize color palette, typography font family, default display modes, and layout density"));

        // Theme chips
        let theme_buttons = {
            let mut row_wrap = row![].spacing(CHIP_ROW_SPACING);
            for name in ctx.theme_names {
                let is_active = name == ctx.active_theme_name;
                let icon_svg = if is_active {
                    icons::CHECK_SVG
                } else {
                    icons::DISC_SVG
                };
                let btn = components::chip_with_icon(
                    theme_cfg,
                    icon_svg,
                    name.clone(),
                    is_active,
                    Message::SelectTheme(name.clone()),
                );
                row_wrap = row_wrap.push(btn);
            }
            row_wrap.wrap()
        };

        let palette_preview = row![
            swatch_card(theme_cfg, "Background", theme_cfg.bg_color()),
            swatch_card(theme_cfg, "Surface", theme_cfg.surface_color()),
            swatch_card(theme_cfg, "Surface hover", theme_cfg.surface_hover_color()),
            swatch_card(theme_cfg, "Border", theme_cfg.border_color()),
            swatch_card(theme_cfg, "Primary", theme_cfg.primary_color()),
            swatch_card(theme_cfg, "Accent", theme_cfg.accent_color()),
            swatch_card(theme_cfg, "Text", theme_cfg.text_color()),
            swatch_card(theme_cfg, "Text muted", theme_cfg.text_muted_color()),
            swatch_card(theme_cfg, "Success", theme_cfg.success_color()),
            swatch_card(theme_cfg, "Danger", theme_cfg.danger_color()),
            swatch_card(theme_cfg, "Player bar", theme_cfg.player_bar_bg_color()),
            swatch_card(
                theme_cfg,
                "Player bar text",
                theme_cfg.player_bar_text_color()
            ),
            swatch_card(theme_cfg, "Slider active", theme_cfg.slider_active_color()),
            swatch_card(theme_cfg, "Slider rail", theme_cfg.slider_rail_color()),
        ]
        .spacing(CARD_SPACING)
        .wrap();

        let reload_themes_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text_color()),
                Space::new().width(SPACE_MD),
                text(theme_cfg, "Reload themes from disk")
                    .size(12)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ReloadThemes);

        let radius_item = |label: &'static str, val: f32, r: Radius| {
            let border = theme_cfg.border_color();
            column![
                container(Space::new().width(48).height(24)).style(move |_theme| {
                    container::Style {
                        background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                        border: Border {
                            radius: r,
                            color: border,
                            width: 1.0,
                        },
                        ..Default::default()
                    }
                }),
                text(theme_cfg, format!("{label} ({}px)", val as u32))
                    .size(10)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING)
            .align_x(Alignment::Center)
            .boxed()
        };

        let radius_preview = row![
            radius_item("xs", theme_cfg.border_radius.xs, theme_cfg.radius_xs()),
            radius_item("sm", theme_cfg.border_radius.sm, theme_cfg.radius_sm()),
            radius_item("md", theme_cfg.border_radius.md, theme_cfg.radius_md()),
            radius_item("lg", theme_cfg.border_radius.lg, theme_cfg.radius_lg()),
            radius_item("xl", theme_cfg.border_radius.xl, theme_cfg.radius_xl()),
        ]
        .spacing(CARD_SPACING)
        .wrap();

        let themes_card = components::settings_card(
            theme_cfg,
            "Theme & color palette",
            column![
                text(theme_cfg, "Installed theme")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                theme_buttons,
                Space::new().height(SPACE_MD),
                text(theme_cfg, "Current palette swatches")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                palette_preview,
                Space::new().height(SPACE_MD),
                text(theme_cfg, "Border radius variants")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                radius_preview,
                Space::new().height(SPACE_SM),
                reload_themes_btn,
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        // Window Layout Card
        let layout_buttons = {
            let mut row_wrap = row![].spacing(CHIP_ROW_SPACING);
            for name in ctx.layout_names {
                let is_active = name == ctx.active_layout_name;
                let icon_svg = if is_active {
                    icons::CHECK_SVG
                } else {
                    icons::ALBUM_SVG
                };
                let btn = components::chip_with_icon(
                    theme_cfg,
                    icon_svg,
                    name.clone(),
                    is_active,
                    Message::SelectLayout(name.clone()),
                );
                row_wrap = row_wrap.push(btn);
            }
            row_wrap.wrap()
        };

        let reload_layouts_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text_color()),
                Space::new().width(SPACE_MD),
                text(theme_cfg, "Reload layouts from disk")
                    .size(12)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ReloadLayouts);

        let layout_card = components::settings_card(
            theme_cfg,
            "Layout",
            column![
                text(theme_cfg, "Select a layout preset")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                layout_buttons,
                Space::new().height(SPACE_SM),
                reload_layouts_btn,
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        // Typography & Font Configuration Card
        let active_font_family = ctx.primary_font.unwrap_or("Default").to_string();

        // Font dropdown selector
        let mut font_options = self.available_fonts.clone();
        if !font_options.iter().any(|f| f == "Default") {
            font_options.insert(0, "Default".to_string());
        }
        if let Some(active) = ctx.primary_font {
            if !font_options.iter().any(|f| f == active) {
                font_options.push(active.to_string());
            }
        }

        let font_selector = pick_list(
            Some(active_font_family.clone()),
            font_options,
            |option: &String| option.clone(),
        )
        .on_select(Message::SelectFont)
        .placeholder("Select a system font...")
        .font(theme_cfg.font())
        .padding(INPUT_PADDING)
        .text_size(12)
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fill)
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let active_iced_font = theme_cfg.font();

        let sample_text_box = container(
            column![
                text(theme_cfg, "Multilingual glyph rendering preview")
                    .size(11)
                    .color(theme_cfg.primary_color()),
                Space::new().height(SPACE_SM),
                text(
                    theme_cfg,
                    "English: The quick brown fox jumps over the lazy dog."
                )
                .size(12)
                .color(theme_cfg.text_color())
                .font(active_iced_font),
                text(
                    theme_cfg,
                    "日本語: あのイーハトーヴォのすきとおった風、夏でも底に冷たさをもつ青いそら"
                )
                .size(12)
                .color(theme_cfg.text_color())
                .font(active_iced_font),
                text(theme_cfg, "中文: 落霞与孤齐飞，秋水共长天一色")
                    .size(12)
                    .color(theme_cfg.text_color())
                    .font(active_iced_font),
                text(
                    theme_cfg,
                    "한국어: 별 하나에 추억과 별 하나에 사랑과 쓸쓸함과 동경"
                )
                .size(12)
                .color(theme_cfg.text_color())
                .font(active_iced_font),
            ]
            .spacing(FORM_FIELD_SPACING),
        )
        .padding(PREVIEW_BOX_PADDING)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface_color())),
            border: Border {
                radius: theme_cfg.radius_sm(),
                color: theme_cfg.border_color(),
                width: 1.0,
            },
            ..Default::default()
        });

        let font_card = components::settings_card(
            theme_cfg,
            "Typography & font configuration",
            column![
                column![
                    text(theme_cfg, "Select font family")
                        .size(12)
                        .color(theme_cfg.text_muted_color()),
                    font_selector,
                ]
                .spacing(FORM_FIELD_SPACING),
                sample_text_box,
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        // 1. Library Tab Section
        let page_sizes = [25, 50, 100, 200];
        let page_size_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &size in &page_sizes {
                let is_sel = ctx.page_size == size;
                let btn = components::chip(
                    theme_cfg,
                    format!("{} tracks", size),
                    is_sel,
                    Message::SetPageSize(size),
                );
                r = r.push(btn);
            }
            r
        };

        let library_tab_card = components::settings_card(
            theme_cfg,
            "Library tab",
            row![
                text(theme_cfg, "Page limit")
                    .size(12)
                    .color(theme_cfg.text_color())
                    .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                page_size_buttons,
            ]
            .align_y(Alignment::Center)
            .boxed(),
        );

        // 2. Albums Tab Section
        let album_tile_presets = [140, 170, 210, 260];
        let album_preset_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &width in &album_tile_presets {
                let is_sel = ctx.album_tile_width == width;
                let btn = components::chip(
                    theme_cfg,
                    format!("{} px", width),
                    is_sel,
                    Message::SetAlbumTileWidth(width),
                );
                r = r.push(btn);
            }
            r
        };

        let albums_tab_card = components::settings_card(
            theme_cfg,
            "Albums tab",
            column![
                row![
                    text(theme_cfg, "Catalog view mode")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                    components::view_mode_toggle(
                        theme_cfg,
                        ctx.albums_view_mode,
                        Message::SetAlbumsViewMode,
                    ),
                ]
                .align_y(Alignment::Center),
                row![
                    text(theme_cfg, "Tile width")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                    row![
                        slider(120.0..=300.0, ctx.album_tile_width as f32, |val| {
                            Message::SetAlbumTileWidth(val.round() as u32)
                        },)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                        Space::new().width(SPACE_LG),
                        text(theme_cfg, format!("{} px", ctx.album_tile_width))
                            .size(12)
                            .color(theme_cfg.primary_color())
                            .width(Length::Fixed(55.0)),
                        album_preset_buttons,
                    ]
                    .align_y(Alignment::Center),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        // 3. Artists Tab Section
        let artist_tile_presets = [140, 160, 200, 240];
        let artist_preset_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &width in &artist_tile_presets {
                let is_sel = ctx.artist_tile_width == width;
                let btn = components::chip(
                    theme_cfg,
                    format!("{} px", width),
                    is_sel,
                    Message::SetArtistTileWidth(width),
                );
                r = r.push(btn);
            }
            r
        };

        let artists_tab_card = components::settings_card(
            theme_cfg,
            "Artists tab",
            column![
                row![
                    text(theme_cfg, "Catalog view mode")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                    components::view_mode_toggle(
                        theme_cfg,
                        ctx.artists_view_mode,
                        Message::SetArtistsViewMode,
                    ),
                ]
                .align_y(Alignment::Center),
                row![
                    text(theme_cfg, "Tile width")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                    row![
                        slider(120.0..=280.0, ctx.artist_tile_width as f32, |val| {
                            Message::SetArtistTileWidth(val.round() as u32)
                        },)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                        Space::new().width(SPACE_LG),
                        text(theme_cfg, format!("{} px", ctx.artist_tile_width))
                            .size(12)
                            .color(theme_cfg.primary_color())
                            .width(Length::Fixed(55.0)),
                        artist_preset_buttons,
                    ]
                    .align_y(Alignment::Center),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        // 4. Now Playing Section
        let now_playing_presets = [180, 220, 260, 300];
        let now_playing_preset_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &size in &now_playing_presets {
                let is_sel = ctx.now_playing_cover_size == size;
                let btn = components::chip(
                    theme_cfg,
                    format!("{} px", size),
                    is_sel,
                    Message::SetNowPlayingCoverSize(size),
                );
                r = r.push(btn);
            }
            r
        };

        let now_playing_card = components::settings_card(
            theme_cfg,
            "Now playing",
            column![
                row![
                    text(theme_cfg, "Cover art dimension")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                    row![
                        slider(160.0..=320.0, ctx.now_playing_cover_size as f32, |val| {
                            Message::SetNowPlayingCoverSize(val.round() as u32)
                        },)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                        Space::new().width(SPACE_LG),
                        text(theme_cfg, format!("{} px", ctx.now_playing_cover_size))
                            .size(12)
                            .color(theme_cfg.primary_color())
                            .width(Length::Fixed(55.0)),
                        now_playing_preset_buttons,
                    ]
                    .align_y(Alignment::Center),
                ]
                .align_y(Alignment::Center),
                row![
                    text(theme_cfg, "Now playing notifications")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(SETTING_LABEL_WIDTH)),
                    components::chip_with_icon(
                        theme_cfg,
                        if ctx.submit_now_playing {
                            icons::CHECK_SVG
                        } else {
                            icons::RADIO_SVG
                        },
                        if ctx.submit_now_playing {
                            "Enabled"
                        } else {
                            "Disabled"
                        },
                        ctx.submit_now_playing,
                        Message::ToggleSubmitNowPlaying(!ctx.submit_now_playing),
                    ),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        column![
            header,
            themes_card,
            layout_card,
            font_card,
            library_tab_card,
            albums_tab_card,
            artists_tab_card,
            now_playing_card,
            Space::new().height(PAGE_BOTTOM_SPACER),
        ]
        .spacing(PAGE_SPACING)
        .padding(PAGE_PADDING)
        .width(Length::Fill)
        .boxed()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 2. Playback Section
    // ─────────────────────────────────────────────────────────────────────────────

    fn view_playback<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(
            theme_cfg,
            "Playback preferences",
            Some("Configure audio transitions and playback behavior"),
        );

        // 1. Audio Transitions Card
        let gapless_row = row![
            column![
                text(theme_cfg, "Gapless audio playback")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(
                    theme_cfg,
                    "Preload upcoming tracks in memory for seamless, uninterrupted transitions",
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            toggler(ctx.gapless)
                .on_toggle(Message::ToggleGapless)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center);

        let transitions_card =
            components::settings_card(theme_cfg, "Audio transitions", column![gapless_row].boxed());

        // 2. Playback Behavior Card
        let volume_row = row![
            column![
                text(theme_cfg, "Master output volume")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(
                    theme_cfg,
                    "Default master volume multiplier applied to audio stream playback",
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            slider(0.0..=1.0, ctx.volume, Message::VolumeChanged)
                .style(styles::settings_slider_style(theme_cfg))
                .width(Length::Fixed(SETTING_LABEL_WIDTH)),
        ]
        .align_y(Alignment::Center);

        let shuffle_row = row![
            column![
                text(theme_cfg, "Shuffle playback")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(
                    theme_cfg,
                    "Randomize track selection and playback order within active queue",
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            toggler(ctx.is_shuffle)
                .on_toggle(|_| Message::ToggleShuffle)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center);

        let repeat_row = row![
            column![
                text(theme_cfg, "Repeat playback")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(
                    theme_cfg,
                    "Loop the active queue continuously when the final track completes",
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            toggler(ctx.is_repeat)
                .on_toggle(|_| Message::ToggleRepeat)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center);

        let behavior_card = components::settings_card(
            theme_cfg,
            "Playback behavior",
            column![volume_row, shuffle_row, repeat_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        );

        column![
            header,
            transitions_card,
            behavior_card,
            Space::new().height(PAGE_BOTTOM_SPACER),
        ]
        .spacing(PAGE_SPACING)
        .padding(PAGE_PADDING)
        .width(Length::Fill)
        .boxed()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 3. Library Section
    // ─────────────────────────────────────────────────────────────────────────────

    fn view_library<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(theme_cfg, "Library & storage", Some("Manage indexed audio directories, rescan metadata, monitor disk caches, or reset storage"));

        // Storage and Index Metrics
        let storage_stats_row1 = row![
            components::metric_box(theme_cfg, "Songs indexed", format!("{}", ctx.total_tracks)),
            components::metric_box(
                theme_cfg,
                "Albums cataloged",
                format!("{}", ctx.total_albums)
            ),
            components::metric_box(
                theme_cfg,
                "Artists cataloged",
                format!("{}", ctx.total_artists)
            ),
        ]
        .spacing(CHIP_ROW_SPACING);

        let storage_stats_row2 = row![
            components::metric_box(
                theme_cfg,
                "Database file size",
                format_file_size(ctx.db_size_bytes),
            ),
            components::metric_box(
                theme_cfg,
                "Disk thumbnail cache",
                format!(
                    "{} files ({})",
                    ctx.cache_file_count,
                    format_file_size(ctx.cache_size_bytes)
                ),
            ),
        ]
        .spacing(CHIP_ROW_SPACING);

        let stats_card = components::settings_card(
            theme_cfg,
            "Storage & index metrics",
            column![storage_stats_row1, storage_stats_row2]
                .spacing(CARD_SPACING)
                .boxed(),
        );

        let add_folder_btn = button(
            row![
                icons::icon(icons::PLUS_SVG, 13.0, theme_cfg.primary_color()),
                Space::new().width(SPACE_MD),
                text(
                    theme_cfg,
                    if ctx.is_scanning {
                        "Scanning library..."
                    } else {
                        "Add music folder"
                    }
                )
                .size(12)
                .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::AddLibraryFolderClicked)
        });

        let folders_content: Element<'a, Message> = if self.library_folders.is_empty() {
            container(
                column![
                    text(theme_cfg, "No music directories configured yet.")
                        .size(13)
                        .color(theme_cfg.text_muted_color()),
                    text(
                        theme_cfg,
                        "Click 'Add music folder' to choose a directory containing audio files."
                    )
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
                ]
                .spacing(FORM_FIELD_SPACING)
                .align_x(Alignment::Center),
            )
            .padding(EMPTY_STATE_PADDING)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface_color())),
                border: Border {
                    radius: theme_cfg.radius_sm(),
                    color: theme_cfg.border_color(),
                    width: 1.0,
                },
                ..Default::default()
            })
            .boxed()
        } else {
            let mut col = column![].spacing(CHIP_ROW_SPACING);
            for folder_path in &self.library_folders {
                let path_clone = folder_path.clone();
                let path_buf = PathBuf::from(folder_path);

                let rescan_btn = button(icons::icon(
                    icons::REFRESH_SVG,
                    12.0,
                    theme_cfg.text_color(),
                ))
                .padding(BUTTON_PADDING_SM)
                .style(styles::ghost_button(theme_cfg, false))
                .on_press_maybe(if ctx.is_scanning {
                    None
                } else {
                    Some(Message::RescanLibraryFolder(path_buf))
                });

                let remove_btn = button(icons::icon(
                    icons::TRASH_SVG,
                    12.0,
                    theme_cfg.danger_color(),
                ))
                .padding(BUTTON_PADDING_SM)
                .style(styles::ghost_button(theme_cfg, false))
                .on_press_maybe(if ctx.is_scanning {
                    None
                } else {
                    Some(Message::RemoveLibraryFolder(path_clone))
                });

                let item_row = container(
                    row![
                        icons::icon(icons::FOLDER_SVG, 15.0, theme_cfg.primary_color()),
                        Space::new().width(SPACE_LG),
                        text(theme_cfg, folder_path.as_str())
                            .size(12)
                            .color(theme_cfg.text_color())
                            .width(Length::Fill),
                        rescan_btn,
                        Space::new().width(SPACE_MD),
                        remove_btn,
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(ROW_ITEM_PADDING)
                .width(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(iced::Background::Color(theme_cfg.surface_color())),
                    border: Border {
                        radius: theme_cfg.radius_sm(),
                        color: theme_cfg.border_color(),
                        width: 1.0,
                    },
                    ..Default::default()
                });

                col = col.push(item_row.boxed());
            }
            col.boxed()
        };

        let watch_dirs_row = row![
            column![
                text(theme_cfg, "Watch library folders")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text(
                    theme_cfg,
                    "Automatically monitor directories and scan for new audio files"
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .width(Length::Fill),
            components::chip_with_icon(
                theme_cfg,
                if ctx.watch_library {
                    icons::CHECK_SVG
                } else {
                    icons::RADIO_SVG
                },
                if ctx.watch_library {
                    "Enabled"
                } else {
                    "Disabled"
                },
                ctx.watch_library,
                Message::ToggleWatchLibrary(!ctx.watch_library),
            ),
        ]
        .align_y(Alignment::Center);

        let folders_card = components::settings_card(
            theme_cfg,
            "Scanned directories",
            column![watch_dirs_row, folders_content, add_folder_btn]
                .spacing(CARD_SPACING)
                .boxed(),
        );

        // Maintenance & Actions Card
        let scan_new_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.primary_color()),
                Space::new().width(SPACE_MD),
                text(
                    theme_cfg,
                    if ctx.is_scanning {
                        "Scanning new files..."
                    } else {
                        "Scan for new files"
                    }
                )
                .size(12)
                .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ScanLibraryForNewFiles)
        });

        let reindex_all_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.primary_color()),
                Space::new().width(SPACE_MD),
                text(
                    theme_cfg,
                    if ctx.is_scanning {
                        "Reindexing library..."
                    } else {
                        "Reindex entire library"
                    }
                )
                .size(12)
                .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ReindexLibrary)
        });

        let clear_cache_btn = button(
            row![
                icons::icon(icons::TRASH_SVG, 13.0, theme_cfg.text_color()),
                Space::new().width(SPACE_MD),
                text(theme_cfg, "Clear thumbnail cache")
                    .size(12)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ClearThumbnailCache);

        let clear_db_btn = button(
            row![
                icons::icon(icons::TRASH_SVG, 13.0, theme_cfg.danger_color()),
                Space::new().width(SPACE_MD),
                text(theme_cfg, "Delete library database")
                    .size(12)
                    .color(theme_cfg.danger_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::danger_button(theme_cfg))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ClearLibrary)
        });

        let actions_card = components::settings_card(
            theme_cfg,
            "Library actions & maintenance",
            column![
                // Action 1: Scan for new files
                row![
                    column![
                        text(theme_cfg, "Scan for new files")
                            .size(12)
                            .color(theme_cfg.text_muted_color()),
                        text(theme_cfg, "Quickly checks library folders and indexes newly added files without rescanning unchanged tracks.")
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .width(Length::Fill),
                    scan_new_btn,
                ]
                .align_y(Alignment::Center),
                Space::new().height(SPACE_MD),
                // Action 2: Reindex
                row![
                    column![
                        text(theme_cfg, "Full reindex")
                            .size(12)
                            .color(theme_cfg.text_muted_color()),
                        text(theme_cfg, "Scans all configured music folders, updates metadata tags, and repopulates the library.")
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .width(Length::Fill),
                    reindex_all_btn,
                ]
                .align_y(Alignment::Center),
                Space::new().height(SPACE_MD),
                // Action 2: Clear Thumbnail Cache
                row![
                    column![
                        text(theme_cfg, "Purge thumbnail cache")
                            .size(12)
                            .color(theme_cfg.text_muted_color()),
                        text(theme_cfg, format!(
                            "Deletes all {} cached thumbnail images ({}) from disk and evicts active decoded cover textures from memory. Thumbnails will regenerate when viewed.",
                            ctx.cache_file_count,
                            format_file_size(ctx.cache_size_bytes),
                        ))
                        .size(11)
                        .color(theme_cfg.text_muted_color()),
                    ]
                    .width(Length::Fill),
                    clear_cache_btn,
                ]
                .align_y(Alignment::Center),
                Space::new().height(SPACE_MD),
                // Action 3: Clear / Reset Database
                row![
                    column![
                        text(theme_cfg, "Reset library database")
                            .size(12)
                            .color(theme_cfg.danger_color()),
                        text(theme_cfg, "Clears all indexed tracks, albums, and artist metadata from database. Your audio files on disk remain untouched.")
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .width(Length::Fill),
                    clear_db_btn,
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(CARD_SPACING).boxed(),
        );

        column![
            header,
            stats_card,
            folders_card,
            actions_card,
            Space::new().height(PAGE_BOTTOM_SPACER),
        ]
        .spacing(PAGE_SPACING)
        .padding(PAGE_PADDING)
        .width(Length::Fill)
        .boxed()
    }

    fn view_scrobbling<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(theme_cfg, "Scrobbling & social audio", Some("Track your listening habits and broadcast live playback status to ListenBrainz and Last.fm"));

        // 1. General Preferences Card
        let prefs_card = components::settings_card(
            theme_cfg,
            "Scrobble preferences",
            column![
                text(theme_cfg, "Scrobble threshold")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                row![
                    components::chip(
                        theme_cfg,
                        "50% (standard)",
                        ctx.scrobble_percent == 50,
                        Message::SetScrobblePercent(50),
                    ),
                    components::chip(
                        theme_cfg,
                        "75%",
                        ctx.scrobble_percent == 75,
                        Message::SetScrobblePercent(75),
                    ),
                    components::chip(
                        theme_cfg,
                        "100% (complete)",
                        ctx.scrobble_percent == 100,
                        Message::SetScrobblePercent(100),
                    ),
                ]
                .spacing(CHIP_ROW_SPACING),
            ]
            .spacing(CARD_SPACING)
            .boxed(),
        );

        // 2. ListenBrainz Card
        let lb_test_banner = if let Some((plugins::scrobbler::ScrobbleService::ListenBrainz, res)) =
            self.scrobble_test_status.as_ref()
        {
            let (color, msg) = match res {
                Ok(ref m) => (theme_cfg.success_color(), m.as_str()),
                Err(ref e) => (theme_cfg.danger_color(), e.as_str()),
            };
            Some(
                container(text(theme_cfg, msg).size(12).color(color))
                    .padding(BANNER_PADDING)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            color.r, color.g, color.b, 0.1,
                        ))),
                        border: Border {
                            radius: theme_cfg.radius_sm(),
                            color,
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
            )
        } else {
            None
        };

        let lb_test_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.primary_color()),
                Space::new().width(SPACE_MD),
                text(theme_cfg, "Test connection")
                    .size(11)
                    .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::TestScrobble(
            plugins::scrobbler::ScrobbleService::ListenBrainz,
        ));

        let mut lb_column = column![
            row![
                text(theme_cfg, "Enable")
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(Length::Fill),
                toggler(ctx.listenbrainz_enabled)
                    .on_toggle(Message::ToggleListenBrainz)
                    .size(TOGGLER_SIZE)
                    .style(styles::toggler_style(theme_cfg)),
            ]
            .align_y(Alignment::Center),
            column![
                text(theme_cfg, "User token")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text_input("Enter ListenBrainz user token...", ctx.listenbrainz_token)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::ListenBrainzTokenChanged)
                    .width(Length::Fill),
                text(
                    theme_cfg,
                    "Obtain your user token from your profile at https://listenbrainz.org/profile/"
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(theme_cfg, "API endpoint URL")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text_input(
                    "https://api.listenbrainz.org/1/submit-listens",
                    ctx.listenbrainz_api_url,
                )
                .size(12)
                .padding(INPUT_PADDING)
                .style(styles::search_input_style(theme_cfg))
                .on_input(Message::ListenBrainzUrlChanged)
                .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            lb_test_btn,
        ]
        .spacing(SECTION_SPACING);

        if let Some(banner) = lb_test_banner {
            lb_column = lb_column.push(banner.boxed());
        }

        let listenbrainz_card =
            components::settings_card(theme_cfg, "ListenBrainz plugin", lb_column.boxed());

        // 3. Last.fm Card
        let lastfm_test_banner = if let Some((plugins::scrobbler::ScrobbleService::LastFm, res)) =
            self.scrobble_test_status.as_ref()
        {
            let (color, msg) = match res {
                Ok(ref m) => (theme_cfg.success_color(), m.as_str()),
                Err(ref e) => (theme_cfg.danger_color(), e.as_str()),
            };
            Some(
                container(text(theme_cfg, msg).size(12).color(color))
                    .padding(BANNER_PADDING)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            color.r, color.g, color.b, 0.1,
                        ))),
                        border: Border {
                            radius: theme_cfg.radius_sm(),
                            color,
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
            )
        } else {
            None
        };

        let lastfm_test_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.primary_color()),
                Space::new().width(SPACE_MD),
                text(theme_cfg, "Test connection")
                    .size(11)
                    .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::TestScrobble(
            plugins::scrobbler::ScrobbleService::LastFm,
        ));

        let mut lastfm_column = column![
            row![
                text(theme_cfg, "Enable")
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(Length::Fill),
                toggler(ctx.lastfm_enabled)
                    .on_toggle(Message::ToggleLastFm)
                    .size(TOGGLER_SIZE)
                    .style(styles::toggler_style(theme_cfg)),
            ]
            .align_y(Alignment::Center),
            column![
                text(theme_cfg, "API key")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text_input("Enter Last.fm API key...", ctx.lastfm_api_key)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmApiKeyChanged)
                    .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(theme_cfg, "Shared secret")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text_input("Enter Last.fm shared secret...", ctx.lastfm_api_secret)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmApiSecretChanged)
                    .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(theme_cfg, "Session key (sk)")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text_input("Enter Last.fm session key (sk)...", ctx.lastfm_session_key)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmSessionKeyChanged)
                    .width(Length::Fill),
                text(
                    theme_cfg,
                    "Obtain your API key and session key from your Last.fm API account settings"
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(theme_cfg, "API endpoint URL")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text_input("https://ws.audioscrobbler.com/2.0/", ctx.lastfm_api_url)
                    .size(12)
                    .padding(INPUT_PADDING)
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::LastFmUrlChanged)
                    .width(Length::Fill),
            ]
            .spacing(FORM_FIELD_SPACING),
            lastfm_test_btn,
        ]
        .spacing(SECTION_SPACING);

        if let Some(banner) = lastfm_test_banner {
            lastfm_column = lastfm_column.push(banner.boxed());
        }

        let lastfm_card =
            components::settings_card(theme_cfg, "Last.fm plugin", lastfm_column.boxed());

        column![
            header,
            prefs_card,
            listenbrainz_card,
            lastfm_card,
            Space::new().height(PAGE_BOTTOM_SPACER),
        ]
        .spacing(PAGE_SPACING)
        .padding(PAGE_PADDING)
        .width(Length::Fill)
        .boxed()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn swatch_card<'a>(
    theme_cfg: &'a ThemeConfig,
    label: &'static str,
    color: iced::Color,
) -> Element<'a, Message> {
    let border = theme_cfg.border_color();
    column![
        container(Space::new().width(52).height(22)).style(move |_theme| container::Style {
            background: Some(iced::Background::Color(color)),
            border: Border {
                radius: theme_cfg.radius_sm(),
                color: border,
                width: 1.0,
            },
            ..Default::default()
        }),
        text(theme_cfg, label)
            .size(10)
            .color(theme_cfg.text_muted_color()),
    ]
    .spacing(FORM_FIELD_SPACING)
    .align_x(Alignment::Center)
    .boxed()
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
