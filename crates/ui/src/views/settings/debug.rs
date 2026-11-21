use iced::border::Radius;
use iced::widget::{column, container, pick_list, row, slider, text_input, toggler, Space, Widget};
use iced::{Alignment, Border, Color, Element, Length};

use crate::message::Message;
use component::Marquee;
use component::{self, button, icons, styles, text};
use model::SortField;
use theme::ThemeConfig;

use super::*;
impl SettingsView {
    pub(super) fn view_debug<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let back_btn = component::chip_with_icon(
            theme_cfg,
            font,
            icons::ARROW_LEFT_SVG,
            "Back to Appearance",
            false,
            Message::SetDebugSubpage(false),
        );

        let header = component::settings_header(
            theme_cfg,
            font,
            "Component gallery & theme tokens",
            Some("Live preview of all active theme color tokens and basic UI components"),
        );

        let colors_card = self.view_debug_colors(theme_cfg, font);
        let typography_card = self.view_debug_typography(theme_cfg, font);
        let buttons_card = self.view_debug_buttons(theme_cfg, font);
        let controls_card = self.view_debug_controls(theme_cfg, font);
        let feedback_card = self.view_debug_feedback(theme_cfg, font);
        let icons_card = self.view_debug_icons(theme_cfg, font);

        column![
            column![back_btn, header].spacing(SECTION_SPACING),
            colors_card,
            typography_card,
            buttons_card,
            controls_card,
            feedback_card,
            icons_card,
            Space::new().height(PAGE_BOTTOM_SPACER),
        ]
        .spacing(PAGE_SPACING)
        .padding(PAGE_PADDING)
        .boxed()
    }

    pub(super) fn view_debug_colors<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let tokens: [(&'static str, Color, bool); 14] = [
            ("background", theme_cfg.background, false),
            ("surface", theme_cfg.surface, false),
            ("surface_hover", theme_cfg.surface_hover, false),
            ("border", theme_cfg.border, false),
            ("primary", theme_cfg.primary, false),
            ("accent", theme_cfg.accent, false),
            ("text", theme_cfg.text, false),
            ("text_muted", theme_cfg.text_muted, false),
            ("success", theme_cfg.success, false),
            ("danger", theme_cfg.danger, false),
            (
                "player_bar_bg",
                theme_cfg.player_bar_bg_color(),
                theme_cfg.player_bar_bg.is_none(),
            ),
            (
                "player_bar_text",
                theme_cfg.player_bar_text_color(),
                theme_cfg.player_bar_text.is_none(),
            ),
            (
                "slider_active",
                theme_cfg.slider_active_color(),
                theme_cfg.slider_active.is_none(),
            ),
            (
                "slider_rail",
                theme_cfg.slider_rail_color(),
                theme_cfg.slider_rail.is_none(),
            ),
        ];

        let mut grid = column![].spacing(SPACE_LG);
        for chunk in tokens.chunks(4) {
            let mut r = row![].spacing(SPACE_LG);
            for &(name, color, is_fallback) in chunk {
                r = r.push(debug_color_tile(theme_cfg, font, name, color, is_fallback));
            }
            for _ in chunk.len()..4 {
                r = r.push(
                    container(Space::new())
                        .width(Length::FillPortion(1))
                        .boxed(),
                );
            }
            grid = grid.push(r);
        }

        component::settings_card(theme_cfg, font, "Theme colors", grid.boxed())
    }

    pub(super) fn view_debug_typography<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let scale_col = column![
            text(font, "Heading 20px — Primary text")
                .size(20)
                .color(theme_cfg.text),
            text(font, "Subheading 16px — Primary text")
                .size(16)
                .color(theme_cfg.text),
            text(font, "Body 13px — Default readable copy")
                .size(13)
                .color(theme_cfg.text),
            text(font, "Label 12px — Muted secondary text")
                .size(12)
                .color(theme_cfg.text_muted),
            text(font, "Caption 11px — Subtle metadata")
                .size(11)
                .color(theme_cfg.text_muted),
            text(font, "Micro 10px — Compact badges & hints")
                .size(10)
                .color(theme_cfg.text_muted),
        ]
        .spacing(SPACE_SM);

        let semantic_row = row![
            text(font, "primary").size(12).color(theme_cfg.primary),
            text(font, "accent").size(12).color(theme_cfg.accent),
            text(font, "success").size(12).color(theme_cfg.success),
            text(font, "danger").size(12).color(theme_cfg.danger),
            text(font, "text").size(12).color(theme_cfg.text),
            text(font, "text_muted")
                .size(12)
                .color(theme_cfg.text_muted),
        ]
        .spacing(SPACE_LG)
        .align_y(Alignment::Center);

        let radii: [(&'static str, Radius); 5] = [
            ("xs", theme_cfg.border_radius.xs),
            ("sm", theme_cfg.border_radius.sm),
            ("md", theme_cfg.border_radius.md),
            ("lg", theme_cfg.border_radius.lg),
            ("xl", theme_cfg.border_radius.xl),
        ];

        let mut radius_row = row![].spacing(SPACE_LG).align_y(Alignment::Center);
        for (label, radius) in radii {
            let border_color = theme_cfg.border;
            let surface = theme_cfg.surface;
            radius_row = radius_row.push(
                container(text(font, label).size(11).color(theme_cfg.text_muted))
                    .padding([6, 12])
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(surface)),
                        border: Border {
                            radius,
                            color: border_color,
                            width: 1.0,
                        },
                        ..Default::default()
                    }),
            );
        }

        component::settings_card(
            theme_cfg,
            font,
            "Typography & border radii",
            column![scale_col, semantic_row, radius_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        )
    }

    pub(super) fn view_debug_buttons<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let buttons_row = row![
            button(text(font, "Primary").size(12).color(theme_cfg.background),)
                .padding(BUTTON_PADDING_MD)
                .style(styles::primary_button(theme_cfg))
                .on_press(Message::DebugNoop),
            button(text(font, "Ghost").size(12).color(theme_cfg.text))
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::DebugNoop),
            button(text(font, "Ghost active").size(12).color(theme_cfg.primary),)
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, true))
                .on_press(Message::DebugNoop),
            button(text(font, "Danger").size(12).color(theme_cfg.danger))
                .padding(BUTTON_PADDING_MD)
                .style(styles::danger_button(theme_cfg))
                .on_press(Message::DebugNoop),
            button(icons::icon(icons::SHUFFLE_SVG, 15.0, theme_cfg.text_muted))
                .padding(BUTTON_PADDING_SM)
                .style(styles::icon_control_button(theme_cfg))
                .on_press(Message::DebugNoop),
            button(icons::icon(
                icons::PREV_SVG,
                16.0,
                theme_cfg.player_bar_text_color()
            ))
            .padding(BUTTON_PADDING_SM)
            .style(styles::player_bar_button(theme_cfg, false))
            .on_press(Message::DebugNoop),
            button(icons::icon(icons::PLAY_SVG, 16.0, theme_cfg.background))
                .padding([6, 10])
                .style(styles::player_bar_play_button(theme_cfg))
                .on_press(Message::DebugNoop),
        ]
        .spacing(SPACE_LG)
        .align_y(Alignment::Center);

        let chips_row = row![
            component::chip(
                theme_cfg,
                font,
                "Chip A",
                self.debug_state.active_chip == 0,
                Message::DebugChipSelected(0),
            ),
            component::chip(
                theme_cfg,
                font,
                "Chip B",
                self.debug_state.active_chip == 1,
                Message::DebugChipSelected(1),
            ),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::PALETTE_SVG,
                "With icon",
                self.debug_state.active_chip == 2,
                Message::DebugChipSelected(2),
            ),
            Space::new().width(SPACE_LG),
            component::view_mode_toggle(
                theme_cfg,
                font,
                self.debug_state.view_mode,
                Message::DebugViewModeChanged,
            ),
        ]
        .spacing(CHIP_ROW_SPACING)
        .align_y(Alignment::Center);

        let sort_row = component::sort_chip_list(
            theme_cfg,
            font,
            &[
                ("Title", SortField::Title),
                ("Artist", SortField::Artist),
                ("Album", SortField::Album),
                ("Duration", SortField::Duration),
            ],
            self.debug_state.sort_field,
            self.debug_state.sort_dir,
            Message::DebugSortChanged,
        );

        component::settings_card(
            theme_cfg,
            font,
            "Buttons & chips",
            column![buttons_row, chips_row, sort_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        )
    }

    pub(super) fn view_debug_controls<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let toggler_row = row![
            toggler(self.debug_state.toggler)
                .on_toggle(Message::DebugToggle)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
            Space::new().width(SPACE_MD),
            text(
                font,
                if self.debug_state.toggler {
                    "Toggler (on)"
                } else {
                    "Toggler (off)"
                },
            )
            .size(12)
            .color(theme_cfg.text),
        ]
        .align_y(Alignment::Center);

        let sliders_row = row![
            column![
                text(
                    font,
                    format!(
                        "Settings slider ({:.0}%)",
                        self.debug_state.slider_val * 100.0
                    ),
                )
                .size(11)
                .color(theme_cfg.text_muted),
                slider(
                    0.0..=1.0,
                    self.debug_state.slider_val,
                    Message::DebugSliderChanged,
                )
                .step(0.01)
                .width(Length::Fixed(SETTING_SLIDER_WIDTH))
                .style(styles::settings_slider_style(theme_cfg)),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(font, "Seek slider")
                    .size(11)
                    .color(theme_cfg.text_muted),
                slider(
                    0.0..=1.0,
                    self.debug_state.slider_val,
                    Message::DebugSliderChanged,
                )
                .step(0.01)
                .width(Length::Fixed(SETTING_SLIDER_WIDTH))
                .style(styles::seek_slider_style(theme_cfg)),
            ]
            .spacing(FORM_FIELD_SPACING),
            column![
                text(font, "Volume slider")
                    .size(11)
                    .color(theme_cfg.text_muted),
                slider(
                    0.0..=1.0,
                    self.debug_state.slider_val,
                    Message::DebugSliderChanged,
                )
                .step(0.01)
                .width(Length::Fixed(120.0))
                .style(styles::volume_slider_style(theme_cfg)),
            ]
            .spacing(FORM_FIELD_SPACING),
        ]
        .spacing(SPACE_LG)
        .align_y(Alignment::Center);

        let options = vec![
            "Option A".to_string(),
            "Option B".to_string(),
            "Option C".to_string(),
        ];

        let inputs_row = row![
            text_input("Sample text input...", &self.debug_state.input_val)
                .on_input(Message::DebugInputChanged)
                .padding(INPUT_PADDING)
                .size(12)
                .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
                .style(styles::search_input_style(theme_cfg)),
            pick_list(
                Some(self.debug_state.selected_option.clone()),
                options,
                |opt: &String| opt.clone(),
            )
            .on_select(Message::DebugPickSelected)
            .padding(INPUT_PADDING)
            .text_size(12)
            .width(Length::Fixed(180.0))
            .style(styles::pick_list_style(theme_cfg))
            .menu_style(styles::menu_style(theme_cfg)),
        ]
        .spacing(SPACE_LG)
        .align_y(Alignment::Center);

        component::settings_card(
            theme_cfg,
            font,
            "Form controls",
            column![toggler_row, sliders_row, inputs_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        )
    }

    pub(super) fn view_debug_feedback<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let metrics_row = row![
            component::metric_box(theme_cfg, font, "Metric A", "1,234".to_string()),
            component::metric_box(theme_cfg, font, "Metric B", "42.0 MB".to_string()),
            component::metric_box(theme_cfg, font, "Metric C", "99.9%".to_string()),
        ]
        .spacing(SPACE_LG);

        let indicators_row = row![
            component::cover_art(
                theme_cfg,
                None,
                56.0,
                theme_cfg.border_radius.md,
                icons::ALBUM_SVG,
                24.0,
                theme_cfg.text_muted,
            ),
            column![
                component::scanning_indicator(theme_cfg, font, Some((12, 48))),
                component::thumbnail_caching_indicator(theme_cfg, font),
                container(
                    Marquee::new(
                        "Marquee sample — Scrolling long track or album metadata smoothly"
                    )
                    .size(12)
                    .color(theme_cfg.text)
                    .font(font),
                )
                .width(Length::Fixed(240.0)),
            ]
            .spacing(SPACE_MD),
        ]
        .spacing(SPACE_LG)
        .align_y(Alignment::Center);

        let empty_preview = container(component::empty_state(
            theme_cfg,
            font,
            "Empty state title",
            "Supporting muted subtitle for empty views",
        ))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(theme_cfg.surface)),
            border: Border {
                radius: theme_cfg.border_radius.md,
                color: theme_cfg.border,
                width: 1.0,
            },
            ..Default::default()
        });

        component::settings_card(
            theme_cfg,
            font,
            "Containers, indicators & feedback",
            column![metrics_row, indicators_row, empty_preview]
                .spacing(SECTION_SPACING)
                .boxed(),
        )
    }

    pub(super) fn view_debug_icons<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        const COLS: usize = 8;
        let all_icons: [(&'static str, &'static str); 44] = [
            ("Play", icons::PLAY_SVG),
            ("Play circle", icons::PLAY_CIRCLE_SVG),
            ("Pause", icons::PAUSE_SVG),
            ("Prev", icons::PREV_SVG),
            ("Next", icons::NEXT_SVG),
            ("Stop", icons::STOP_SVG),
            ("Vol mute", icons::VOL_MUTE_SVG),
            ("Vol low", icons::VOL_LOW_SVG),
            ("Vol high", icons::VOL_HIGH_SVG),
            ("Album", icons::ALBUM_SVG),
            ("Music", icons::MUSIC_NOTE_SVG),
            ("Playlist", icons::PLAYLIST_SVG),
            ("Queue", icons::QUEUE_SVG),
            ("Shuffle", icons::SHUFFLE_SVG),
            ("Repeat", icons::REPEAT_SVG),
            ("Repeat one", icons::REPEAT_ONE_SVG),
            ("Search", icons::SEARCH_SVG),
            ("Speaker", icons::SPEAKER_PLAYING_SVG),
            ("Chevron up", icons::CHEVRON_UP_SVG),
            ("Chevron down", icons::CHEVRON_DOWN_SVG),
            ("Chevron right", icons::CHEVRON_RIGHT_SVG),
            ("Disc", icons::DISC_SVG),
            ("User", icons::USER_AVATAR_SVG),
            ("Settings", icons::SETTINGS_SVG),
            ("List view", icons::LIST_VIEW_SVG),
            ("Grid view", icons::GRID_VIEW_SVG),
            ("Folder", icons::FOLDER_SVG),
            ("Refresh", icons::REFRESH_SVG),
            ("Check", icons::CHECK_SVG),
            ("Palette", icons::PALETTE_SVG),
            ("Database", icons::DATABASE_SVG),
            ("Trash", icons::TRASH_SVG),
            ("Plus", icons::PLUS_SVG),
            ("Radio", icons::RADIO_SVG),
            ("Arrow left", icons::ARROW_LEFT_SVG),
            ("Download", icons::DOWNLOAD_SVG),
            ("External", icons::EXTERNAL_LINK_SVG),
            ("Open", icons::OPEN_SVG),
            ("Bug", icons::BUG_SVG),
            ("Extension", icons::EXTENSION_SVG),
            ("Win min", icons::WINDOW_MINIMIZE_SVG),
            ("Win max", icons::WINDOW_MAXIMIZE_SVG),
            ("Win restore", icons::WINDOW_RESTORE_SVG),
            ("Win close", icons::WINDOW_CLOSE_SVG),
        ];

        let mut grid = column![].spacing(SPACE_LG);
        for chunk in all_icons.chunks(COLS) {
            let mut r = row![].spacing(SPACE_LG);
            for &(label, svg) in chunk {
                r = r.push(
                    column![
                        icons::icon(svg, 18.0, theme_cfg.text),
                        text(font, label).size(10).color(theme_cfg.text_muted),
                    ]
                    .spacing(SPACE_SM)
                    .align_x(Alignment::Center)
                    .width(Length::FillPortion(1)),
                );
            }
            for _ in chunk.len()..COLS {
                r = r.push(column![].width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }

        component::settings_card(theme_cfg, font, "Vector icons", grid.boxed())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub(super) fn format_color_hex(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (c.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }
}

pub(super) fn debug_color_tile<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    name: &'static str,
    color: Color,
    is_fallback: bool,
) -> Element<'a, Message> {
    let hex = format_color_hex(color);
    let subtitle = if is_fallback {
        format!("{hex} (fallback)")
    } else {
        hex
    };
    let border = theme_cfg.border;
    let surface = theme_cfg.surface;

    container(
        row![
            container(Space::new().width(28).height(28)).style(move |_| container::Style {
                background: Some(iced::Background::Color(color)),
                border: Border {
                    radius: theme_cfg.border_radius.sm,
                    color: border,
                    width: 1.0,
                },
                ..Default::default()
            }),
            column![
                text(font, name).size(11).color(theme_cfg.text),
                text(font, subtitle).size(10).color(theme_cfg.text_muted),
            ]
            .spacing(2),
        ]
        .spacing(SPACE_LG)
        .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .width(Length::FillPortion(1))
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(surface)),
        border: Border {
            radius: theme_cfg.border_radius.sm,
            color: border,
            width: 1.0,
        },
        ..Default::default()
    })
    .boxed()
}
