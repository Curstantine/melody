use iced::widget::{button, column, container, row, Space, Widget};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use components::{self, icons, styles, text};
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_update_section<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let current_version = env!("CARGO_PKG_VERSION");

        let content = match &self.update_status {
            updater::UpdateStatus::Available(info) => {
                let update_btn = button(
                    row![
                        icons::icon(icons::DOWNLOAD_SVG, 13.0, theme_cfg.background),
                        Space::new().width(SPACE_MD),
                        text(font, "Update & restart")
                            .size(12)
                            .color(theme_cfg.background),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(BUTTON_PADDING_MD)
                .style(styles::primary_button(theme_cfg))
                .on_press(Message::StartUpdateDownload(info.clone()));

                let status_label = row![
                    icons::icon(icons::DOWNLOAD_SVG, 11.0, theme_cfg.primary),
                    Space::new().width(SPACE_SM),
                    text(font, format!("{} ready", info.version))
                        .size(11)
                        .color(theme_cfg.primary),
                ]
                .align_y(Alignment::Center);

                let release_link = button(
                    row![
                        text(font, "View release notes")
                            .size(10)
                            .color(theme_cfg.text_muted),
                        Space::new().width(SPACE_SM),
                        icons::icon(icons::EXTERNAL_LINK_SVG, 10.0, theme_cfg.text_muted),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([2, 0])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::OpenUrl(info.release_url.clone()));

                let ver_label = text(font, format!("v{current_version}"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![
                    update_btn,
                    column![status_label, release_link, ver_label].spacing(2),
                ]
                .spacing(SPACE_SM)
                .width(Length::Fill)
            }
            updater::UpdateStatus::Downloading { info, percent } => {
                let downloading_btn = button(
                    row![
                        icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.primary),
                        Space::new().width(SPACE_MD),
                        text(font, format!("Downloading {}%...", percent.clamp(&0, &100)),)
                            .size(12)
                            .color(theme_cfg.primary),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, false));

                let info_label = text(font, format!("Downloading {}", info.version))
                    .size(11)
                    .color(theme_cfg.text);

                let ver_label = text(font, format!("v{current_version}"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![downloading_btn, column![info_label, ver_label].spacing(2),]
                    .spacing(SPACE_SM)
                    .width(Length::Fill)
            }
            updater::UpdateStatus::ReadyToRestart { info } => {
                let restart_btn = button(
                    row![
                        icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.background),
                        Space::new().width(SPACE_MD),
                        text(font, "Restart now")
                            .size(12)
                            .color(theme_cfg.background),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(BUTTON_PADDING_MD)
                .style(styles::primary_button(theme_cfg))
                .on_press(Message::ApplyUpdateAndRestart);

                let status_label = row![
                    icons::icon(icons::DOWNLOAD_SVG, 11.0, theme_cfg.accent),
                    Space::new().width(SPACE_SM),
                    text(font, format!("{} downloaded", info.version))
                        .size(11)
                        .color(theme_cfg.accent),
                ]
                .align_y(Alignment::Center);

                let release_link = button(
                    row![
                        text(font, "View release notes")
                            .size(10)
                            .color(theme_cfg.text_muted),
                        Space::new().width(SPACE_SM),
                        icons::icon(icons::EXTERNAL_LINK_SVG, 10.0, theme_cfg.text_muted),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([2, 0])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::OpenUrl(info.release_url.clone()));

                let ver_label = text(font, format!("v{current_version}"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![
                    restart_btn,
                    column![status_label, release_link, ver_label].spacing(2),
                ]
                .spacing(SPACE_SM)
                .width(Length::Fill)
            }
            updater::UpdateStatus::Checking => {
                let checking_btn = button(
                    row![
                        icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text_muted),
                        Space::new().width(SPACE_MD),
                        text(font, "Checking for updates...")
                            .size(12)
                            .color(theme_cfg.text_muted),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, false));

                let ver_label = text(font, format!("v{current_version}"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![checking_btn, ver_label]
                    .spacing(SPACE_SM)
                    .width(Length::Fill)
            }
            updater::UpdateStatus::UpToDate => {
                let ver_label = text(font, format!("v{current_version} • Up to date"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![ver_label].width(Length::Fill)
            }
            updater::UpdateStatus::Idle => {
                let ver_label = text(font, format!("v{current_version}"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![ver_label].width(Length::Fill)
            }
            updater::UpdateStatus::Failed(_) => {
                let check_btn = button(
                    row![
                        icons::icon(icons::REFRESH_SVG, 13.0, theme_cfg.text),
                        Space::new().width(SPACE_MD),
                        text(font, "Check for updates")
                            .size(12)
                            .color(theme_cfg.text),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(BUTTON_PADDING_MD)
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::CheckForUpdates);

                let ver_label = text(font, format!("v{current_version} • Check failed"))
                    .size(10)
                    .color(theme_cfg.text_muted);

                column![check_btn, ver_label]
                    .spacing(SPACE_SM)
                    .width(Length::Fill)
            }
        };

        container(content)
            .padding(SIDEBAR_PADDING)
            .width(Length::Fill)
            .boxed()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // 1. Appearance Section
    // ─────────────────────────────────────────────────────────────────────────────
}
