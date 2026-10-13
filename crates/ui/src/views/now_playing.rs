use core_engine::{AudioTechnicalInfo, Track, TrackRow};
use iced::border::Radius;
use iced::widget::{button, column, container, image, row, scrollable, stack, Space};
use iced::{Alignment, ContentFit, Element, Length};

use crate::components::{self, text};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub const DEFAULT_NOW_PLAYING_COVER_SIZE: u32 = 260;

/// Self-contained state and view component for the Now Playing / Lyrics screen.
#[derive(Debug, Clone)]
pub struct NowPlayingView {
    pub current_lyrics: Option<String>,
    pub cover_size: u32,
}

impl Default for NowPlayingView {
    fn default() -> Self {
        Self {
            current_lyrics: None,
            cover_size: DEFAULT_NOW_PLAYING_COVER_SIZE,
        }
    }
}

impl NowPlayingView {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cover_size(cover_size: u32) -> Self {
        Self {
            current_lyrics: None,
            cover_size,
        }
    }

    pub fn set_cover_size(&mut self, size: u32) {
        self.cover_size = size;
    }

    pub fn set_lyrics(&mut self, lyrics: Option<String>) {
        self.current_lyrics = lyrics;
    }

    pub fn clear(&mut self) {
        self.current_lyrics = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        current_track: Option<&'a Track>,
        current_cover: Option<&'a image::Handle>,
        blurred_cover: Option<&'a image::Handle>,
        next_track: Option<&'a TrackRow>,
        next_cover: Option<&'a image::Handle>,
        is_repeat: bool,
        is_shuffle: bool,
        tech_info: Option<&'a AudioTechnicalInfo>,
    ) -> Element<'a, Message> {
        let content: Element<'a, Message> = match current_track {
            Some(track) => {
                // 1. Left side: Cover art & clean floating lyrics (no background box)
                let cover_size = self.cover_size as f32;
                let cover_icon_size = (cover_size * 0.36).clamp(48.0, 100.0);
                let cover_art = components::cover_art(
                    theme_cfg,
                    current_cover,
                    cover_size,
                    Radius::from(6.0),
                    icons::ALBUM_SVG,
                    cover_icon_size,
                    theme_cfg.primary_color(),
                );

                let lyrics_body: Element<'a, Message> = if let Some(ref lyr) = self.current_lyrics {
                    text(theme_cfg, lyr)
                        .size(15)
                        .color(theme_cfg.text_color())
                        .into()
                } else {
                    column![
                        text(theme_cfg, "No lyrics found for this track.")
                            .size(14)
                            .color(theme_cfg.text_color()),
                        Space::with_height(4),
                        text(
                            theme_cfg,
                            "Place a .lrc file in the same directory or configure a lyrics plugin."
                        )
                        .size(12)
                        .color(theme_cfg.text_muted_color()),
                    ]
                    .into()
                };

                let lyrics_panel = scrollable(lyrics_body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(styles::scrollable_style(theme_cfg));

                let left_section = row![cover_art, Space::with_width(28), lyrics_panel,]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::Center);

                let title = text(theme_cfg, &track.title)
                    .size(17)
                    .color(theme_cfg.text_color())
                    .align_y(Alignment::Center);

                let artist_album_text =
                    text(theme_cfg, format!("{} • {}", track.artist, track.album))
                        .size(13)
                        .color(theme_cfg.text_muted_color());

                let mut details_items = Vec::new();
                if let Some(yr) = track.year {
                    details_items.push(format!("{}", yr));
                }
                if let Some(ref g) = track.genre {
                    if !g.trim().is_empty() {
                        details_items.push(g.clone());
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

                let details_row = text(theme_cfg, details_items.join("  ·  "))
                    .size(11)
                    .color(theme_cfg.text_muted_color());

                // Optional tags (Album Artist, Composer, Label)
                let mut extra_tags = column![].spacing(2);
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

                // Up Next Section Header (natural casing, font size 11 matching secondary headers)
                let mut up_next_tags = row![].spacing(6).align_y(Alignment::Center);
                if is_repeat {
                    up_next_tags = up_next_tags.push(
                        row![
                            icons::icon(icons::REPEAT_SVG, 10.0, theme_cfg.primary_color()),
                            text(theme_cfg, "Repeat")
                                .size(10)
                                .color(theme_cfg.primary_color()),
                        ]
                        .spacing(3)
                        .align_y(Alignment::Center),
                    );
                }
                if is_shuffle {
                    up_next_tags = up_next_tags.push(
                        row![
                            icons::icon(icons::SHUFFLE_SVG, 10.0, theme_cfg.primary_color()),
                            text(theme_cfg, "Shuffle")
                                .size(10)
                                .color(theme_cfg.primary_color()),
                        ]
                        .spacing(3)
                        .align_y(Alignment::Center),
                    );
                }

                let up_next_header = row![
                    text(theme_cfg, "Up Next")
                        .size(11)
                        .color(theme_cfg.primary_color()),
                    Space::with_width(Length::Fill),
                    up_next_tags,
                ]
                .align_y(Alignment::Center);

                // Up Next Content Item (unboxed, with thumbnail cover image)
                let next_track_item: Element<'a, Message> = match next_track {
                    Some(next) => {
                        let next_id = next.id;
                        let next_duration = next.formatted_duration();
                        let next_title = &next.title;
                        let next_subtitle = format!("{} • {}", next.artist, next.album);

                        let next_cover_widget = components::cover_art(
                            theme_cfg,
                            next_cover,
                            32.0,
                            Radius::default(),
                            icons::DISC_SVG,
                            16.0,
                            theme_cfg.primary_color(),
                        );

                        let next_info = column![
                            text(theme_cfg, next_title)
                                .size(12)
                                .color(theme_cfg.text_color()),
                            text(theme_cfg, next_subtitle)
                                .size(11)
                                .color(theme_cfg.text_muted_color()),
                        ]
                        .spacing(2)
                        .width(Length::Fill);

                        let duration_tag = text(theme_cfg, next_duration)
                            .size(11)
                            .color(theme_cfg.text_muted_color());

                        button(
                            row![next_cover_widget, next_info, duration_tag,]
                                .spacing(8)
                                .align_y(Alignment::Center),
                        )
                        .style(styles::tile(theme_cfg))
                        .on_press(Message::PlayTrack(next_id))
                        .width(Length::Fill)
                        .into()
                    }
                    None => text(theme_cfg, "No upcoming tracks")
                        .size(11)
                        .color(theme_cfg.text_muted_color())
                        .into(),
                };

                // Technical Details Section ("Properties", size 11, without icon)
                let tech_info_section: Element<'a, Message> = if let Some(tech) = tech_info {
                    let make_prop_row = |label: &'static str, value: String| {
                        row![
                            text(theme_cfg, label)
                                .size(11)
                                .color(theme_cfg.text_muted_color())
                                .width(80),
                            text(theme_cfg, value)
                                .size(11)
                                .color(theme_cfg.text_color()),
                        ]
                        .align_y(Alignment::Center)
                    };

                    let mut prop_col = column![
                        text(theme_cfg, "Properties")
                            .size(11)
                            .color(theme_cfg.primary_color()),
                        make_prop_row("Format", tech.format_name.clone()),
                        make_prop_row("Codec", tech.codec_name.clone()),
                        make_prop_row("Quality", tech.formatted_quality()),
                        make_prop_row(
                            "Channels",
                            format!("{} ({})", tech.channel_layout, tech.channels)
                        ),
                    ]
                    .spacing(4);

                    if let Some(kbps) = tech.bitrate_kbps {
                        prop_col =
                            prop_col.push(make_prop_row("Bitrate", format!("{} kbps", kbps)));
                    }

                    if tech.file_size_bytes > 0 {
                        prop_col =
                            prop_col.push(make_prop_row("File Size", tech.formatted_file_size()));
                    }

                    prop_col.into()
                } else {
                    column![].into()
                };

                // Card container matching the general app UI with balanced bottom padding
                let bottom_right_card = container(
                    column![
                        title,
                        artist_album_text,
                        details_row,
                        extra_tags,
                        components::border_vert(1.0, theme_cfg.border_color()),
                        tech_info_section,
                        components::border_vert(1.0, theme_cfg.border_color()),
                        up_next_header,
                        next_track_item,
                    ]
                    .spacing(8)
                    .width(Length::Fill),
                )
                .width(340)
                .padding(iced::Padding {
                    top: 14.0,
                    right: 14.0,
                    bottom: 18.0,
                    left: 14.0,
                })
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(theme_cfg.surface_color())),
                    border: iced::Border {
                        radius: Radius::from(6.0),
                        color: theme_cfg.border_color(),
                        width: 1.0,
                    },
                    text_color: Some(theme_cfg.text_color()),
                    ..Default::default()
                });

                let right_section = column![Space::with_height(Length::Fill), bottom_right_card,]
                    .width(340)
                    .height(Length::Fill)
                    .align_x(Alignment::End);

                row![left_section, Space::with_width(20), right_section,]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
            None => components::empty_state(
                theme_cfg,
                "No Track Playing",
                "Select a song from the library to view track details and lyrics",
            ),
        };

        // Very blurry version of the cover as the background
        let foreground = container(content)
            .padding(24)
            .width(Length::Fill)
            .height(Length::Fill);

        match blurred_cover {
            Some(handle) => {
                let bg_img = image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(ContentFit::Cover);

                let dark_tint = container("")
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgba(
                            0.0, 0.0, 0.0, 0.65,
                        ))),
                        ..Default::default()
                    });

                stack![bg_img, dark_tint, foreground]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
            None => container(foreground)
                .style(styles::main_background(theme_cfg))
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        }
    }
}
