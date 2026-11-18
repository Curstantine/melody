use iced::widget::{button, column, container, pick_list, row, slider, toggler, Space, Widget};
use iced::{Alignment, Border, Element, Length};

use crate::message::Message;
use component::{self, icons, styles, text};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_appearance<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = component::settings_header(theme_cfg, font, "Appearance & interface", Some("Customize color palette, typography font family, default display modes, and layout density"));

        // Theme & Layout Card
        let theme_buttons = {
            let mut row_wrap = row![].spacing(CHIP_ROW_SPACING);
            for name in ctx.theme_names {
                let is_active = name == ctx.active_theme_name;
                let icon_svg = if is_active {
                    icons::CHECK_SVG
                } else {
                    icons::DISC_SVG
                };
                let btn = component::chip_with_icon(
                    theme_cfg,
                    font,
                    icon_svg,
                    name.clone(),
                    is_active,
                    Message::SelectTheme(name.clone()),
                );
                row_wrap = row_wrap.push(btn);
            }
            row_wrap.wrap()
        };

        let layout_buttons = {
            let mut row_wrap = row![].spacing(CHIP_ROW_SPACING);
            for name in ctx.layout_names {
                let is_active = name == ctx.active_layout_name;
                let icon_svg = if is_active {
                    icons::CHECK_SVG
                } else {
                    icons::ALBUM_SVG
                };
                let btn = component::chip_with_icon(
                    theme_cfg,
                    font,
                    icon_svg,
                    name.clone(),
                    is_active,
                    Message::SelectLayout(name.clone()),
                );
                row_wrap = row_wrap.push(btn);
            }
            row_wrap.wrap()
        };

        let titlebar_toggle = row![
            column![
                text(font, "Custom titlebar").size(13).color(theme_cfg.text),
                text(
                    font,
                    "Merge navigation bar and window controls into a single custom titlebar"
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(2)
            .width(Length::Fill),
            toggler(ctx.custom_titlebar)
                .on_toggle(Message::ToggleCustomTitlebar)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center)
        .spacing(SPACE_LG);

        let reload_appearance_btn = button(
            row![
                icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text),
                Space::new().width(SPACE_MD),
                text(font, "Reload themes & layouts from disk")
                    .size(12)
                    .color(theme_cfg.text),
            ]
            .align_y(Alignment::Center),
        )
        .padding(BUTTON_PADDING_MD)
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::ReloadThemesAndLayouts);

        let theme_layout_card = component::settings_card(
            theme_cfg,
            font,
            "Theme & layout",
            column![
                column![
                    text(font, "Color palette")
                        .size(12)
                        .color(theme_cfg.text_muted),
                    theme_buttons,
                ]
                .spacing(CARD_SPACING),
                column![
                    text(font, "Layout preset")
                        .size(12)
                        .color(theme_cfg.text_muted),
                    layout_buttons,
                ]
                .spacing(CARD_SPACING),
                titlebar_toggle,
                reload_appearance_btn,
            ]
            .spacing(SECTION_SPACING)
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
        .font(font)
        .padding(INPUT_PADDING)
        .text_size(12)
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fill)
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let active_iced_font = font;

        let sample_text_box = container(
            column![
                text(font, "Multilingual glyph rendering preview")
                    .size(11)
                    .color(theme_cfg.primary),
                Space::new().height(SPACE_SM),
                text(
                    font,
                    "English: The quick brown fox jumps over the lazy dog."
                )
                .size(12)
                .color(theme_cfg.text)
                .font(active_iced_font),
                text(
                    font,
                    "日本語: あのイーハトーヴォのすきとおった風、夏でも底に冷たさをもつ青いそら"
                )
                .size(12)
                .color(theme_cfg.text)
                .font(active_iced_font),
                text(font, "中文: 落霞与孤齐飞，秋水共长天一色")
                    .size(12)
                    .color(theme_cfg.text)
                    .font(active_iced_font),
                text(
                    font,
                    "한국어: 별 하나에 추억과 별 하나에 사랑과 쓸쓸함과 동경"
                )
                .size(12)
                .color(theme_cfg.text)
                .font(active_iced_font),
            ]
            .spacing(FORM_FIELD_SPACING),
        )
        .padding(PREVIEW_BOX_PADDING)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface)),
            border: Border {
                radius: theme_cfg.border_radius.sm,
                color: theme_cfg.border,
                width: 1.0,
            },
            ..Default::default()
        });

        let font_card = component::settings_card(
            theme_cfg,
            font,
            "Typography & font configuration",
            column![
                component::settings_row(
                    theme_cfg,
                    font,
                    "Select font family",
                    "System font family used for UI text rendering and catalog metadata",
                    container(font_selector).width(Length::Fixed(SETTING_DROPDOWN_WIDTH)),
                ),
                sample_text_box,
            ]
            .spacing(SECTION_SPACING)
            .boxed(),
        );

        // 1. Library Tab Section
        let page_sizes = [25, 50, 100, 200];
        let page_size_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &size in &page_sizes {
                let is_sel = ctx.page_size == size;
                let btn = component::chip(
                    theme_cfg,
                    font,
                    format!("{} tracks", size),
                    is_sel,
                    Message::SetPageSize(size),
                );
                r = r.push(btn);
            }
            r
        };

        let library_tab_card = component::settings_card(
            theme_cfg,
            font,
            "Library tab",
            component::settings_row(
                theme_cfg,
                font,
                "Page limit",
                "Number of tracks fetched and rendered per viewport window",
                page_size_buttons,
            ),
        );

        // 2. Albums Tab Section
        let album_view_mode_row = component::settings_row(
            theme_cfg,
            font,
            "Catalog view mode",
            "Switch between grid tile card presentation and compact list table",
            component::view_mode_toggle(
                theme_cfg,
                font,
                ctx.albums_view_mode,
                Message::SetAlbumsViewMode,
            ),
        );

        let album_tile_presets = [140, 170, 210, 260];
        let album_preset_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &width in &album_tile_presets {
                let is_sel = ctx.album_tile_width == width;
                let btn = component::chip(
                    theme_cfg,
                    font,
                    format!("{} px", width),
                    is_sel,
                    Message::SetAlbumTileWidth(width),
                );
                r = r.push(btn);
            }
            r
        };

        let album_tile_width_row = component::settings_row(
            theme_cfg,
            font,
            "Tile width",
            "Artwork card width for the album catalog grid",
            row![
                text(font, format!("{} px", ctx.album_tile_width))
                    .size(12)
                    .color(theme_cfg.primary)
                    .width(Length::Fixed(55.0)),
                Space::new().width(SPACE_SM),
                slider(120.0..=300.0, ctx.album_tile_width as f32, |val| {
                    Message::SetAlbumTileWidth(val.round() as u32)
                },)
                .on_release(Message::SaveConfig)
                .style(styles::settings_slider_style(theme_cfg))
                .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
                Space::new().width(SPACE_LG),
                album_preset_buttons,
            ]
            .align_y(Alignment::Center),
        );

        let albums_tab_card = component::settings_card(
            theme_cfg,
            font,
            "Albums tab",
            column![album_view_mode_row, album_tile_width_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        );

        // 3. Artists Tab Section
        let artist_view_mode_row = component::settings_row(
            theme_cfg,
            font,
            "Catalog view mode",
            "Switch between grid tile card presentation and compact list table",
            component::view_mode_toggle(
                theme_cfg,
                font,
                ctx.artists_view_mode,
                Message::SetArtistsViewMode,
            ),
        );

        let artist_tile_presets = [140, 160, 200, 240];
        let artist_preset_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &width in &artist_tile_presets {
                let is_sel = ctx.artist_tile_width == width;
                let btn = component::chip(
                    theme_cfg,
                    font,
                    format!("{} px", width),
                    is_sel,
                    Message::SetArtistTileWidth(width),
                );
                r = r.push(btn);
            }
            r
        };

        let artist_tile_width_row = component::settings_row(
            theme_cfg,
            font,
            "Tile width",
            "Avatar card width for the artist catalog grid",
            row![
                text(font, format!("{} px", ctx.artist_tile_width))
                    .size(12)
                    .color(theme_cfg.primary)
                    .width(Length::Fixed(55.0)),
                Space::new().width(SPACE_SM),
                slider(120.0..=280.0, ctx.artist_tile_width as f32, |val| {
                    Message::SetArtistTileWidth(val.round() as u32)
                },)
                .on_release(Message::SaveConfig)
                .style(styles::settings_slider_style(theme_cfg))
                .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
                Space::new().width(SPACE_LG),
                artist_preset_buttons,
            ]
            .align_y(Alignment::Center),
        );

        let artists_tab_card = component::settings_card(
            theme_cfg,
            font,
            "Artists tab",
            column![artist_view_mode_row, artist_tile_width_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        );

        // 4. Now Playing Section
        let active_lyrics_font = ctx.lyrics_font.unwrap_or("Default").to_string();
        let mut lyrics_font_options = self.available_fonts.clone();
        if !lyrics_font_options.iter().any(|f| f == "Default") {
            lyrics_font_options.insert(0, "Default".to_string());
        }
        if let Some(active) = ctx.lyrics_font {
            if !lyrics_font_options.iter().any(|f| f == active) {
                lyrics_font_options.push(active.to_string());
            }
        }

        let lyrics_font_selector = pick_list(
            Some(active_lyrics_font),
            lyrics_font_options,
            |option: &String| option.clone(),
        )
        .on_select(Message::SelectLyricsFont)
        .placeholder("Select a system font...")
        .font(font)
        .padding(INPUT_PADDING)
        .text_size(12)
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fixed(220.0))
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let lyrics_size_presets: [u16; 4] = [13, 15, 18, 22];
        let lyrics_size_preset_buttons = {
            let mut r = row![].spacing(CHIP_ROW_SPACING).align_y(Alignment::Center);
            for &size in &lyrics_size_presets {
                let is_sel = ctx.lyrics_font_size == size;
                let btn = component::chip(
                    theme_cfg,
                    font,
                    format!("{} px", size),
                    is_sel,
                    Message::SetLyricsFontSize(size),
                );
                r = r.push(btn);
            }
            r
        };

        let now_playing_card = component::settings_card(
            theme_cfg,
            font,
            "Now playing",
            column![
                component::settings_row(
                    theme_cfg,
                    font,
                    "Lyrics font family",
                    "Font used for lyrics in Now Playing (Default follows the primary UI font)",
                    lyrics_font_selector,
                ),
                component::settings_row(
                    theme_cfg,
                    font,
                    "Lyrics font size",
                    "Base text size for synced and plain lyrics in Now Playing",
                    row![
                        text(font, format!("{} px", ctx.lyrics_font_size))
                            .size(12)
                            .color(theme_cfg.primary)
                            .width(Length::Fixed(55.0)),
                        Space::new().width(SPACE_SM),
                        slider(11.0..=28.0, ctx.lyrics_font_size as f32, |val| {
                            Message::SetLyricsFontSize(val.round() as u16)
                        })
                        .on_release(Message::SaveConfig)
                        .style(styles::settings_slider_style(theme_cfg))
                        .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
                        Space::new().width(SPACE_LG),
                        lyrics_size_preset_buttons,
                    ]
                    .align_y(Alignment::Center),
                ),
            ]
            .spacing(SPACE_MD)
            .boxed(),
        );

        let debug_gallery_card = component::settings_card(
            theme_cfg,
            font,
            "Miscellaneous",
            component::settings_row(
                theme_cfg,
                font,
                "Component gallery & theme tokens",
                "Inspect all active palette colors, typography, and interactive controls",
                button(
                    row![
                        icons::icon(icons::OPEN_SVG, 14.0, theme_cfg.text),
                        Space::new().width(SPACE_MD),
                        text(font, "Open gallery").size(12).color(theme_cfg.text),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::SetDebugSubpage(true)),
            ),
        );

        column![
            header,
            theme_layout_card,
            font_card,
            library_tab_card,
            albums_tab_card,
            artists_tab_card,
            now_playing_card,
            debug_gallery_card,
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
}
