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

use core_engine::{AlbumSummary, ArtistSummary};
use message::{AlbumSortField, ArtistSortField, Message, ViewMode};
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

fn sample_albums(count: usize) -> Vec<AlbumSummary> {
    (0..count)
        .map(|i| AlbumSummary {
            album: format!("Album {}", i).into(),
            artist: format!("Artist {}", i % 500).into(),
            track_count: 10,
            total_duration_ms: 180_000,
            year: Some(2020),
            cover_track_id: Some(i as i64 + 1),
            added_timestamp: 1_700_000_000 + i as i64,
            year_label: "2020".to_string(),
            tracks_label: "10 tracks".to_string(),
            duration_label: "3:00".to_string(),
        })
        .collect()
}

#[test]
fn test_fast_scroll_abandons_stale_cover_loads() {
    let mut view = albums::AlbumsView::new(ViewMode::List, AlbumSortField::Title);
    view.set_albums(sample_albums(5_000));

    // Begin loading covers for the initial window.
    let initial = view.get_missing_covers_to_load();
    assert!(!initial.is_empty());
    view.mark_loading(initial.iter().map(|(k, _)| k.clone()));
    assert!(!view.in_flight.is_empty());

    // Jump to the bottom: in-flight loads for the abandoned window must be dropped.
    let bottom = 5_000.0 * view.row_height();
    view.on_scroll_metrics(bottom, 1_100.0, 720.0);

    let visible_keys: std::collections::HashSet<String> = view
        .visible_albums()
        .iter()
        .map(|a| format!("{}\u{1f}{}", a.artist, a.album))
        .collect();
    assert!(
        view.in_flight.keys().all(|k| visible_keys.contains(k)),
        "stale in-flight cover loads were not abandoned"
    );

    // The landing window has covers to (re)load.
    assert!(!view.get_missing_covers_to_load().is_empty());

    // Scrolling back to the top must also report missing covers to reload.
    view.on_scroll_metrics(0.0, 1_100.0, 720.0);
    assert!(!view.get_missing_covers_to_load().is_empty());
}

#[test]
fn in_flight_requests_expire_so_they_can_retry() {
    let mut view = albums::AlbumsView::new(ViewMode::List, AlbumSortField::Title);
    view.set_albums(sample_albums(10));

    let to_load = view.get_missing_covers_to_load();
    view.mark_loading(to_load.iter().map(|(k, _)| k.clone()));
    assert!(!view.in_flight.is_empty());

    // A generous age leaves fresh requests alone.
    assert!(!view.expire_stale_in_flight(std::time::Duration::from_secs(60)));
    assert!(!view.in_flight.is_empty());

    // An expired age releases them so they can be requested again.
    assert!(view.expire_stale_in_flight(std::time::Duration::ZERO));
    assert!(view.in_flight.is_empty());
    assert!(!view.get_missing_covers_to_load().is_empty());
}

#[test]
fn stale_failure_does_not_poison_a_new_request() {
    let mut view = albums::AlbumsView::new(ViewMode::List, AlbumSortField::Title);
    view.set_albums(sample_albums(100));

    // A first request owns the visible covers.
    let to_load = view.get_missing_covers_to_load();
    let stale_id = view.mark_loading(to_load.iter().map(|(k, _)| k.clone()));
    let key = to_load[0].0.clone();

    // The album is re-requested under a newer id while still visible.
    let fresh_id = view.mark_loading(vec![key.clone()]);
    assert_ne!(stale_id, fresh_id);

    // The abandoned request fails for that key: it must not poison the fresh one.
    view.insert_covers(stale_id, vec![(key.clone(), Vec::new())]);
    assert!(
        !view.failed.contains(&key),
        "stale failure permanently poisoned the key"
    );
    assert!(view.is_in_flight(&key), "fresh request was wrongly cleared");

    // The fresh request succeeds and caches the cover.
    view.insert_covers(fresh_id, vec![(key.clone(), vec![1u8, 2, 3])]);
    assert!(view.album_covers.contains_key(&key));
    assert!(!view.failed.contains(&key));
}

#[test]
fn sim_fast_scroll_roundtrip_converges() {
    // Mirrors the app's cover flow: on a window change and after every delivered
    // batch, request all still-missing visible covers. Deliveries are delayed by
    // a couple of scroll steps to model the async pipeline.
    let mut view = albums::AlbumsView::new(ViewMode::Tile, AlbumSortField::Title);
    view.set_albums(sample_albums(759));
    let row_h = view.row_height();
    let bottom = view.len() as f32 * row_h;

    let mut positions: Vec<f32> = Vec::new();
    let mut y = 0.0_f32;
    while y <= bottom {
        positions.push(y);
        y += 1_500.0;
    }
    positions.push(bottom);
    y = bottom;
    while y >= 0.0 {
        positions.push(y);
        y -= 1_500.0;
    }
    positions.push(0.0);

    type CoverBatch = Vec<(String, Vec<u8>)>;
    type PendingBatch = (usize, u64, CoverBatch);
    type DueBatch = (u64, CoverBatch);

    // (deliver_at_step, request_id, results)
    let mut pending: Vec<PendingBatch> = Vec::new();

    fn schedule(view: &mut albums::AlbumsView, step: usize, pending: &mut Vec<PendingBatch>) {
        let to_load = view.get_missing_covers_to_load();
        if to_load.is_empty() {
            return;
        }
        let request_id = view.mark_loading(to_load.iter().map(|(k, _)| k.clone()));
        let results: CoverBatch = to_load
            .into_iter()
            .map(|(k, _)| (k, vec![1u8, 2, 3]))
            .collect();
        pending.push((step + 2, request_id, results));
    }

    for (step, &pos) in positions.iter().enumerate() {
        let mut due: Vec<DueBatch> = Vec::new();
        let mut still = Vec::new();
        for (at, id, res) in pending.drain(..) {
            if at <= step {
                due.push((id, res));
            } else {
                still.push((at, id, res));
            }
        }
        pending = still;
        for (id, res) in due {
            view.insert_covers(id, res);
            schedule(&mut view, step, &mut pending);
        }

        if view.on_scroll_metrics(pos, 1_100.0, 580.0) {
            schedule(&mut view, step, &mut pending);
        }
    }

    // Settle remaining async work.
    for _ in 0..300 {
        if pending.is_empty() {
            schedule(&mut view, 0, &mut pending);
            if pending.is_empty() {
                break;
            }
        }
        let next = pending.iter().map(|(a, _, _)| *a).min().unwrap();
        let mut due: Vec<DueBatch> = Vec::new();
        let mut still = Vec::new();
        for (at, id, res) in pending.drain(..) {
            if at <= next {
                due.push((id, res));
            } else {
                still.push((at, id, res));
            }
        }
        pending = still;
        for (id, res) in due {
            view.insert_covers(id, res);
        }
        schedule(&mut view, next, &mut pending);
    }

    let missing = view.get_missing_covers_to_load();
    assert!(
        missing.is_empty(),
        "after roundtrip, {} visible covers are still missing",
        missing.len()
    );
}

#[test]
fn test_albums_are_virtualized_not_paginated() {
    // The whole catalog is addressable (no "load 60 more"), but only a bounded
    // window of albums is ever materialized.
    for view_mode in [ViewMode::Tile, ViewMode::List] {
        let mut view = albums::AlbumsView::new(view_mode, AlbumSortField::Title);
        view.set_albums(sample_albums(5_000));

        assert_eq!(view.len(), 5_000);
        let initial = view.window_len();
        assert!(initial > 0 && initial < 5_000, "initial window = {initial}");

        let row_h = view.row_height();
        let bottom = 5_000.0 * row_h;

        let mut y = 0.0_f32;
        let mut peak = initial;
        while y <= bottom {
            view.on_scroll_metrics(y, 1_100.0, 720.0);
            peak = peak.max(view.window_len());
            y += row_h;
        }

        assert!(peak < 5_000, "window grew to {peak} (mode {view_mode:?})");
        // The very end of the catalog is reachable.
        view.on_scroll_metrics(bottom, 1_100.0, 720.0);
        assert_eq!(view.window_end, 5_000, "mode {view_mode:?}");
        // And so is the very start when scrolling back up.
        view.on_scroll_metrics(0.0, 1_100.0, 720.0);
        assert_eq!(view.window_start, 0, "mode {view_mode:?}");
    }
}

#[test]
fn test_album_and_artist_views_render_sliced() {
    let theme_cfg = ThemeConfig::default();
    let albums_list: Vec<AlbumSummary> = (0..120)
        .map(|i| AlbumSummary {
            album: format!("Album {}", i).into(),
            artist: format!("Artist {}", i).into(),
            track_count: 10,
            total_duration_ms: 180_000,
            year: Some(2020),
            cover_track_id: None,
            added_timestamp: 1700000000 + i as i64,
            year_label: "2020".to_string(),
            tracks_label: "10 tracks".to_string(),
            duration_label: "3:00".to_string(),
        })
        .collect();

    let artists_list: Vec<ArtistSummary> = (0..120)
        .map(|i| ArtistSummary {
            artist: format!("Artist {}", i).into(),
            track_count: 5,
            album_count: 1,
            stats_label: "1 albums • 5 tracks".to_string(),
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
            album: format!("Album {}", i).into(),
            artist: format!("Artist {}", i).into(),
            track_count: 5,
            total_duration_ms: 60_000,
            year: Some(2021),
            cover_track_id: None,
            added_timestamp: 1700000000 + i as i64,
            year_label: "2021".to_string(),
            tracks_label: "5 tracks".to_string(),
            duration_label: "1:00".to_string(),
        })
        .collect();

    albums_view.set_albums(albums_list);
    assert!(!albums_view.is_empty());
    assert_eq!(albums_view.len(), 50);

    albums_view.toggle_sort(AlbumSortField::Title);
    albums_view.set_view_mode(ViewMode::List);
    assert_eq!(albums_view.tile_width, 140);
    albums_view.set_tile_width(220);
    assert_eq!(albums_view.tile_width, 220);

    // Verify bounded LRU cover insertion and eviction
    let dummy_bytes = vec![0u8; 10];
    let covers_to_insert: Vec<(String, Vec<u8>)> = (0..150)
        .map(|i| (format!("Artist:Album_{}", i), dummy_bytes.clone()))
        .collect();
    let id = albums_view.mark_loading(covers_to_insert.iter().map(|(k, _)| k.clone()));
    albums_view.insert_covers(id, covers_to_insert);
    assert_eq!(albums_view.album_covers.len(), albums::MAX_ALBUM_COVERS);
    assert_eq!(albums_view.cover_order.len(), albums::MAX_ALBUM_COVERS);
    // Oldest item (0) should have been evicted; newest item (149) should be present
    assert!(!albums_view.album_covers.contains_key("Artist:Album_0"));
    assert!(albums_view.album_covers.contains_key("Artist:Album_149"));

    // Verify LRU recency promotion: re-insert item 30 (which makes it newest), then insert 60 new items.
    // Since MAX_ALBUM_COVERS is 120, the 60 oldest items (31..91) are evicted, but item 30 is retained.
    let id = albums_view.mark_loading(vec!["Artist:Album_30".to_string()]);
    albums_view.insert_covers(
        id,
        vec![("Artist:Album_30".to_string(), dummy_bytes.clone())],
    );
    let new_batch: Vec<(String, Vec<u8>)> = (150..210)
        .map(|i| (format!("Artist:Album_{}", i), dummy_bytes.clone()))
        .collect();
    let id = albums_view.mark_loading(new_batch.iter().map(|(k, _)| k.clone()));
    albums_view.insert_covers(id, new_batch);
    assert_eq!(albums_view.album_covers.len(), albums::MAX_ALBUM_COVERS);
    assert_eq!(albums_view.cover_order.len(), albums::MAX_ALBUM_COVERS);
    assert!(albums_view.album_covers.contains_key("Artist:Album_30"));
    assert!(!albums_view.album_covers.contains_key("Artist:Album_31"));

    albums_view.clear_covers();
    assert!(albums_view.album_covers.is_empty());
    assert!(albums_view.cover_order.is_empty());

    let theme_cfg = ThemeConfig::default();
    let _rendered = albums_view.view(&theme_cfg);
}

#[test]
fn test_artists_view_component_state() {
    let mut artists_view = artists::ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
    assert!(artists_view.is_empty());
    assert_eq!(artists_view.len(), 0);
    assert_eq!(artists_view.tile_width, 140);
    artists_view.set_tile_width(200);
    assert_eq!(artists_view.tile_width, 200);

    let artists_list: Vec<ArtistSummary> = (0..30)
        .map(|i| ArtistSummary {
            artist: format!("Artist {}", i).into(),
            track_count: 3,
            album_count: 1,
            stats_label: "1 albums • 3 tracks".to_string(),
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

#[test]
fn test_artists_view_is_virtualized() {
    let mut view = artists::ArtistsView::new(ViewMode::List, ArtistSortField::Artist);
    let all: Vec<ArtistSummary> = (0..10_000)
        .map(|i| ArtistSummary {
            artist: format!("Artist {}", i).into(),
            track_count: 1,
            album_count: 1,
            stats_label: "1 albums • 1 tracks".to_string(),
        })
        .collect();
    view.set_artists(all);

    // Only a small window is materialized regardless of catalog size.
    assert!(view.window_len() < 200, "window_len={}", view.window_len());
    assert_eq!(view.total_tracks, 10_000);
    assert_eq!(view.total_albums, 10_000);

    // Scrolling to the middle relocates a bounded window without growing it.
    view.on_scroll_metrics(100_000.0, 1000.0, 700.0);
    assert!(view.window_len() < 200, "window_len={}", view.window_len());

    let theme_cfg = ThemeConfig::default();
    let _rendered = view.view(&theme_cfg);
}
