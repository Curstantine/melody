use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use player_core::{format_duration, AlbumSummary};

use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    albums: &'a [AlbumSummary],
) -> Element<'a, Message> {
    if albums.is_empty() {
        return container(
            column![
                text("No albums indexed")
                    .size(16)
                    .color(theme_cfg.text_color()),
                text("Scan a folder containing music to populate the album catalog")
                    .size(13)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .padding(40)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into();
    }

    let mut album_list = column![].spacing(6);

    for album in albums {
        let title_block = column![
            text(&album.album)
                .size(14)
                .color(theme_cfg.text_color()),
            text(&album.artist)
                .size(12)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill);

        let details = column![
            text(format!("{} tracks", album.track_count))
                .size(12)
                .color(theme_cfg.text_muted_color()),
            text(format_duration(album.total_duration_ms))
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .align_x(Alignment::End)
        .width(100);

        let album_row = row![
            icons::icon(icons::ALBUM_SVG, 24.0, theme_cfg.primary_color()),
            Space::with_width(8),
            title_block,
            details,
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([8, 14]);

        let row_btn = button(album_row)
            .style(styles::ghost_button(theme_cfg, false))
            .width(Length::Fill)
            .on_press(Message::SelectAlbum(album.album.clone()));

        album_list = album_list.push(row_btn);
    }

    let content = column![
        text("Albums")
            .size(18)
            .color(theme_cfg.text_color()),
        Space::with_height(8),
        scrollable(album_list).height(Length::Fill),
    ]
    .spacing(8)
    .padding(16)
    .height(Length::Fill)
    .width(Length::Fill);

    content.into()
}
