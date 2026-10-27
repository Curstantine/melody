use core_engine::{format_duration, Track};
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
    pub total_tracks: usize,
    pub search_query: &'a str,
    // Albums context
    pub album_count: usize,
    pub album_tracks: usize,
    pub album_duration_ms: u64,
    /// Artists context
    pub artist_count: usize,
    pub artist_albums: usize,
    pub artist_tracks: usize,
    // Now Playing context
    pub current_track: Option<&'a Track>,
    pub is_playing: bool,
    // Playlists / Queues context
    pub playlist_count: usize,
    pub queue_count: usize,
    /// Active queue name and length, when a queue drives playback.
    pub active_queue: Option<(&'a str, usize)>,
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
                let count_text = if !ctx.search_query.trim().is_empty() {
                    format!("{} matching tracks", ctx.total_tracks)
                } else {
                    format!("{} tracks in library", ctx.total_tracks)
                };

                let filter_indicator: Element<'a, Message> = if !ctx.search_query.trim().is_empty()
                {
                    text(theme_cfg, format!("·  Filter: {}", ctx.search_query.trim()))
                        .size(11)
                        .color(muted)
                        .boxed()
                } else {
                    Space::new().width(0).boxed()
                };

                row![
                    icons::icon(icons::MUSIC_NOTE_SVG, 11.0, muted),
                    text(theme_cfg, count_text).size(11).color(muted),
                    filter_indicator,
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .boxed()
            }

            ViewTab::Albums => row![
                icons::icon(icons::ALBUM_SVG, 11.0, muted),
                text(theme_cfg, format!("{} albums", ctx.album_count))
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, format!("{} tracks", ctx.album_tracks))
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, format_duration(ctx.album_duration_ms))
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .boxed(),

            ViewTab::Artists => row![
                icons::icon(icons::USER_AVATAR_SVG, 11.0, muted),
                text(theme_cfg, format!("{} artists", ctx.artist_count))
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, format!("{} albums", ctx.artist_albums))
                    .size(11)
                    .color(muted),
                text(theme_cfg, "·").size(11).color(muted),
                text(theme_cfg, format!("{} tracks", ctx.artist_tracks))
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .boxed(),

            ViewTab::Lyrics => {
                if let Some(track) = ctx.current_track {
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
                        text(
                            theme_cfg,
                            format!("Now playing: {} • {}", track.title, track.artist)
                        )
                        .size(11)
                        .color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, &track.album).size(11).color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, track.formatted_duration())
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
                if let Some((name, len)) = ctx.active_queue {
                    row![
                        icons::icon(icons::QUEUE_SVG, 11.0, muted),
                        text(theme_cfg, format!("Queue: {}", name))
                            .size(11)
                            .color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, format!("{} tracks", len))
                            .size(11)
                            .color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, format!("{} playlists", ctx.playlist_count))
                            .size(11)
                            .color(muted),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .boxed()
                } else {
                    row![
                        icons::icon(icons::PLAYLIST_SVG, 11.0, muted),
                        text(theme_cfg, format!("{} playlists", ctx.playlist_count))
                            .size(11)
                            .color(muted),
                        text(theme_cfg, "·").size(11).color(muted),
                        text(theme_cfg, format!("{} queues open", ctx.queue_count))
                            .size(11)
                            .color(muted),
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
                text(theme_cfg, format!("{} tracks indexed", ctx.total_tracks))
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
            total_tracks: 100,
            search_query: "",
            album_count: 10,
            album_tracks: 100,
            album_duration_ms: 100000,
            artist_count: 5,
            artist_albums: 10,
            artist_tracks: 100,
            current_track: None,
            is_playing: false,
            playlist_count: 2,
            queue_count: 1,
            active_queue: None,
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
