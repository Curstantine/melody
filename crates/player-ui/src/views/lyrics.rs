use iced::border::Radius;
use iced::widget::{column, container, image, scrollable, text, Space};
use iced::{Alignment, Border, ContentFit, Element, Length};
use player_core::Track;

use crate::icons;
use crate::message::Message;
use crate::theme::ThemeConfig;

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    current_track: Option<&'a Track>,
    current_cover: Option<&'a image::Handle>,
    lyrics: Option<&'a str>,
) -> Element<'a, Message> {
    let content = match current_track {
        Some(track) => {
            let cover_art: Element<'a, Message> = match current_cover {
                Some(handle) => container(
                    image(handle.clone())
                        .width(Length::Fixed(220.0))
                        .height(Length::Fixed(220.0))
                        .content_fit(ContentFit::Contain),
                )
                .width(220)
                .height(220)
                .style(|_theme| container::Style {
                    border: Border {
                        radius: Radius::from(8.0),
                        color: theme_cfg.border_color(),
                        width: 1.0,
                    },
                    ..Default::default()
                })
                .into(),
                None => container(
                    icons::icon(icons::ALBUM_SVG, 80.0, theme_cfg.primary_color()),
                )
                .width(220)
                .height(220)
                .center_x(220)
                .center_y(220)
                .style(|_theme| container::Style {
                    background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                    border: Border {
                        radius: Radius::from(8.0),
                        color: theme_cfg.border_color(),
                        width: 1.0,
                    },
                    ..Default::default()
                })
                .into(),
            };

            let metadata = column![
                text(&track.title)
                    .size(24)
                    .color(theme_cfg.text_color()),
                text(format!("{} — {}", track.artist, track.album))
                    .size(16)
                    .color(theme_cfg.text_muted_color()),
                text(format!("Duration: {}", track.formatted_duration()))
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(6)
            .align_x(Alignment::Center);

            let lyrics_body = if let Some(lyr) = lyrics {
                text(lyr)
                    .size(15)
                    .color(theme_cfg.text_color())
            } else {
                text("No lyrics found for this track.\nPlace a .lrc file in the same directory or configure a lyrics plugin.")
                    .size(14)
                    .color(theme_cfg.text_muted_color())
            };

            column![
                cover_art,
                Space::with_height(16),
                metadata,
                Space::with_height(20),
                scrollable(lyrics_body).height(Length::Fill),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
        }
        None => column![
            text("No Track Playing")
                .size(20)
                .color(theme_cfg.text_muted_color()),
            text("Select a song from the library to view track details and lyrics")
                .size(14)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(8)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill),
    };

    container(content)
        .padding(30)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}
