use iced::widget::{container, row, text, Space};
use iced::{Alignment, Element, Length};
use player_core::{format_duration, AlbumSummary, ArtistSummary, Track};

use crate::icons;
use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeConfig};

#[allow(dead_code)]
pub struct StatusBarContext<'a> {
    pub active_tab: ViewTab,
    // Tracks (Music) context
    pub page: usize,
    pub page_size: usize,
    pub total_tracks: usize,
    pub visible_count: usize,
    pub search_query: &'a str,
    // Albums context
    pub albums: &'a [AlbumSummary],
    // Artists context
    pub artists: &'a [ArtistSummary],
    // Now Playing context
    pub current_track: Option<&'a Track>,
    pub is_playing: bool,
    // Global telemetry
    pub is_scanning: bool,
}

pub fn view<'a>(theme_cfg: &'a ThemeConfig, ctx: StatusBarContext<'a>) -> Element<'a, Message> {
    let muted = theme_cfg.text_muted_color();

    let left_items: Element<'a, Message> = match ctx.active_tab {
        ViewTab::Tracks => {
            let count_text = if !ctx.search_query.trim().is_empty() {
                format!(
                    "Showing {} of {} tracks",
                    ctx.visible_count, ctx.total_tracks
                )
            } else if ctx.visible_count >= ctx.total_tracks {
                format!("{} tracks in library", ctx.total_tracks)
            } else {
                format!(
                    "Loaded {} of {} tracks",
                    ctx.visible_count, ctx.total_tracks
                )
            };

            let filter_indicator: Element<'a, Message> = if !ctx.search_query.trim().is_empty() {
                text(format!("·  Filter: \"{}\"", ctx.search_query.trim()))
                    .size(11)
                    .color(muted)
                    .into()
            } else {
                Space::with_width(0).into()
            };

            row![
                icons::icon(icons::MUSIC_NOTE_SVG, 11.0, muted),
                text(count_text).size(11).color(muted),
                filter_indicator,
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        }

        ViewTab::Albums => {
            let total_albums = ctx.albums.len();
            let total_tracks: usize = ctx.albums.iter().map(|a| a.track_count).sum();
            let total_duration_ms: u64 = ctx.albums.iter().map(|a| a.total_duration_ms).sum();

            row![
                icons::icon(icons::ALBUM_SVG, 11.0, muted),
                text(format!("{} albums viewing", total_albums))
                    .size(11)
                    .color(muted),
                text("·").size(11).color(muted),
                text(format!("{} tracks", total_tracks))
                    .size(11)
                    .color(muted),
                text("·").size(11).color(muted),
                text(format_duration(total_duration_ms))
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        }

        ViewTab::Artists => {
            let total_artists = ctx.artists.len();
            let total_albums: usize = ctx.artists.iter().map(|a| a.album_count).sum();
            let total_tracks: usize = ctx.artists.iter().map(|a| a.track_count).sum();

            row![
                icons::icon(icons::USER_AVATAR_SVG, 11.0, muted),
                text(format!("{} artists viewing", total_artists))
                    .size(11)
                    .color(muted),
                text("·").size(11).color(muted),
                text(format!("{} albums", total_albums))
                    .size(11)
                    .color(muted),
                text("·").size(11).color(muted),
                text(format!("{} tracks", total_tracks))
                    .size(11)
                    .color(muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        }

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
                    text(format!("Now Playing: {} • {}", track.title, track.artist))
                        .size(11)
                        .color(muted),
                    text("·").size(11).color(muted),
                    text(&track.album).size(11).color(muted),
                    text("·").size(11).color(muted),
                    text(track.formatted_duration()).size(11).color(muted),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            } else {
                row![
                    icons::icon(icons::DISC_SVG, 11.0, muted),
                    text("Playback Idle").size(11).color(muted),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            }
        }

        ViewTab::Settings => row![
            icons::icon(icons::SETTINGS_SVG, 11.0, muted),
            text("Preferences").size(11).color(muted),
            text("·").size(11).color(muted),
            text(format!("{} tracks indexed", ctx.total_tracks))
                .size(11)
                .color(muted),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into(),
    };

    // 2. Global Right Section
    let right_items: Element<'a, Message> = if ctx.is_scanning {
        row![
            icons::icon(icons::REFRESH_SVG, 11.0, muted),
            Space::with_width(3),
            text("Scanning library...").size(11).color(muted),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
    } else {
        Space::with_width(0).into()
    };

    let status_row = row![left_items, Space::with_width(Length::Fill), right_items,]
        .spacing(8)
        .align_y(Alignment::Center)
        .height(Length::Fill);

    container(status_row)
        .style(styles::status_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(24)
        .center_y(24)
        .padding([0, 10])
        .into()
}
