use std::path::PathBuf;

use iced::border::Radius;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, slider, text_input, Space, Widget,
};
use iced::{Alignment, Border, Color, Element, Length};

use crate::components::{self, border_hori, text};
use crate::icons;
use crate::message::{Message, SettingsMessage, SettingsTab, ViewMode};
use crate::theme::{styles, ThemeConfig};

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
    pub listenbrainz_status: String,
    pub listenbrainz_scrobbles: u64,

    pub lastfm_enabled: bool,
    pub lastfm_api_key: &'a str,
    pub lastfm_api_secret: &'a str,
    pub lastfm_session_key: &'a str,
    pub lastfm_api_url: &'a str,
    pub lastfm_status: String,
    pub lastfm_scrobbles: u64,

    pub submit_now_playing: bool,
    pub scrobble_percent: u32,
    pub gapless: bool,
}

/// Self-contained state and view component for the Settings preferences screen.
pub struct SettingsView {
    pub active_tab: SettingsTab,
    pub available_fonts: Vec<String>,
    pub custom_font_input: String,
    pub library_folders: Vec<String>,
    pub scrobble_test_status: Option<(plugins::scrobbler::ScrobbleService, Result<String, String>)>,
}

impl SettingsView {
    pub fn new(active_tab: SettingsTab, mut available_fonts: Vec<String>) -> Self {
        if !available_fonts.iter().any(|f| f == "Default") {
            available_fonts.insert(0, "Default".to_string());
        }
        Self {
            active_tab,
            available_fonts,
            custom_font_input: String::new(),
            library_folders: Vec::new(),
            scrobble_test_status: None,
        }
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::CustomFontInputChanged(input) => {
                self.custom_font_input = input;
            }
        }
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
            text(theme_cfg, "System Configuration")
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .padding([8, 12]);

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
                    Space::new().width(10),
                    column![
                        text(theme_cfg, title).size(13).color(color),
                        text(theme_cfg, subtitle)
                            .size(10)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .spacing(1),
                ]
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
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
                    "Theme, Views, Fonts",
                ),
                nav_item(
                    SettingsTab::Library,
                    icons::DATABASE_SVG,
                    "Library",
                    "Folders, Indexes & Cache",
                ),
                nav_item(
                    SettingsTab::Scrobbling,
                    icons::RADIO_SVG,
                    "Scrobbling",
                    "ListenBrainz & Last.fm",
                )
            ]
            .width(220.0),
            border_hori(1.0, theme_cfg.border_color())
        ])
        .width(Length::Fixed(221.0))
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
        let header = components::settings_header(theme_cfg, "Appearance & Interface", Some("Customize color palette, typography font family, default display modes, and layout density"));

        // Theme chips
        let theme_buttons = {
            let mut row_wrap = row![].spacing(6);
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
            swatch_card(theme_cfg, "Surface Hover", theme_cfg.surface_hover_color()),
            swatch_card(theme_cfg, "Border", theme_cfg.border_color()),
            swatch_card(theme_cfg, "Primary", theme_cfg.primary_color()),
            swatch_card(theme_cfg, "Accent", theme_cfg.accent_color()),
            swatch_card(theme_cfg, "Text", theme_cfg.text_color()),
            swatch_card(theme_cfg, "Text Muted", theme_cfg.text_muted_color()),
            swatch_card(theme_cfg, "Success", theme_cfg.success_color()),
            swatch_card(theme_cfg, "Danger", theme_cfg.danger_color()),
            swatch_card(theme_cfg, "Player Bar", theme_cfg.player_bar_bg_color()),
            swatch_card(
                theme_cfg,
                "Player Bar Text",
                theme_cfg.player_bar_text_color()
            ),
            swatch_card(theme_cfg, "Slider Active", theme_cfg.slider_active_color()),
            swatch_card(theme_cfg, "Slider Rail", theme_cfg.slider_rail_color()),
        ]
        .spacing(10)
        .wrap();

        let reload_themes_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text_color()),
                Space::new().width(6),
                text(theme_cfg, "Reload Themes from Disk")
                    .size(12)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ReloadThemes);

        let themes_card = components::settings_card(
            theme_cfg,
            "Theme & Color Palette",
            column![
                text(theme_cfg, "Installed theme")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                theme_buttons,
                Space::new().height(6),
                text(theme_cfg, "Current palette swatches")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                palette_preview,
                Space::new().height(4),
                reload_themes_btn,
            ]
            .spacing(8)
            .boxed(),
        );

        // Window Layout Card
        let layout_buttons = {
            let mut row_wrap = row![].spacing(6);
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
                Space::new().width(6),
                text(theme_cfg, "Reload Layouts from Disk")
                    .size(12)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
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
                Space::new().height(4),
                reload_layouts_btn,
            ]
            .spacing(8)
            .boxed(),
        );

        // Typography & Font Configuration Card
        let active_font_family = ctx.primary_font.unwrap_or("Default").to_string();

        let font_metrics = row![
            components::metric_box(theme_cfg, "Active Font Family", active_font_family.clone()),
            components::metric_box(
                theme_cfg,
                "Discovered Font Files",
                format!("{} files", self.available_fonts.len()),
            ),
        ]
        .spacing(12);

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
        .padding([8, 12])
        .text_size(12)
        .width(Length::Fill)
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        // Custom font input row
        let custom_font_trimmed = self.custom_font_input.trim().to_string();
        let has_custom_input = !custom_font_trimmed.is_empty();

        let apply_custom_font_btn = button(
            row![
                icons::icon(
                    icons::CHECK_SVG,
                    12.0,
                    if has_custom_input {
                        theme_cfg.primary_color()
                    } else {
                        theme_cfg.text_muted_color()
                    }
                ),
                Space::new().width(4),
                text(theme_cfg, "Apply Font")
                    .size(11)
                    .color(if has_custom_input {
                        theme_cfg.primary_color()
                    } else {
                        theme_cfg.text_muted_color()
                    }),
            ]
            .align_y(Alignment::Center),
        )
        .padding([5, 10])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if has_custom_input {
            Some(Message::SelectFont(custom_font_trimmed))
        } else {
            None
        });

        let custom_font_row = row![
            text_input(
                "Enter custom system font family (e.g. Inter, Roboto, Arial)...",
                &self.custom_font_input,
            )
            .size(12)
            .padding([5, 8])
            .style(styles::search_input_style(theme_cfg))
            .on_input(|s| Message::Settings(SettingsMessage::CustomFontInputChanged(s)))
            .on_submit(Message::SelectFont(
                self.custom_font_input.trim().to_string()
            ))
            .width(Length::Fill),
            Space::new().width(8),
            apply_custom_font_btn,
        ]
        .align_y(Alignment::Center);

        let active_iced_font = theme_cfg.font();

        let sample_text_box = container(
            column![
                text(theme_cfg, "Multilingual Glyph Rendering Preview:")
                    .size(11)
                    .color(theme_cfg.primary_color()),
                Space::new().height(4),
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
            .spacing(4),
        )
        .padding(12)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface_color())),
            border: Border {
                radius: Radius::from(4.0),
                color: theme_cfg.border_color(),
                width: 1.0,
            },
            ..Default::default()
        });

        let font_card = components::settings_card(
            theme_cfg,
            "Typography & Font Configuration",
            column![
                text(
                    theme_cfg,
                    "Select from discovered system/CJK fonts or specify any installed font family"
                )
                .size(12)
                .color(theme_cfg.text_muted_color()),
                font_metrics,
                Space::new().height(4),
                text(theme_cfg, "Select Font Family:")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                font_selector,
                Space::new().height(4),
                text(theme_cfg, "Or Specify Custom Font:")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                custom_font_row,
                Space::new().height(6),
                sample_text_box,
            ]
            .spacing(8)
            .boxed(),
        );

        // 1. Library Tab Section
        let page_sizes = [25, 50, 100, 200];
        let page_size_buttons = {
            let mut r = row![].spacing(6).align_y(Alignment::Center);
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
            "Library Tab",
            column![
                text(
                    theme_cfg,
                    "Preferences and viewport pagination density for the music library"
                )
                .size(12)
                .color(theme_cfg.text_muted_color()),
                row![
                    text(theme_cfg, "Page Limit:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    page_size_buttons,
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(10)
            .boxed(),
        );

        // 2. Albums Tab Section
        let album_tile_presets = [140, 170, 210, 260];
        let album_preset_buttons = {
            let mut r = row![].spacing(6).align_y(Alignment::Center);
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
            "Albums Tab",
            column![
                text(
                    theme_cfg,
                    "Layout presentation and tile dimensions for album catalog"
                )
                .size(12)
                .color(theme_cfg.text_muted_color()),
                row![
                    text(theme_cfg, "Catalog View Mode:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    components::view_mode_toggle(
                        theme_cfg,
                        ctx.albums_view_mode,
                        Message::SetAlbumsViewMode,
                    ),
                ]
                .align_y(Alignment::Center),
                row![
                    text(theme_cfg, "Tile Width:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    row![
                        slider(120.0..=300.0, ctx.album_tile_width as f32, |val| {
                            Message::SetAlbumTileWidth(val.round() as u32)
                        },)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(160.0)),
                        Space::new().width(10),
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
            .spacing(10)
            .boxed(),
        );

        // 3. Artists Tab Section
        let artist_tile_presets = [140, 160, 200, 240];
        let artist_preset_buttons = {
            let mut r = row![].spacing(6).align_y(Alignment::Center);
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
            "Artists Tab",
            column![
                text(
                    theme_cfg,
                    "Layout presentation and tile dimensions for artist catalog"
                )
                .size(12)
                .color(theme_cfg.text_muted_color()),
                row![
                    text(theme_cfg, "Catalog View Mode:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    components::view_mode_toggle(
                        theme_cfg,
                        ctx.artists_view_mode,
                        Message::SetArtistsViewMode,
                    ),
                ]
                .align_y(Alignment::Center),
                row![
                    text(theme_cfg, "Tile Width:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    row![
                        slider(120.0..=280.0, ctx.artist_tile_width as f32, |val| {
                            Message::SetArtistTileWidth(val.round() as u32)
                        },)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(160.0)),
                        Space::new().width(10),
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
            .spacing(10)
            .boxed(),
        );

        // 4. Now Playing Section
        let now_playing_presets = [180, 220, 260, 300];
        let now_playing_preset_buttons = {
            let mut r = row![].spacing(6).align_y(Alignment::Center);
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
            "Now Playing",
            column![
                text(theme_cfg, "Artwork dimensions and playback display options")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                row![
                    text(theme_cfg, "Cover Art Dimension:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    row![
                        slider(160.0..=320.0, ctx.now_playing_cover_size as f32, |val| {
                            Message::SetNowPlayingCoverSize(val.round() as u32)
                        },)
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(160.0)),
                        Space::new().width(10),
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
                    text(theme_cfg, "Now Playing Notifications:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
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
                row![
                    text(theme_cfg, "Gapless Audio Playback:")
                        .size(12)
                        .color(theme_cfg.text_color())
                        .width(Length::Fixed(160.0)),
                    components::chip_with_icon(
                        theme_cfg,
                        if ctx.gapless {
                            icons::CHECK_SVG
                        } else {
                            icons::RADIO_SVG
                        },
                        if ctx.gapless { "Enabled" } else { "Disabled" },
                        ctx.gapless,
                        Message::ToggleGapless(!ctx.gapless),
                    ),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(10)
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
            Space::new().height(20),
        ]
        .spacing(18)
        .padding(24)
        .width(Length::Fill)
        .boxed()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 2. Library Section
    // ─────────────────────────────────────────────────────────────────────────────

    fn view_library<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(theme_cfg, "Library & Storage", Some("Manage indexed audio directories, rescan metadata, monitor disk caches, or reset storage"));

        // Storage and Index Metrics
        let storage_stats_row1 = row![
            components::metric_box(theme_cfg, "Songs Indexed", format!("{}", ctx.total_tracks)),
            components::metric_box(
                theme_cfg,
                "Albums Cataloged",
                format!("{}", ctx.total_albums)
            ),
            components::metric_box(
                theme_cfg,
                "Artists Cataloged",
                format!("{}", ctx.total_artists)
            ),
        ]
        .spacing(12);

        let storage_stats_row2 = row![
            components::metric_box(
                theme_cfg,
                "Database File Size",
                format_file_size(ctx.db_size_bytes),
            ),
            components::metric_box(
                theme_cfg,
                "Disk Thumbnail Cache",
                format!(
                    "{} files ({})",
                    ctx.cache_file_count,
                    format_file_size(ctx.cache_size_bytes)
                ),
            ),
        ]
        .spacing(12);

        let stats_card = components::settings_card(
            theme_cfg,
            "Storage & Index Metrics",
            column![storage_stats_row1, storage_stats_row2]
                .spacing(10)
                .boxed(),
        );

        // Scanned Folders List
        let add_folder_btn = button(
            row![
                icons::icon(icons::PLUS_SVG, 13.0, theme_cfg.primary_color()),
                Space::new().width(6),
                text(
                    theme_cfg,
                    if ctx.is_scanning {
                        "Scanning Library..."
                    } else {
                        "Add Music Folder"
                    }
                )
                .size(12)
                .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 14])
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
                        "Click 'Add Music Folder' to choose a directory containing audio files."
                    )
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
                ]
                .spacing(4)
                .align_x(Alignment::Center),
            )
            .padding(20)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface_color())),
                border: Border {
                    radius: Radius::from(4.0),
                    color: theme_cfg.border_color(),
                    width: 1.0,
                },
                ..Default::default()
            })
            .boxed()
        } else {
            let mut col = column![].spacing(6);
            for folder_path in &self.library_folders {
                let path_clone = folder_path.clone();
                let path_buf = PathBuf::from(folder_path);

                let rescan_btn = button(
                    row![
                        icons::icon(icons::REFRESH_SVG, 12.0, theme_cfg.text_color()),
                        Space::new().width(4),
                        text(theme_cfg, "Rescan")
                            .size(11)
                            .color(theme_cfg.text_color()),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([4, 8])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press_maybe(if ctx.is_scanning {
                    None
                } else {
                    Some(Message::RescanLibraryFolder(path_buf))
                });

                let remove_btn = button(
                    row![
                        icons::icon(icons::TRASH_SVG, 12.0, theme_cfg.danger_color()),
                        Space::new().width(4),
                        text(theme_cfg, "Remove")
                            .size(11)
                            .color(theme_cfg.danger_color()),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([4, 8])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press_maybe(if ctx.is_scanning {
                    None
                } else {
                    Some(Message::RemoveLibraryFolder(path_clone))
                });

                let item_row = container(
                    row![
                        icons::icon(icons::FOLDER_SVG, 15.0, theme_cfg.primary_color()),
                        Space::new().width(10),
                        text(theme_cfg, folder_path.as_str())
                            .size(12)
                            .color(theme_cfg.text_color())
                            .width(Length::Fill),
                        rescan_btn,
                        Space::new().width(6),
                        remove_btn,
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(iced::Background::Color(theme_cfg.surface_color())),
                    border: Border {
                        radius: Radius::from(4.0),
                        color: theme_cfg.border_color(),
                        width: 1.0,
                    },
                    ..Default::default()
                });

                col = col.push(item_row.boxed());
            }
            col.boxed()
        };

        let folders_card = components::settings_card(
            theme_cfg,
            "Scanned Music Directories",
            column![
                row![
                    text(
                        theme_cfg,
                        "Audio files inside these directories are indexed recursively."
                    )
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(Length::Fill),
                    add_folder_btn,
                ]
                .align_y(Alignment::Center),
                Space::new().height(4),
                folders_content,
            ]
            .spacing(8)
            .boxed(),
        );

        // Maintenance & Actions Card
        let reindex_all_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.primary_color()),
                Space::new().width(6),
                text(
                    theme_cfg,
                    if ctx.is_scanning {
                        "Reindexing Library..."
                    } else {
                        "Reindex Entire Library"
                    }
                )
                .size(12)
                .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([8, 16])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ReindexLibrary)
        });

        let clear_cache_btn = button(
            row![
                icons::icon(icons::TRASH_SVG, 13.0, theme_cfg.text_color()),
                Space::new().width(6),
                text(theme_cfg, "Clear Thumbnail Cache")
                    .size(12)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([8, 16])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ClearThumbnailCache);

        let clear_db_btn = button(
            row![
                icons::icon(icons::TRASH_SVG, 13.0, theme_cfg.danger_color()),
                Space::new().width(6),
                text(theme_cfg, "Delete Library Database")
                    .size(12)
                    .color(theme_cfg.danger_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([8, 16])
        .style(styles::danger_button(theme_cfg))
        .on_press_maybe(if ctx.is_scanning {
            None
        } else {
            Some(Message::ClearLibrary)
        });

        let actions_card = components::settings_card(
            theme_cfg,
            "Library Actions & Maintenance",
            column![
                // Action 1: Reindex
                row![
                    column![
                        text(theme_cfg, "Full Reindex")
                            .size(13)
                            .color(theme_cfg.text_color()),
                        text(theme_cfg, "Scans all configured music folders, updates metadata tags, and repopulates the library.")
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .width(Length::Fill),
                    reindex_all_btn,
                ]
                .align_y(Alignment::Center),
                Space::new().height(6),
                // Action 2: Clear Thumbnail Cache
                row![
                    column![
                        text(theme_cfg, "Purge Thumbnail Cache")
                            .size(13)
                            .color(theme_cfg.text_color()),
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
                Space::new().height(6),
                // Action 3: Clear / Reset Database
                row![
                    column![
                        text(theme_cfg, "Reset Library Database")
                            .size(13)
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
            .spacing(10).boxed(),
        );

        column![
            header,
            stats_card,
            folders_card,
            actions_card,
            Space::new().height(20),
        ]
        .spacing(18)
        .padding(24)
        .width(Length::Fill)
        .boxed()
    }

    fn view_scrobbling<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(theme_cfg, "Scrobbling & Social Audio", Some("Track your listening habits and broadcast live playback status to ListenBrainz and Last.fm"));

        // 1. General Preferences Card
        let prefs_card = components::settings_card(
        theme_cfg,
        "Scrobble Preferences",
        column![
            text(theme_cfg, "Playback notification and logging thresholds")
                .size(12)
                .color(theme_cfg.text_muted_color()),
            row![
                column![
                    text(theme_cfg, "Now Playing Notifications")
                        .size(13)
                        .color(theme_cfg.text_color()),
                    text(
                        theme_cfg,
                        "Broadcast currently playing track metadata when playback begins"
                    )
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
                ]
                .width(Length::Fill),
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
            Space::new().height(6),
            column![
                text(theme_cfg, "Scrobble Threshold")
                    .size(13)
                    .color(theme_cfg.text_color()),
                text(
                    theme_cfg,
                    "Percentage of track length to listen to before logging a scrobble (capped at 4 minutes)"
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
                row![
                    components::chip(
                        theme_cfg,
                        "50% (Standard)",
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
                        "100% (Complete)",
                        ctx.scrobble_percent == 100,
                        Message::SetScrobblePercent(100),
                    ),
                ]
                .spacing(8),
            ]
            .spacing(6),
        ]
        .spacing(12)
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
                    .padding([6, 10])
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            color.r, color.g, color.b, 0.1,
                        ))),
                        border: Border {
                            radius: Radius::from(4.0),
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
                Space::new().width(6),
                text(theme_cfg, "Test ListenBrainz Connection")
                    .size(11)
                    .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::TestScrobble(
            plugins::scrobbler::ScrobbleService::ListenBrainz,
        ));

        let mut lb_column = column![
            row![
                column![
                    text(theme_cfg, "MetaBrainz ListenBrainz")
                        .size(14)
                        .color(theme_cfg.text_color()),
                    text(
                        theme_cfg,
                        "Open-source community listening registry operated by MetaBrainz"
                    )
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
                ]
                .width(Length::Fill),
                components::chip_with_icon(
                    theme_cfg,
                    if ctx.listenbrainz_enabled {
                        icons::CHECK_SVG
                    } else {
                        icons::RADIO_SVG
                    },
                    if ctx.listenbrainz_enabled {
                        "Plugin Active"
                    } else {
                        "Plugin Inactive"
                    },
                    ctx.listenbrainz_enabled,
                    Message::ToggleListenBrainz(!ctx.listenbrainz_enabled),
                ),
            ]
            .align_y(Alignment::Center),
            row![
                components::metric_box(theme_cfg, "Status", ctx.listenbrainz_status.to_string()),
                components::metric_box(
                    theme_cfg,
                    "Session Scrobbles",
                    ctx.listenbrainz_scrobbles.to_string(),
                ),
            ]
            .spacing(12),
            column![
                text(theme_cfg, "User Token")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text_input("Enter ListenBrainz user token...", ctx.listenbrainz_token)
                    .on_input(Message::ListenBrainzTokenChanged)
                    .style(styles::search_input_style(theme_cfg))
                    .padding([7, 10]),
                text(
                    theme_cfg,
                    "Obtain your user token from your profile at https://listenbrainz.org/profile/"
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(4),
            column![
                text(theme_cfg, "API Endpoint URL")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text_input(
                    "https://api.listenbrainz.org/1/submit-listens",
                    ctx.listenbrainz_api_url
                )
                .on_input(Message::ListenBrainzUrlChanged)
                .style(styles::search_input_style(theme_cfg))
                .padding([7, 10]),
            ]
            .spacing(4),
            lb_test_btn,
        ]
        .spacing(12);

        if let Some(banner) = lb_test_banner {
            lb_column = lb_column.push(banner.boxed());
        }

        let listenbrainz_card =
            components::settings_card(theme_cfg, "ListenBrainz Plugin", lb_column.boxed());

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
                    .padding([6, 10])
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            color.r, color.g, color.b, 0.1,
                        ))),
                        border: Border {
                            radius: Radius::from(4.0),
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
                Space::new().width(6),
                text(theme_cfg, "Test Last.fm Connection")
                    .size(11)
                    .color(theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::TestScrobble(
            plugins::scrobbler::ScrobbleService::LastFm,
        ));

        let mut lastfm_column = column![
            row![
                column![
                    text(theme_cfg, "Last.fm Scrobbler")
                        .size(14)
                        .color(theme_cfg.text_color()),
                    text(
                        theme_cfg,
                        "Submit listening history and now playing status to Last.fm"
                    )
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
                ]
                .width(Length::Fill),
                components::chip_with_icon(
                    theme_cfg,
                    if ctx.lastfm_enabled {
                        icons::CHECK_SVG
                    } else {
                        icons::RADIO_SVG
                    },
                    if ctx.lastfm_enabled {
                        "Plugin Active"
                    } else {
                        "Plugin Inactive"
                    },
                    ctx.lastfm_enabled,
                    Message::ToggleLastFm(!ctx.lastfm_enabled),
                ),
            ]
            .align_y(Alignment::Center),
            row![
                components::metric_box(theme_cfg, "Status", ctx.lastfm_status.to_string()),
                components::metric_box(
                    theme_cfg,
                    "Session Scrobbles",
                    ctx.lastfm_scrobbles.to_string(),
                ),
            ]
            .spacing(12),
            column![
                text(theme_cfg, "API Key")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text_input("Enter Last.fm API Key...", ctx.lastfm_api_key)
                    .on_input(Message::LastFmApiKeyChanged)
                    .style(styles::search_input_style(theme_cfg))
                    .padding([7, 10]),
            ]
            .spacing(4),
            column![
                text(theme_cfg, "Shared Secret")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text_input("Enter Last.fm Shared Secret...", ctx.lastfm_api_secret)
                    .on_input(Message::LastFmApiSecretChanged)
                    .style(styles::search_input_style(theme_cfg))
                    .padding([7, 10]),
            ]
            .spacing(4),
            column![
                text(theme_cfg, "Session Key (sk)")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text_input("Enter Last.fm Session Key (sk)...", ctx.lastfm_session_key)
                    .on_input(Message::LastFmSessionKeyChanged)
                    .style(styles::search_input_style(theme_cfg))
                    .padding([7, 10]),
                text(
                    theme_cfg,
                    "Obtain your API Key and Session Key from your Last.fm API account settings"
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(4),
            column![
                text(theme_cfg, "API Endpoint URL")
                    .size(12)
                    .color(theme_cfg.text_color()),
                text_input("https://ws.audioscrobbler.com/2.0/", ctx.lastfm_api_url)
                    .on_input(Message::LastFmUrlChanged)
                    .style(styles::search_input_style(theme_cfg))
                    .padding([7, 10]),
            ]
            .spacing(4),
            lastfm_test_btn,
        ]
        .spacing(12);

        if let Some(banner) = lastfm_test_banner {
            lastfm_column = lastfm_column.push(banner.boxed());
        }

        let lastfm_card =
            components::settings_card(theme_cfg, "Last.fm Plugin", lastfm_column.boxed());

        column![
            header,
            prefs_card,
            listenbrainz_card,
            lastfm_card,
            Space::new().height(20),
        ]
        .spacing(18)
        .padding(24)
        .width(Length::Fill)
        .boxed()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn swatch_card<'a>(
    theme_cfg: &ThemeConfig,
    label: &'static str,
    color: iced::Color,
) -> Element<'a, Message> {
    let border = theme_cfg.border_color();
    column![
        container(Space::new().width(52).height(22)).style(move |_theme| container::Style {
            background: Some(iced::Background::Color(color)),
            border: Border {
                radius: Radius::from(4.0),
                color: border,
                width: 1.0,
            },
            ..Default::default()
        }),
        text(theme_cfg, label)
            .size(10)
            .color(theme_cfg.text_muted_color()),
    ]
    .spacing(4)
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
