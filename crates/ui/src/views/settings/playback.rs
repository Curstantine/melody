use iced::widget::{column, pick_list, row, slider, toggler, Widget};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use component::{self, button, icons, styles};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_playback<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: font::Font,
        ctx: &SettingsContext<'a>,
    ) -> Element<'a, Message> {
        let header = component::settings_header(
            theme_cfg,
            font,
            "Playback preferences",
            Some("Configure audio transitions and playback behavior"),
        );

        // 0. Audio Output Device Card
        const SYSTEM_DEFAULT_LABEL: &str = "System Default";
        let mut device_options = Vec::with_capacity(1 + ctx.available_output_devices.len());
        device_options.push(SYSTEM_DEFAULT_LABEL.to_string());
        for dev in ctx.available_output_devices {
            device_options.push(dev.clone());
        }
        if let Some(configured) = ctx.output_device {
            if !device_options.iter().any(|d| d == configured) {
                device_options.push(configured.to_string());
            }
        }
        let selected_device = ctx
            .output_device
            .map(ToString::to_string)
            .unwrap_or_else(|| SYSTEM_DEFAULT_LABEL.to_string());

        let device_selector = pick_list(Some(selected_device), device_options, |d: &String| {
            d.clone()
        })
        .on_select(|selected| {
            if selected == SYSTEM_DEFAULT_LABEL {
                Message::SetOutputDevice(None)
            } else {
                Message::SetOutputDevice(Some(selected))
            }
        })
        .font(font.family)
        .padding(INPUT_PADDING)
        .text_size(font.px(12.0))
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let refresh_devices_btn = button(icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text))
            .padding([6, 8])
            .style(styles::ghost_button(theme_cfg, false))
            .on_press(Message::RefreshOutputDevices);

        let output_device_row = component::settings_row(
            theme_cfg,
            font,
            "Output device",
            "Select the audio hardware device used for playback",
            row![refresh_devices_btn, device_selector]
                .spacing(SPACE_SM)
                .align_y(Alignment::Center),
        );

        let output_card = component::settings_card(
            theme_cfg,
            font,
            "Audio output",
            column![output_device_row].boxed(),
        );

        // 1. Audio Transitions Card
        let gapless_row = component::settings_row(
            theme_cfg,
            font,
            "Gapless audio playback",
            "Preload upcoming tracks in memory for seamless, uninterrupted transitions",
            toggler(ctx.gapless)
                .on_toggle(Message::ToggleGapless)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        );

        let transitions_card = component::settings_card(
            theme_cfg,
            font,
            "Audio transitions",
            column![gapless_row].boxed(),
        );

        // 2. Playback Behavior Card
        let volume_row = component::settings_row(
            theme_cfg,
            font,
            "Master output volume",
            "Default master volume multiplier applied to audio stream playback",
            slider(0.0..=1.0, ctx.volume, Message::VolumeChanged)
                .step(0.01)
                .on_release(Message::SaveState)
                .style(styles::settings_slider_style(theme_cfg))
                .width(Length::Fixed(SETTING_SLIDER_WIDTH)),
        );

        let shuffle_row = component::settings_row(
            theme_cfg,
            font,
            "Shuffle playback",
            "Randomize track selection and playback order within active queue",
            toggler(ctx.is_shuffle)
                .on_toggle(|_| Message::ToggleShuffle)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        );

        let repeat_selector = pick_list(
            Some(ctx.repeat_mode),
            &model::RepeatMode::ALL[..],
            |mode: &model::RepeatMode| mode.label().to_string(),
        )
        .on_select(Message::SetRepeatMode)
        .font(font.family)
        .padding(INPUT_PADDING)
        .text_size(font.px(12.0))
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let repeat_row = component::settings_row(
            theme_cfg,
            font,
            "Repeat mode",
            "Choose whether playback stops, loops the active queue, or loops the current track",
            repeat_selector,
        );

        let tray_row = component::settings_row(
            theme_cfg,
            font,
            "Minimize to tray on close",
            "Keep Melody running in the system tray when closed, freeing GPU memory",
            toggler(ctx.minimize_to_tray)
                .on_toggle(Message::ToggleMinimizeToTray)
                .size(TOGGLER_SIZE)
                .style(styles::toggler_style(theme_cfg)),
        );

        let behavior_card = component::settings_card(
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
        .font(font.family)
        .padding(INPUT_PADDING)
        .text_size(font.px(12.0))
        .handle(pick_list::Handle::Arrow {
            size: Some(11.0.into()),
        })
        .width(Length::Fixed(SETTING_DROPDOWN_WIDTH))
        .style(styles::pick_list_style(theme_cfg))
        .menu_style(styles::menu_style(theme_cfg));

        let filter_row = component::settings_row(
            theme_cfg,
            font,
            "Resampling filter algorithm",
            "Polyphase sinc filter or interpolation method used for sample rate conversion",
            filter_selector,
        );

        let active_filter = ctx.resampling.filter;
        let quality_selector = pick_list(
            Some(ctx.resampling.quality),
            &audio::ResamplingQuality::ALL[..],
            move |quality: &audio::ResamplingQuality| {
                quality.label_for_filter(active_filter).to_string()
            },
        )
        .on_select(Message::SetResamplingQuality)
        .font(font.family)
        .padding(INPUT_PADDING)
        .text_size(font.px(12.0))
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

        let quality_row = component::settings_row(
            theme_cfg,
            font,
            "Filter precision & taps",
            quality_desc,
            quality_selector,
        );

        let mut resampling_rows = vec![filter_row, quality_row];

        if active_filter != audio::ResamplingFilter::Soxr {
            let linear_interp_row = component::settings_row(
                theme_cfg,
                font,
                "Filter phase linear interpolation",
                "Linearly interpolate between sinc filter phases for smoother frequency response and reduced noise",
                toggler(ctx.resampling.linear_interp)
                    .on_toggle(Message::ToggleResamplingLinearInterp)
                    .size(TOGGLER_SIZE)
                    .style(styles::toggler_style(theme_cfg)),
            );

            resampling_rows.push(linear_interp_row);
        }

        let resampling_card = component::settings_card(
            theme_cfg,
            font,
            "Audio resampling",
            column(resampling_rows).spacing(SECTION_SPACING).boxed(),
        );

        column![
            header,
            output_card,
            transitions_card,
            behavior_card,
            resampling_card,
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
