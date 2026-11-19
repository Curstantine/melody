use iced::widget::{column, container, row, Space, Widget};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use component::{self, border_hori, button, icons, styles, text};
use config::SettingsTab;
use theme::ThemeConfig;

use super::*;

impl SettingsView {
    pub(super) fn view_sidebar<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
    ) -> Element<'a, Message> {
        let sidebar_title = column![
            text(font, "Preferences").size(11).color(theme_cfg.primary),
            text(font, "System configuration")
                .size(11)
                .color(theme_cfg.text_muted),
        ]
        .spacing(SIDEBAR_HEADER_SPACING)
        .padding(SIDEBAR_PADDING);

        let nav_item = |tab: SettingsTab,
                        icon_svg: &'static str,
                        title: &'static str,
                        subtitle: &'static str| {
            let is_active = self.active_tab == tab;
            let color = if is_active {
                theme_cfg.primary
            } else {
                theme_cfg.text
            };

            button(
                row![
                    icons::icon(icon_svg, 16.0, color),
                    Space::new().width(SPACE_LG),
                    column![
                        text(font, title).size(13).color(color),
                        text(font, subtitle).size(10).color(theme_cfg.text_muted),
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

        let update_section = self.view_update_section(theme_cfg, font);

        let nav_col = column![
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
            ),
            Space::new().height(Length::Fill),
            update_section,
        ]
        .width(SIDEBAR_WIDTH)
        .height(Length::Fill);

        container(row![nav_col, border_hori(1.0, theme_cfg.border)])
            .width(Length::Fixed(SIDEBAR_WIDTH + 1.0))
            .height(Length::Fill)
            .style(styles::sidebar_container(theme_cfg))
            .boxed()
    }
}
