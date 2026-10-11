#![allow(dead_code)]

#[path = "../src/views/albums.rs"]
mod albums;
#[path = "../src/views/artists.rs"]
mod artists;
#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/theme/mod.rs"]
mod theme;

use message::{AlbumSortField, Message, ViewMode};
use player_core::{AlbumSummary, ArtistSummary, SortDirection};
use std::collections::HashMap;
use theme::ThemeConfig;

#[test]
fn test_adaptive_cjk_text_creation() {
    let _ascii = components::cjk_text("The Beatles - Abbey Road");
    let _japanese = components::cjk_text("宇多田ヒカル - First Love");
    let _chinese = components::cjk_text("周杰伦 - 范特西");
    let _korean = components::cjk_text("아이유 - Palette");
}

#[test]
fn test_album_and_artist_views_render_sliced() {
    let theme_cfg = ThemeConfig::default();
    let albums_list: Vec<AlbumSummary> = (0..120)
        .map(|i| AlbumSummary {
            album: format!("Album {}", i),
            artist: format!("Artist {}", i),
            track_count: 10,
            total_duration_ms: 180_000,
            year: Some(2020),
            cover_track_path: None,
        })
        .collect();

    let artists_list: Vec<ArtistSummary> = (0..120)
        .map(|i| ArtistSummary {
            artist: format!("Artist {}", i),
            track_count: 5,
            album_count: 1,
        })
        .collect();

    let album_covers = HashMap::new();

    // Sliced view for albums (first 60 items)
    let visible_albums = &albums_list[..60];
    let _album_tile = albums::view(
        &theme_cfg,
        visible_albums,
        &album_covers,
        ViewMode::Tile,
        AlbumSortField::Title,
        SortDirection::Ascending,
    );
    let _album_list = albums::view(
        &theme_cfg,
        visible_albums,
        &album_covers,
        ViewMode::List,
        AlbumSortField::Title,
        SortDirection::Ascending,
    );

    // Sliced view for artists (first 60 items)
    let visible_artists = &artists_list[..60];
    let _artist_tile = artists::view(&theme_cfg, visible_artists, ViewMode::Tile);
    let _artist_list = artists::view(&theme_cfg, visible_artists, ViewMode::List);
}

#[test]
fn test_scroll_messages_variants() {
    fn check_message(_m: Message) {}
    check_message(Message::NextTrack);
}
