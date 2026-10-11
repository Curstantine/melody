use iced::border::Radius;
use iced::widget::{column, container, image, row, scrollable, Space};
use iced::{Alignment, Element, Length};
use player_core::Track;

use crate::components::{self, text};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    current_track: Option<&'a Track>,
    current_cover: Option<&'a image::Handle>,
    lyrics: Option<&'a str>,
) -> Element<'a, Message> {
    let content = match current_track {
        Some(track) => {
            let cover_art = components::cover_art(
                theme_cfg,
                current_cover,
                220.0,
                Radius::from(8.0),
                icons::ALBUM_SVG,
                80.0,
                theme_cfg.primary_color(),
            );

            let stars = row![
                icons::icon(icons::STAR_FILLED_SVG, 14.0, theme_cfg.primary_color()),
                icons::icon(icons::STAR_FILLED_SVG, 14.0, theme_cfg.primary_color()),
                icons::icon(icons::STAR_FILLED_SVG, 14.0, theme_cfg.primary_color()),
                icons::icon(icons::STAR_FILLED_SVG, 14.0, theme_cfg.primary_color()),
                icons::icon(icons::STAR_OUTLINE_SVG, 14.0, theme_cfg.text_muted_color()),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            let mut details_items = Vec::new();
            if let Some(yr) = track.year {
                details_items.push(format!("{}", yr));
            }
            if let Some(ref genre) = track.genre {
                if !genre.trim().is_empty() {
                    details_items.push(genre.clone());
                }
            }
            if let Some(t_num) = track.track_num {
                if let Some(d_num) = track.disc_num {
                    details_items.push(format!("Disc {} · Track {}", d_num, t_num));
                } else {
                    details_items.push(format!("Track {}", t_num));
                }
            }
            details_items.push(track.formatted_duration());

            let mut extra_tags = column![].spacing(3).align_x(Alignment::Center);
            if let Some(ref alb_artist) = track.album_artist {
                if !alb_artist.trim().is_empty() && alb_artist != &track.artist {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, format!("Album Artist: {}", alb_artist))
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
            }
            if let Some(ref comp) = track.composer {
                if !comp.trim().is_empty() {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, format!("Composer: {}", comp))
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
            }
            if let Some(ref lbl) = track.label {
                if !lbl.trim().is_empty() {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, format!("Label: {}", lbl))
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
            }
            if let Some(ref mb_id) = track.musicbrainz_track_id {
                if !mb_id.trim().is_empty() {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, format!("MusicBrainz Track: {}", mb_id))
                            .size(10)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
            }

            let metadata = column![
                text(theme_cfg, &track.title)
                    .size(22)
                    .color(theme_cfg.text_color()),
                text(theme_cfg, format!("{} • {}", track.artist, track.album))
                    .size(15)
                    .color(theme_cfg.text_muted_color()),
                stars,
                text(theme_cfg, details_items.join("  ·  "))
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                extra_tags,
            ]
            .spacing(6)
            .align_x(Alignment::Center);

            let lyrics_body = if let Some(lyr) = lyrics {
                text(theme_cfg, lyr).size(15).color(theme_cfg.text_color())
            } else {
                text(theme_cfg,"No lyrics found for this track.\nPlace a .lrc file in the same directory or configure a lyrics plugin.")
                    .size(14)
                    .color(theme_cfg.text_muted_color())
            };

            column![
                cover_art,
                Space::with_height(16),
                metadata,
                Space::with_height(20),
                scrollable(lyrics_body)
                    .height(Length::Fill)
                    .style(styles::scrollable_style(theme_cfg)),
            ]
            .spacing(12)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill)
        }
        None => column![
            text(theme_cfg, "No Track Playing")
                .size(20)
                .color(theme_cfg.text_muted_color()),
            text(
                theme_cfg,
                "Select a song from the library to view track details and lyrics"
            )
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
