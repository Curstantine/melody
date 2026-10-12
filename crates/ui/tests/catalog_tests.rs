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

use message::{AlbumSortField, ArtistSortField, Message, ViewMode};
use core_engine::{AlbumSummary, ArtistSummary};
use theme::ThemeConfig;

use crate::components::text;

#[test]
fn test_adaptive_cjk_text_creation() {
    let theme_cfg = ThemeConfig::default();
    let _ascii = text(&theme_cfg, "The Beatles - Abbey Road");
    let _japanese = text(&theme_cfg, "宇多田ヒカル - First Love");
    let _chinese = text(&theme_cfg, "周杰伦 - 范特西");
    let _korean = text(&theme_cfg, "아이유 - Palette");
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
            added_timestamp: 1700000000 + i as i64,
        })
        .collect();

    let artists_list: Vec<ArtistSummary> = (0..120)
        .map(|i| ArtistSummary {
            artist: format!("Artist {}", i),
            track_count: 5,
            album_count: 1,
        })
        .collect();

    // Sliced view for albums (first 60 items)
    let mut albums_view_tile = albums::AlbumsView::new(ViewMode::Tile, AlbumSortField::Title);
    albums_view_tile.set_albums(albums_list.clone());
    let _album_tile = albums_view_tile.view(&theme_cfg);

    let mut albums_view_list = albums::AlbumsView::new(ViewMode::List, AlbumSortField::Title);
    albums_view_list.set_albums(albums_list);
    let _album_list = albums_view_list.view(&theme_cfg);

    // Sliced view for artists (first 60 items)
    let mut artists_view_tile = artists::ArtistsView::new(ViewMode::Tile, ArtistSortField::Artist);
    artists_view_tile.set_artists(artists_list.clone());
    let _artist_tile = artists_view_tile.view(&theme_cfg);

    let mut artists_view_list = artists::ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
    artists_view_list.set_artists(artists_list);
    let _artist_list = artists_view_list.view(&theme_cfg);
}

#[test]
fn test_scroll_messages_variants() {
    fn check_message(_m: Message) {}
    check_message(Message::NextTrack);
}

#[test]
fn test_albums_view_component_state() {
    let mut albums_view = albums::AlbumsView::new(ViewMode::Tile, AlbumSortField::Title);
    assert!(albums_view.is_empty());
    assert_eq!(albums_view.len(), 0);

    let albums_list: Vec<AlbumSummary> = (0..50)
        .map(|i| AlbumSummary {
            album: format!("Album {}", i),
            artist: format!("Artist {}", i),
            track_count: 5,
            total_duration_ms: 60_000,
            year: Some(2021),
            cover_track_path: None,
            added_timestamp: 1700000000 + i as i64,
        })
        .collect();

    albums_view.set_albums(albums_list);
    assert!(!albums_view.is_empty());
    assert_eq!(albums_view.len(), 50);

    albums_view.toggle_sort(AlbumSortField::Title);
    albums_view.set_view_mode(ViewMode::List);

    let theme_cfg = ThemeConfig::default();
    let _rendered = albums_view.view(&theme_cfg);
}

#[test]
fn test_artists_view_component_state() {
    let mut artists_view = artists::ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
    assert!(artists_view.is_empty());
    assert_eq!(artists_view.len(), 0);

    let artists_list: Vec<ArtistSummary> = (0..30)
        .map(|i| ArtistSummary {
            artist: format!("Artist {}", i),
            track_count: 3,
            album_count: 1,
        })
        .collect();

    artists_view.set_artists(artists_list);
    assert!(!artists_view.is_empty());
    assert_eq!(artists_view.len(), 30);

    artists_view.toggle_sort(ArtistSortField::Artist);
    artists_view.set_view_mode(ViewMode::Tile);

    let theme_cfg = ThemeConfig::default();
    let _rendered = artists_view.view(&theme_cfg);
}

