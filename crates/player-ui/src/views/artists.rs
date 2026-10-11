use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use player_core::ArtistSummary;

use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    artists: &'a [ArtistSummary],
) -> Element<'a, Message> {
    if artists.is_empty() {
        return container(
            column![
                text("No artists indexed")
                    .size(16)
                    .color(theme_cfg.text_color()),
                text("Scan a folder containing music to populate the artist list")
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

    let mut artist_list = column![].spacing(6);

    for artist in artists {
        let title_block = column![
            text(&artist.artist)
                .size(14)
                .color(theme_cfg.text_color()),
            text(format!(
                "{} tracks • {} albums",
                artist.track_count, artist.album_count
            ))
            .size(12)
            .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill);

        let artist_row = row![
            text("👤").size(22),
            Space::with_width(8),
            title_block,
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([8, 14]);

        let row_btn = button(artist_row)
            .style(styles::ghost_button(theme_cfg, false))
            .width(Length::Fill)
            .on_press(Message::SelectArtist(artist.artist.clone()));

        artist_list = artist_list.push(row_btn);
    }

    let content = column![
        text("Artists")
            .size(18)
            .color(theme_cfg.text_color()),
        Space::with_height(8),
        scrollable(artist_list).height(Length::Fill),
    ]
    .spacing(8)
    .padding(16)
    .height(Length::Fill)
    .width(Length::Fill);

    content.into()
}
