use iced::widget::{button, column, container, image, row, scrollable, stack, Space, Widget};
use iced::{Alignment, ContentFit, Element, Length};
use metadata::AudioTechnicalInfo;
use model::{ParsedLyrics, Track, TrackRow};

use crate::message::Message;
use component::{self, icons, styles, text};
use theme::ThemeConfig;

use super::ScrollOffset;

pub const INFO_PANEL_WIDTH: f32 = 340.0;

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
#[derive(Debug, Clone, Default)]
pub struct NowPlayingView {
    pub current_lyrics: Option<String>,
    pub parsed_lyrics: Option<ParsedLyrics>,
    pub last_active_line: Option<usize>,
    pub cached: CachedNowPlaying,
    pub scroll_offset: ScrollOffset,
}

impl NowPlayingView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Estimated vertical stride (in logical pixels) per synced lyric line for auto-scrolling.
    pub fn line_stride(lyrics_size: u16) -> f32 {
        (lyrics_size as f32 * 2.0).max(24.0)
    }

    pub fn set_lyrics(&mut self, lyrics: Option<String>) {
        self.parsed_lyrics = lyrics.as_deref().map(ParsedLyrics::parse);
        self.current_lyrics = lyrics;
        self.last_active_line = None;
        self.scroll_offset.set(0.0);
    }

    /// Updates the active synced lyric line for `position_ms` and returns `Some(idx)`
    /// only when the active line changed.
    pub fn update_active_lyric_line(&mut self, position_ms: u64) -> Option<usize> {
        let active = self
            .parsed_lyrics
            .as_ref()
            .and_then(|p| p.active_line_index(position_ms));
        if active != self.last_active_line {
            self.last_active_line = active;
            active
        } else {
            None
        }
    }

    pub fn update_track(&mut self, track: Option<&Track>) {
        if let Some(track) = track {
            if self.cached.track_id == Some(track.id) {
                return;
            }
            self.scroll_offset.set(0.0);
            self.cached.track_id = Some(track.id);
            self.cached.artist_album = format!("{} • {}", track.artist, track.album);

            let mut details_items = Vec::new();
            if let Some(yr) = track.year {
                details_items.push(format!("{}", yr));
            }
            if let Some(ref g) = track.genre {
                let formatted = model::model::format_multi_value(g);
                if !formatted.trim().is_empty() {
                    details_items.push(formatted.into_owned());
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
        self.parsed_lyrics = None;
        self.last_active_line = None;
        self.scroll_offset.set(0.0);
        self.update_track(None);
        self.update_next_track(None);
        self.update_tech_info(None);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        fonts: &font::FontConfig,
        current_track: Option<&'a Track>,
        current_cover: Option<&'a image::Handle>,
        blurred_cover: Option<&'a image::Handle>,
        next_track: Option<&'a TrackRow>,
        repeat_mode: model::RepeatMode,
        is_shuffle: bool,
        position_ms: u64,
        _tech_info: Option<&'a AudioTechnicalInfo>,
    ) -> Element<'a, Message> {
        let content: Element<'a, Message> = match current_track {
            Some(track) => {
                let cover_icon_size = (INFO_PANEL_WIDTH * 0.36).clamp(48.0, 100.0);
                let cover_art = component::cover_art(
                    theme_cfg,
                    current_cover,
                    INFO_PANEL_WIDTH,
                    theme_cfg.border_radius.md,
                    icons::ALBUM_SVG,
                    cover_icon_size,
                    theme_cfg.primary,
                );

                let font = fonts.primary_font();
                let lyrics_font = fonts.lyrics_font();
                let base_size = u32::from(fonts.lyrics_size);
                let inactive_size = base_size.saturating_sub(1).max(10);
                let active_size = base_size.saturating_add(1);

                let lyrics_body: Element<'a, Message> = match &self.parsed_lyrics {
                    Some(ParsedLyrics::Synced(lines)) => {
                        let active_idx = self
                            .parsed_lyrics
                            .as_ref()
                            .and_then(|p| p.active_line_index(position_ms));

                        let mut col = column![].spacing(6).width(Length::Fill);
                        for (idx, line) in lines.iter().enumerate() {
                            let is_active = active_idx == Some(idx);
                            let display_text = if line.text.is_empty() {
                                "•••"
                            } else {
                                line.text.as_str()
                            };
                            let line_color = if is_active {
                                theme_cfg.primary
                            } else {
                                theme_cfg.text_muted
                            };
                            let line_size = if is_active {
                                active_size
                            } else {
                                inactive_size
                            };
                            let lyric_btn = button(
                                text(fonts.primary_font(), display_text)
                                    .size(line_size)
                                    .color(line_color)
                                    .font(lyrics_font),
                            )
                            .on_press(Message::SeekToMs(line.time_ms))
                            .padding([2, 4])
                            .width(Length::Fill)
                            .style(styles::ghost_button(theme_cfg, false));
                            col = col.push(lyric_btn);
                        }
                        col.boxed()
                    }
                    Some(ParsedLyrics::Plain(lyr)) => text(fonts.primary_font(), lyr)
                        .size(base_size)
                        .color(theme_cfg.text)
                        .font(lyrics_font)
                        .boxed(),
                    None => {
                        if let Some(ref lyr) = self.current_lyrics {
                            text(fonts.primary_font(), lyr)
                                .size(base_size)
                                .color(theme_cfg.text)
                                .font(lyrics_font)
                                .boxed()
                        } else {
                            column![
                                text(fonts.primary_font(), "No lyrics found for this track.")
                                    .size(14)
                                    .color(theme_cfg.text),
                                Space::new().height(4),
                                text(
                                    font,
                                    "Place a .lrc file in the same directory or embed lyrics in the audio file."
                                )
                                .size(12)
                                .color(theme_cfg.text_muted),
                            ]
                            .boxed()
                        }
                    }
                };

                let scroll_offset = &self.scroll_offset;
                let lyrics_panel = scrollable(lyrics_body)
                    .id(iced::widget::Id::new("lyrics_scroll"))
                    .on_scroll(move |scroll| {
                        scroll_offset.set(scroll.viewport.absolute_offset().y);
                        None::<Message>
                    })
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(styles::scrollable_style(theme_cfg));

                let title = text(fonts.primary_font(), &track.title)
                    .size(17)
                    .color(theme_cfg.text)
                    .align_y(Alignment::Center);

                let artist_album_str = if !self.cached.artist_album.is_empty() {
                    self.cached.artist_album.as_str()
                } else {
                    track.album.as_str()
                };
                let artist_album_text = text(fonts.primary_font(), artist_album_str)
                    .size(13)
                    .color(theme_cfg.text_muted);

                let details_row = text(fonts.primary_font(), self.cached.details_row.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted);

                // Optional tags (Album Artist, Composer, Label)
                let mut extra_tags = column![].spacing(2);
                if let Some(ref alb_artist) = self.cached.album_artist_tag {
                    extra_tags = extra_tags.push(
                        text(fonts.primary_font(), alb_artist.as_str())
                            .size(11)
                            .color(theme_cfg.text_muted),
                    );
                }
                if let Some(ref comp) = self.cached.composer_tag {
                    extra_tags = extra_tags.push(
                        text(fonts.primary_font(), comp.as_str())
                            .size(11)
                            .color(theme_cfg.text_muted),
                    );
                }

                // Up Next Section Header (natural casing, font size 11 matching secondary headers)
                let mut up_next_tags = row![].spacing(6).align_y(Alignment::Center);
                if repeat_mode != model::RepeatMode::Off {
                    let repeat_svg = if repeat_mode == model::RepeatMode::One {
                        icons::REPEAT_ONE_SVG
                    } else {
                        icons::REPEAT_SVG
                    };
                    up_next_tags = up_next_tags.push(
                        row![
                            icons::icon(repeat_svg, 10.0, theme_cfg.primary),
                            text(fonts.primary_font(), repeat_mode.label())
                                .size(10)
                                .color(theme_cfg.primary),
                        ]
                        .spacing(3)
                        .align_y(Alignment::Center),
                    );
                }
                if is_shuffle {
                    up_next_tags = up_next_tags.push(
                        row![
                            icons::icon(icons::SHUFFLE_SVG, 10.0, theme_cfg.primary),
                            text(fonts.primary_font(), "Shuffle")
                                .size(10)
                                .color(theme_cfg.primary),
                        ]
                        .spacing(3)
                        .align_y(Alignment::Center),
                    );
                }

                let up_next_header = row![
                    text(fonts.primary_font(), "Up next")
                        .size(11)
                        .color(theme_cfg.primary),
                    Space::new().width(Length::Fill),
                    up_next_tags,
                ]
                .align_y(Alignment::Center);

                // Up Next Content Item
                let next_track_item: Element<'a, Message> = match next_track {
                    Some(next) => {
                        let next_id = next.id;
                        let next_duration = next.duration_label.as_str();
                        let next_title = &next.title;
                        let next_subtitle = if !self.cached.next_subtitle.is_empty() {
                            self.cached.next_subtitle.as_str()
                        } else {
                            next.album.as_ref()
                        };

                        let next_info = column![
                            text(fonts.primary_font(), next_title)
                                .size(12)
                                .color(theme_cfg.text),
                            text(fonts.primary_font(), next_subtitle)
                                .size(11)
                                .color(theme_cfg.text_muted),
                        ]
                        .spacing(2)
                        .width(Length::Fill);

                        let duration_tag = text(fonts.primary_font(), next_duration)
                            .size(11)
                            .color(theme_cfg.text_muted);

                        button(
                            row![next_info, duration_tag,]
                                .spacing(8)
                                .align_y(Alignment::Center),
                        )
                        .style(styles::tile(theme_cfg))
                        .padding(0)
                        .on_press(Message::PlayTrack(next_id))
                        .width(Length::Fill)
                        .boxed()
                    }
                    None => text(fonts.primary_font(), "No upcoming tracks")
                        .size(11)
                        .color(theme_cfg.text_muted)
                        .boxed(),
                };

                // Technical Details Section ("Properties", size 11, without icon)
                let tech_info_section: Element<'a, Message> =
                    if !self.cached.tech_properties.is_empty() {
                        let mut prop_col = column![text(fonts.primary_font(), "Properties")
                            .size(11)
                            .color(theme_cfg.primary),]
                        .spacing(4);

                        for (label, val) in &self.cached.tech_properties {
                            prop_col = prop_col.push(
                                row![
                                    text(fonts.primary_font(), *label)
                                        .size(11)
                                        .color(theme_cfg.text_muted)
                                        .width(80),
                                    text(fonts.primary_font(), val.as_str())
                                        .size(11)
                                        .color(theme_cfg.text),
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
                        component::border_vert(1.0, theme_cfg.border),
                        tech_info_section,
                        component::border_vert(1.0, theme_cfg.border),
                        up_next_header,
                        next_track_item,
                    ]
                    .spacing(8)
                    .width(Length::Fill),
                )
                .width(INFO_PANEL_WIDTH)
                .padding(iced::Padding {
                    top: 14.0,
                    right: 14.0,
                    bottom: 18.0,
                    left: 14.0,
                })
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(theme_cfg.surface)),
                    border: iced::Border {
                        radius: theme_cfg.border_radius.md,
                        color: theme_cfg.border,
                        width: 1.0,
                    },
                    text_color: Some(theme_cfg.text),
                    ..Default::default()
                });

                let right_section = column![
                    Space::new().height(Length::Fill),
                    cover_art,
                    Space::new().height(16),
                    bottom_right_card,
                ]
                .width(INFO_PANEL_WIDTH)
                .height(Length::Fill)
                .align_x(Alignment::Center);

                row![lyrics_panel, Space::new().width(20), right_section,]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .boxed()
            }
            None => component::empty_state(
                theme_cfg,
                fonts.primary_font(),
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
    use metadata::AudioTechnicalInfo;
    use model::{Track, TrackRow};
    use theme::ThemeConfig;

    #[test]
    fn test_now_playing_scroll_state() {
        let mut view = NowPlayingView::default();
        assert_eq!(view.scroll_offset.get(), 0.0);
        view.scroll_offset.set(175.0);
        assert_eq!(view.scroll_offset.get(), 175.0);

        // Updating track to a new track resets scroll
        let track = Track {
            id: 2,
            file_path: "/music/track2.flac".to_string(),
            title: "Track 2".to_string(),
            artist: "Artist".to_string(),
            album_artist: None,
            album: "Album".to_string(),
            genre: None,
            duration_ms: 100_000,
            track_num: None,
            disc_num: None,
            year: None,
            composer: None,
            label: None,
            musicbrainz_track_id: None,
            musicbrainz_artist_id: None,
            musicbrainz_album_id: None,
            added_timestamp: 1700000000,
        };
        view.update_track(Some(&track));
        assert_eq!(view.scroll_offset.get(), 0.0);

        view.scroll_offset.set(90.0);
        // Updating track to the SAME track does not reset scroll
        view.update_track(Some(&track));
        assert_eq!(view.scroll_offset.get(), 90.0);

        view.clear();
        assert_eq!(view.scroll_offset.get(), 0.0);
    }

    #[test]
    fn test_now_playing_empty_state_renders() {
        let theme_cfg = ThemeConfig::default();
        let now_playing_view = NowPlayingView::default();

        let _element = now_playing_view.view(
            &theme_cfg,
            &font::FontConfig::default(),
            None,
            None,
            None,
            None,
            model::RepeatMode::Off,
            false,
            0,
            None,
        );
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
        let mut now_playing_view = NowPlayingView::default();
        now_playing_view.set_lyrics(Some(lyrics_text.to_string()));
        now_playing_view.update_track(Some(&track));
        now_playing_view.update_next_track(Some(&next_track));
        now_playing_view.update_tech_info(Some(&tech_info));

        let _element = now_playing_view.view(
            &theme_cfg,
            &font::FontConfig::default(),
            Some(&track),
            None,
            None,
            Some(&next_track),
            model::RepeatMode::One,
            true,
            5_000,
            Some(&tech_info),
        );
    }
}
