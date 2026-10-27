use iced::widget::{column, container, row, Space, Widget};
use iced::{Alignment, Element, Length};

use crate::components::{border_vert, scanning_indicator, text, thumbnail_caching_indicator};
use crate::icons;
use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeConfig};

#[derive(Debug, Clone, Copy)]
pub struct StatusBarContext<'a> {
    pub active_tab: ViewTab,
    // Tracks (Music) context
    pub tracks_count_label: &'a str,
    pub filter_indicator_label: &'a str,
    // Albums context
    pub album_count_label: &'a str,
    pub album_tracks_label: &'a str,
    pub album_duration_label: &'a str,
    /// Artists context
    pub artist_count_label: &'a str,
    pub artist_albums_label: &'a str,
    pub artist_tracks_label: &'a str,
    // Now Playing context
    pub now_playing_label: Option<&'a str>,
    pub now_playing_album: Option<&'a str>,
    pub now_playing_duration: Option<&'a str>,
    pub is_playing: bool,
    // Playlists / Queues context
    pub queue_name_label: Option<&'a str>,
    pub queue_tracks_label: Option<&'a str>,
    pub playlist_count_label: &'a str,
    pub queue_count_label: &'a str,
    // Settings context
    pub settings_indexed_label: &'a str,
    // Global telemetry
    pub is_scanning: bool,
    pub scan_progress: Option<(usize, usize)>,
    pub is_generating_thumbnails: bool,
}

/// Persistent bottom status bar component.
pub struct StatusBar;

impl StatusBar {
    pub fn view<'a>(theme_cfg: &'a ThemeConfig, ctx: StatusBarContext<'a>) -> Element<'a, Message> {
        let muted = theme_cfg.text_muted_color();

        let left_items: Element<'a, Message> = match ctx.active_tab {
            ViewTab::Tracks => {
                let filter_indicator: Element<'a, Message> =
                    if !ctx.filter_indicator_label.is_empty() {
                        text(theme_cfg, ctx.filter_indicator_label)
                            .size(11)
                            .color(muted)
                            .boxed()
                    } else {
                        Space::new().width(0).boxed()
                    };

                row![
                    icons::icon(icons::MUSIC_NOTE_SVG, 11.0, muted),
                    text(theme_cfg, ctx.tracks_count_label)
                        .size(11)
                        .color(muted),
                    filter_indicator,
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .boxed()
            }

            ViewTab::Albums => row![
                icons::icon(icons::ALBUM_SVG, 11.0, muted),
                text(theme_cfg, ctx.album_count_label).size(11).color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, ctx.album_tracks_label)
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, ctx.album_duration_label)
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .boxed(),

            ViewTab::Artists => row![
                icons::icon(icons::USER_AVATAR_SVG, 11.0, muted),
                text(theme_cfg, ctx.artist_count_label)
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, ctx.artist_albums_label)
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, ctx.artist_tracks_label)
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .boxed(),

            ViewTab::Lyrics => {
                if let Some(now_playing) = ctx.now_playing_label {
                    row![
                        icons::icon(
                            if ctx.is_playing {
                                icons::SPEAKER_PLAYING_SVG
                            } else {
                                icons::DISC_SVG
                            },
                            11.0,
                            muted,
                        ),
                        text(theme_cfg, now_playing).size(11).color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, ctx.now_playing_album.unwrap_or(""))
                            .size(11)
                            .color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, ctx.now_playing_duration.unwrap_or(""))
                            .size(11)
                            .color(muted),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .boxed()
                } else {
                    row![
                        icons::icon(icons::DISC_SVG, 11.0, muted),
                        text(theme_cfg, "Playback idle").size(11).color(muted),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .boxed()
                }
            }

            ViewTab::Playlists => {
                if let (Some(queue_name), Some(queue_len)) =
                    (ctx.queue_name_label, ctx.queue_tracks_label)
                {
                    row![
                        icons::icon(icons::QUEUE_SVG, 11.0, muted),
                        text(theme_cfg, queue_name).size(11).color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, queue_len).size(11).color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, ctx.playlist_count_label)
                            .size(11)
                            .color(muted),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .boxed()
                } else {
                    row![
                        icons::icon(icons::PLAYLIST_SVG, 11.0, muted),
                        text(theme_cfg, ctx.playlist_count_label)
                            .size(11)
                            .color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, ctx.queue_count_label).size(11).color(muted),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .boxed()
                }
            }

            ViewTab::Settings => row![
                icons::icon(icons::SETTINGS_SVG, 11.0, muted),
                text(theme_cfg, "Preferences").size(11).color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, ctx.settings_indexed_label)
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .boxed(),
        };

        // 2. Global Right Section
        let mut status_row = row![left_items, Space::new().width(Length::Fill),];
        if ctx.is_scanning {
            status_row = status_row.push(scanning_indicator(theme_cfg, ctx.scan_progress));
        } else if ctx.is_generating_thumbnails {
            status_row = status_row.push(thumbnail_caching_indicator(theme_cfg));
        }
        let status_row = status_row
            .spacing(8)
            .height(24)
            .align_y(Alignment::Center)
            .padding([0, 10]);

        container(column![
            border_vert(1.0, theme_cfg.border_color()),
            status_row
        ])
        .style(styles::status_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(25)
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message;
    use crate::theme::ThemeConfig;

    #[test]
    fn test_status_bar_indicators_render() {
        let theme_cfg = ThemeConfig::default();
        let ctx = StatusBarContext {
            active_tab: message::ViewTab::Tracks,
            tracks_count_label: "100 tracks in library",
            filter_indicator_label: "",
            album_count_label: "10 albums",
            album_tracks_label: "100 tracks",
            album_duration_label: "45:00",
            artist_count_label: "5 artists",
            artist_albums_label: "10 albums",
            artist_tracks_label: "100 tracks",
            now_playing_label: None,
            now_playing_album: None,
            now_playing_duration: None,
            is_playing: false,
            queue_name_label: None,
            queue_tracks_label: None,
            playlist_count_label: "2 playlists",
            queue_count_label: "1 queues open",
            settings_indexed_label: "100 tracks indexed",
            is_scanning: true,
            scan_progress: Some((45, 120)),
            is_generating_thumbnails: false,
        };
        let _el = StatusBar::view(&theme_cfg, ctx);

        let ctx_thumbs = StatusBarContext {
            is_scanning: false,
            scan_progress: None,
            is_generating_thumbnails: true,
            ..ctx
        };
        let _el2 = StatusBar::view(&theme_cfg, ctx_thumbs);
    }
}
