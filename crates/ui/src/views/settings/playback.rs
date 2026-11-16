use iced::widget::{column, pick_list, row, slider, toggler, Space, Widget};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use components::{self, styles, text};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_playback<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = components::settings_header(
            theme_cfg,
            font,
            "Playback preferences",
            Some("Configure audio transitions and playback behavior"),
        );

        // 1. Audio Transitions Card
        let gapless_row = row![
            column![
                text(font, "Gapless audio playback")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    "Preload upcoming tracks in memory for seamless, uninterrupted transitions",
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            toggler(ctx.gapless)
                .on_toggle(Message::ToggleGapless)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center);

        let transitions_card = components::settings_card(
            theme_cfg,
            font,
            "Audio transitions",
            column![gapless_row].boxed(),
        );

        // 2. Playback Behavior Card
        let volume_row = row![
            column![
                text(font, "Master output volume")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    "Default master volume multiplier applied to audio stream playback",
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            slider(0.0..=1.0, ctx.volume, Message::VolumeChanged)
                .style(styles::settings_slider_style(theme_cfg))
                .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
        ]
        .align_y(Alignment::Center);

        let shuffle_row = row![
            column![
                text(font, "Shuffle playback")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    "Randomize track selection and playback order within active queue",
                )
                .size(11)
                .color(theme_cfg.text_muted),
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
                text(font, "Repeat playback")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    "Loop the active queue continuously when the final track completes",
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            toggler(ctx.is_repeat)
                .on_toggle(|_| Message::ToggleRepeat)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center);

        let tray_row = row![
            column![
                text(font, "Minimize to tray on close")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    "Keep Melody running in the system tray when closed, freeing GPU memory",
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            toggler(ctx.minimize_to_tray)
                .on_toggle(Message::ToggleMinimizeToTray)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        ]
        .align_y(Alignment::Center);

        let behavior_card = components::settings_card(
            theme_cfg,
            font,
            "Playback behavior",
            column![volume_row, shuffle_row, repeat_row, tray_row]
                .spacing(SECTION_SPACING)
                .boxed(),
        );

        // 3. Audio Resampling Card
        let filter_selector = pick_list(
            Some(ctx.resampling.filter),
            &audio::ResamplingFilter::ALL[..],
            |filter: &audio::ResamplingFilter| filter.label().to_string(),
        )
        .on_select(Message::SetResamplingFilter)
        .font(font)
        .padding(INPUT_PADDING)
        .text_size(12)
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let filter_row = row![
            column![
                text(font, "Resampling filter algorithm")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(
                    font,
                    "Polyphase sinc filter or interpolation method used for sample rate conversion",
                )
                .size(11)
                .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            filter_selector,
        ]
        .align_y(Alignment::Center);

        let active_filter = ctx.resampling.filter;
        let quality_selector = pick_list(
            Some(ctx.resampling.quality),
            &audio::ResamplingQuality::ALL[..],
            move |quality: &audio::ResamplingQuality| {
                quality.label_for_filter(active_filter).to_string()
            },
        )
        .on_select(Message::SetResamplingQuality)
        .font(font)
        .padding(INPUT_PADDING)
        .text_size(12)
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let quality_desc = if active_filter == audio::ResamplingFilter::Soxr {
            "Internal computation bit precision; higher precision improves dynamic range and reduces quantization noise"
        } else {
            "Length of windowed sinc filter; higher taps sharpen transition band cutoff and reduce aliasing"
        };

        let quality_row = row![
            column![
                text(font, "Filter precision & taps")
                    .size(12)
                    .color(theme_cfg.text_muted),
                text(font, quality_desc)
                    .size(11)
                    .color(theme_cfg.text_muted),
            ]
            .spacing(FORM_FIELD_SPACING)
            .width(Length::Fill),
            quality_selector,
        ]
        .align_y(Alignment::Center);

        let mut resampling_rows = vec![filter_row, quality_row];

        if active_filter != audio::ResamplingFilter::Soxr {
            let linear_interp_row = row![
                column![
                    text(font, "Filter phase linear interpolation")
                        .size(12)
                        .color(theme_cfg.text_muted),
                    text(
                        font,
                        "Linearly interpolate between sinc filter phases for smoother frequency response and reduced noise",
                    )
                    .size(11)
                    .color(theme_cfg.text_muted),
                ]
                .spacing(FORM_FIELD_SPACING)
                .width(Length::Fill),
                toggler(ctx.resampling.linear_interp)
                    .on_toggle(Message::ToggleResamplingLinearInterp)
                    .size(TOGGLER_SIZE)
                    .style(styles::toggler_style(theme_cfg)),
            ]
            .align_y(Alignment::Center);

            resampling_rows.push(linear_interp_row);
        }

        let resampling_card = components::settings_card(
            theme_cfg,
            font,
            "Audio resampling",
            column(resampling_rows).spacing(SECTION_SPACING).boxed(),
        );

        column![
            header,
            transitions_card,
            behavior_card,
            resampling_card,
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
}
