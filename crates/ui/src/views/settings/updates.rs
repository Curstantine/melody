use iced::widget::{column, container, row, svg, Space, Widget};
use iced::{Alignment, Color, Element, Length};

use crate::message::Message;
use component::{self, border_vert, button, icons, styles, text, Spinner};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_update_section<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let current_version = env!("CARGO_PKG_VERSION");
        let ver_label = || {
            text(font, format!("v{current_version}"))
                .size(10)
                .color(theme_cfg.text_muted)
        };

        let mut section = column![].spacing(SPACE_MD).width(Length::Fill);

        match &self.update_status {
            updater::UpdateStatus::Available(info) => {
                section = section
                    .push(update_info_row(theme_cfg, font, info, current_version))
                    .push(primary_action_button(
                        theme_cfg,
                        font,
                        icons::DOWNLOAD_SVG,
                        "Update & restart",
                        Message::StartUpdateDownload(info.clone()),
                    ));
            }
            updater::UpdateStatus::Downloading { info, percent } => {
                let handle = svg::Handle::from_memory(icons::REFRESH_SVG.as_bytes());
                let spinner = Spinner::new(handle).size(13.0).color(theme_cfg.text_muted);
                let downloading_btn = button(
                    container(
                        row![
                            spinner,
                            Space::new().width(SPACE_MD),
                            text(font, format!("Downloading {}%...", percent.clamp(&0, &100)))
                                .size(12)
                                .color(theme_cfg.text_muted),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(BUTTON_PADDING_MD)
                .style(styles::primary_button(theme_cfg));

                section = section
                    .push(update_info_row(theme_cfg, font, info, current_version))
                    .push(downloading_btn.boxed());
            }
            updater::UpdateStatus::ReadyToRestart { info } => {
                section = section
                    .push(update_info_row(theme_cfg, font, info, current_version))
                    .push(primary_action_button(
                        theme_cfg,
                        font,
                        icons::REFRESH_SVG,
                        "Restart now",
                        Message::ApplyUpdateAndRestart,
                    ));
            }
            updater::UpdateStatus::Checking => {
                let handle = svg::Handle::from_memory(icons::REFRESH_SVG.as_bytes());
                let spinner = Spinner::new(handle).size(11.0).color(theme_cfg.text_muted);
                let checking_row = row![
                    spinner,
                    Space::new().width(SPACE_SM),
                    text(font, "Checking for updates...")
                        .size(11)
                        .line_height(1.0)
                        .color(theme_cfg.text_muted),
                    Space::new().width(Length::Fill),
                    ver_label(),
                ]
                .align_y(Alignment::Center);

                section = section.push(checking_row.boxed());
            }
            updater::UpdateStatus::UpToDate | updater::UpdateStatus::Idle => {
                section = section.push(
                    row![Space::new().width(Length::Fill), ver_label()]
                        .align_y(Alignment::Center)
                        .boxed(),
                );
            }
            updater::UpdateStatus::Failed(_) => {
                let error_row = row![
                    text(font, "Update check failed")
                        .size(11)
                        .color(theme_cfg.danger),
                    Space::new().width(Length::Fill),
                    ver_label(),
                ]
                .align_y(Alignment::Center);

                section = section.push(error_row.boxed()).push(primary_action_button(
                    theme_cfg,
                    font,
                    icons::REFRESH_SVG,
                    "Check for updates",
                    Message::CheckForUpdates,
                ));
            }
        }

        column![
            border_vert(1.0, theme_cfg.border),
            container(section)
                .padding(SIDEBAR_PADDING)
                .width(Length::Fill),
        ]
        .width(Length::Fill)
        .boxed()
    }
}

fn primary_action_button<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    icon_svg: &'static str,
    label: &'static str,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        container(
            row![
                icons::icon(icon_svg, 13.0, theme_cfg.background),
                Space::new().width(SPACE_MD),
                text(font, label).size(12).color(theme_cfg.background),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(BUTTON_PADDING_MD)
    .style(styles::primary_button(theme_cfg))
    .on_press(on_press)
    .boxed()
}

fn update_info_row<'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    info: &'a updater::UpdateInfo,
    current_version: &str,
) -> Element<'a, Message> {
    row![
        release_notes_link(
            font,
            format!("{} available", info.version),
            theme_cfg.primary,
            info.release_url.clone(),
        ),
        Space::new().width(Length::Fill),
        text(font, format!("v{current_version}"))
            .size(10)
            .color(theme_cfg.text_muted),
    ]
    .align_y(Alignment::Center)
    .boxed()
}

fn release_notes_link<'a>(
    font: iced::Font,
    label: String,
    color: Color,
    release_url: String,
) -> Element<'a, Message> {
    button(
        row![
            text(font, label).size(11).color(color),
            Space::new().width(SPACE_SM),
            icons::icon(icons::EXTERNAL_LINK_SVG, 10.0, color),
        ]
        .align_y(Alignment::Center),
    )
    .padding(0)
    .style(move |_theme, _status| button::Style {
        background: None,
        text_color: color,
        ..button::Style::default()
    })
    .interaction(iced::mouse::Interaction::Pointer)
    .on_press(Message::OpenUrl(release_url))
    .boxed()
}
