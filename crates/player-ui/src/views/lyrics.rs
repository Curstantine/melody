use iced::widget::{column, container, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use player_core::Track;

use crate::message::Message;
use crate::theme::ThemeConfig;

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    current_track: Option<&'a Track>,
    lyrics: Option<&'a str>,
) -> Element<'a, Message> {
    let content = match current_track {
        Some(track) => {
            let art_placeholder = text("🎵")
                .size(72)
                .color(theme_cfg.primary_color());

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
                art_placeholder,
                Space::with_height(16),
                metadata,
                Space::with_height(24),
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
