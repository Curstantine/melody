use core_engine::{AudioTechnicalInfo, Track, TrackRow};
use iced::widget::{button, column, container, image, row, scrollable, stack, Space, Widget};
use iced::{Alignment, ContentFit, Element, Length};

use crate::components::{self, text};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub const DEFAULT_NOW_PLAYING_COVER_SIZE: u32 = 260;

/// Pre-rendered display strings so Now Playing rendering performs no string allocations per frame.
#[derive(Debug, Default, Clone)]
pub struct CachedNowPlaying {
    pub track_id: Option<i64>,
    pub artist_album: String,
    pub details_row: String,
    pub album_artist_tag: Option<String>,
    pub composer_tag: Option<String>,
    pub next_track_id: Option<i64>,
    pub next_subtitle: String,
    pub tech_properties: Vec<(&'static str, String)>,
}

/// Self-contained state and view component for the Now Playing / Lyrics screen.
#[derive(Debug, Clone)]
pub struct NowPlayingView {
    pub current_lyrics: Option<String>,
    pub cover_size: u32,
    pub cached: CachedNowPlaying,
}

impl Default for NowPlayingView {
    fn default() -> Self {
        Self {
            current_lyrics: None,
            cover_size: DEFAULT_NOW_PLAYING_COVER_SIZE,
            cached: CachedNowPlaying::default(),
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
            cached: CachedNowPlaying::default(),
        }
    }

    pub fn set_cover_size(&mut self, size: u32) {
        self.cover_size = size;
    }

    pub fn set_lyrics(&mut self, lyrics: Option<String>) {
        self.current_lyrics = lyrics;
    }

    pub fn update_track(&mut self, track: Option<&Track>) {
        if let Some(track) = track {
            if self.cached.track_id == Some(track.id) {
                return;
            }
            self.cached.track_id = Some(track.id);
            self.cached.artist_album = format!("{} • {}", track.artist, track.album);

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
            self.cached.details_row = details_items.join("  ·  ");

            self.cached.album_artist_tag = track.album_artist.as_ref().and_then(|alb_artist| {
                if !alb_artist.trim().is_empty() && alb_artist != &track.artist {
                    Some(format!("Album artist: {}", alb_artist))
                } else {
                    None
                }
            });

            self.cached.composer_tag = track.composer.as_ref().and_then(|comp| {
                if !comp.trim().is_empty() {
                    Some(format!("Composer: {}", comp))
                } else {
                    None
                }
            });
        } else {
            self.cached.track_id = None;
            self.cached.artist_album.clear();
            self.cached.details_row.clear();
            self.cached.album_artist_tag = None;
            self.cached.composer_tag = None;
            self.cached.tech_properties.clear();
        }
    }

    pub fn update_next_track(&mut self, next: Option<&TrackRow>) {
        if let Some(next) = next {
            if self.cached.next_track_id == Some(next.id) {
                return;
            }
            self.cached.next_track_id = Some(next.id);
            self.cached.next_subtitle = format!("{} • {}", next.artist, next.album);
        } else {
            self.cached.next_track_id = None;
            self.cached.next_subtitle.clear();
        }
    }

    pub fn update_tech_info(&mut self, tech: Option<&AudioTechnicalInfo>) {
        if let Some(tech) = tech {
            let mut props = Vec::with_capacity(6);
            props.push(("Format", tech.format_name.clone()));
            props.push(("Codec", tech.codec_name.clone()));
            props.push(("Quality", tech.formatted_quality()));
            props.push((
                "Channels",
                format!("{} ({})", tech.channel_layout, tech.channels),
            ));
            if let Some(kbps) = tech.bitrate_kbps {
                props.push(("Bitrate", format!("{} kbps", kbps)));
            }
            if tech.file_size_bytes > 0 {
                props.push(("File size", tech.formatted_file_size()));
            }
            self.cached.tech_properties = props;
        } else {
            self.cached.tech_properties.clear();
        }
    }

    pub fn clear(&mut self) {
        self.current_lyrics = None;
        self.update_track(None);
        self.update_next_track(None);
        self.update_tech_info(None);
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
        _tech_info: Option<&'a AudioTechnicalInfo>,
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
                    theme_cfg.radius_md(),
                    icons::ALBUM_SVG,
                    cover_icon_size,
                    theme_cfg.primary_color(),
                );

                let lyrics_body: Element<'a, Message> = if let Some(ref lyr) = self.current_lyrics {
                    text(theme_cfg, lyr)
                        .size(15)
                        .color(theme_cfg.text_color())
                        .boxed()
                } else {
                    column![
                        text(theme_cfg, "No lyrics found for this track.")
                            .size(14)
                            .color(theme_cfg.text_color()),
                        Space::new().height(4),
                        text(
                            theme_cfg,
                            "Place a .lrc file in the same directory or configure a lyrics plugin."
                        )
                        .size(12)
                        .color(theme_cfg.text_muted_color()),
                    ]
                    .boxed()
                };

                let lyrics_panel = scrollable(lyrics_body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(styles::scrollable_style(theme_cfg));

                let left_section = row![cover_art, Space::new().width(28), lyrics_panel,]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::Center);

                let title = text(theme_cfg, &track.title)
                    .size(17)
                    .color(theme_cfg.text_color())
                    .align_y(Alignment::Center);

                let artist_album_str = if !self.cached.artist_album.is_empty() {
                    self.cached.artist_album.as_str()
                } else {
                    track.album.as_str()
                };
                let artist_album_text = text(theme_cfg, artist_album_str)
                    .size(13)
                    .color(theme_cfg.text_muted_color());

                let details_row = text(theme_cfg, self.cached.details_row.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted_color());

                // Optional tags (Album Artist, Composer, Label)
                let mut extra_tags = column![].spacing(2);
                if let Some(ref alb_artist) = self.cached.album_artist_tag {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, alb_artist.as_str())
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
                }
                if let Some(ref comp) = self.cached.composer_tag {
                    extra_tags = extra_tags.push(
                        text(theme_cfg, comp.as_str())
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    );
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
                    text(theme_cfg, "Up next")
                        .size(11)
                        .color(theme_cfg.primary_color()),
                    Space::new().width(Length::Fill),
                    up_next_tags,
                ]
                .align_y(Alignment::Center);

                // Up Next Content Item (unboxed, with thumbnail cover image)
                let next_track_item: Element<'a, Message> = match next_track {
                    Some(next) => {
                        let next_id = next.id;
                        let next_duration = next.formatted_duration();
                        let next_title = &next.title;
                        let next_subtitle = if !self.cached.next_subtitle.is_empty() {
                            self.cached.next_subtitle.as_str()
                        } else {
                            next.album.as_ref()
                        };

                        let next_cover_widget = components::cover_art(
                            theme_cfg,
                            next_cover,
                            32.0,
                            theme_cfg.radius_sm(),
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
                        .boxed()
                    }
                    None => text(theme_cfg, "No upcoming tracks")
                        .size(11)
                        .color(theme_cfg.text_muted_color())
                        .boxed(),
                };

                // Technical Details Section ("Properties", size 11, without icon)
                let tech_info_section: Element<'a, Message> =
                    if !self.cached.tech_properties.is_empty() {
                        let mut prop_col = column![text(theme_cfg, "Properties")
                            .size(11)
                            .color(theme_cfg.primary_color()),]
                        .spacing(4);

                        for (label, val) in &self.cached.tech_properties {
                            prop_col = prop_col.push(
                                row![
                                    text(theme_cfg, *label)
                                        .size(11)
                                        .color(theme_cfg.text_muted_color())
                                        .width(80),
                                    text(theme_cfg, val.as_str())
                                        .size(11)
                                        .color(theme_cfg.text_color()),
                                ]
                                .align_y(Alignment::Center)
                                .boxed(),
                            );
                        }

                        prop_col.boxed()
                    } else {
                        Space::new().boxed()
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
                        radius: theme_cfg.radius_md(),
                        color: theme_cfg.border_color(),
                        width: 1.0,
                    },
                    text_color: Some(theme_cfg.text_color()),
                    ..Default::default()
                });

                let right_section = column![Space::new().height(Length::Fill), bottom_right_card,]
                    .width(340)
                    .height(Length::Fill)
                    .align_x(Alignment::End);

                row![left_section, Space::new().width(20), right_section,]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .boxed()
            }
            None => components::empty_state(
                theme_cfg,
                "No track playing",
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
                    .boxed()
            }
            None => container(foreground)
                .style(styles::main_background(theme_cfg))
                .width(Length::Fill)
                .height(Length::Fill)
                .boxed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeConfig;
    use core_engine::{AudioTechnicalInfo, Track, TrackRow};

    #[test]
    fn test_now_playing_empty_state_renders() {
        let theme_cfg = ThemeConfig::default();
        let mut now_playing_view = NowPlayingView::new();
        assert_eq!(now_playing_view.cover_size, 260);
        now_playing_view.set_cover_size(300);
        assert_eq!(now_playing_view.cover_size, 300);

        let _element =
            now_playing_view.view(&theme_cfg, None, None, None, None, None, false, false, None);
    }

    #[test]
    fn test_now_playing_with_track_and_next_track() {
        let theme_cfg = ThemeConfig::default();

        let track = Track {
            id: 1,
            file_path: "/music/track1.flac".to_string(),
            title: "Neon Dreams".to_string(),
            artist: "Synthwave Master".to_string(),
            album_artist: None,
            album: "Future Echoes".to_string(),
            genre: Some("Synthwave".to_string()),
            duration_ms: 215_000,
            track_num: Some(3),
            disc_num: Some(1),
            year: Some(2025),
            composer: Some("Composer X".to_string()),
            label: Some("Vibe Records".to_string()),
            musicbrainz_track_id: None,
            musicbrainz_artist_id: None,
            musicbrainz_album_id: None,
            added_timestamp: 1700000000,
        };

        let next_track = TrackRow {
            id: 2,
            file_path: "/music/track2.flac".to_string(),
            title: "Night Cruise".to_string(),
            artist: "Synthwave Master".into(),
            album: "Future Echoes".into(),
            duration_ms: 198_000,
            duration_label: "3:18".to_string(),
            year: None,
            track_num: None,
            disc_num: None,
            album_artist: None,
            genre: None,
        };

        let tech_info = AudioTechnicalInfo {
            format_name: "FLAC Lossless".to_string(),
            codec_name: "FLAC".to_string(),
            sample_rate_hz: 48000,
            bit_depth: Some(24),
            channels: 2,
            channel_layout: "Stereo".to_string(),
            bitrate_kbps: Some(960),
            file_size_bytes: 25_800_000,
        };

        let lyrics_text = "[00:00.00] Cruising down the neon street\n[00:15.00] Bass in the night";
        let mut now_playing_view = NowPlayingView::new();
        now_playing_view.set_lyrics(Some(lyrics_text.to_string()));
        now_playing_view.update_track(Some(&track));
        now_playing_view.update_next_track(Some(&next_track));
        now_playing_view.update_tech_info(Some(&tech_info));

        let _element = now_playing_view.view(
            &theme_cfg,
            Some(&track),
            None,
            None,
            Some(&next_track),
            None,
            true,
            true,
            Some(&tech_info),
        );
    }
}
