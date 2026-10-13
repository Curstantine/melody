#![allow(dead_code)]

#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/views/playlists.rs"]
mod playlists;
#[path = "../src/queue.rs"]
mod queue;
#[path = "../src/theme/mod.rs"]
mod theme;

use std::path::PathBuf;

use core_engine::{parse_m3u, write_m3u, PlaylistEntry};
use message::{PlaylistItem, PlaylistSection, PlaylistSortField, ViewMode};
use playlists::PlaylistsView;

fn item(title: &str, track_id: Option<i64>) -> PlaylistItem {
    PlaylistItem {
        title: title.to_string(),
        artist: "Artist".to_string(),
        duration_label: "3:00".to_string(),
        track_id,
    }
}

fn entry(raw: &str) -> PlaylistEntry {
    PlaylistEntry {
        raw_path: raw.to_string(),
        resolved_path: PathBuf::from(raw),
        title: None,
        duration_ms: Some(180_000),
    }
}

fn detail_view() -> PlaylistsView {
    let mut view = PlaylistsView::new(ViewMode::List, PlaylistSortField::Name);
    view.set_detail(
        7,
        "Mixtape".to_string(),
        "/playlists/mixtape.m3u8".to_string(),
        vec![
            item("One", Some(1)),
            item("Missing", None),
            item("Three", Some(3)),
        ],
        vec![
            entry("/music/1.flac"),
            entry("/music/x.flac"),
            entry("/music/3.flac"),
        ],
    );
    view
}

#[test]
fn move_detail_entry_permutes_items_and_entries_together() {
    let mut view = detail_view();
    assert!(view.move_detail_entry(0, 2));
    assert_eq!(
        view.detail_items
            .iter()
            .map(|i| i.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Missing", "Three", "One"]
    );
    assert_eq!(
        view.detail_entries
            .iter()
            .map(|e| e.raw_path.as_str())
            .collect::<Vec<_>>(),
        vec!["/music/x.flac", "/music/3.flac", "/music/1.flac"],
        "raw file entries (including unmatched lines) follow the same move"
    );
}

#[test]
fn move_detail_entry_rejects_bad_moves() {
    let mut view = detail_view();
    assert!(!view.move_detail_entry(1, 1), "same slot is a no-op");
    assert!(!view.move_detail_entry(0, 3), "out of bounds is a no-op");
    assert!(!view.move_detail_entry(9, 0), "out of bounds is a no-op");
    assert_eq!(view.detail_items[0].title, "One");
}

#[test]
fn detail_click_needs_same_row_twice_quickly() {
    let mut view = detail_view();
    assert!(!view.on_detail_click(7, 0), "first click only selects");
    assert!(
        view.on_detail_click(7, 0),
        "second click on the same row plays"
    );
    assert!(
        !view.on_detail_click(7, 1),
        "a different row starts a new arm"
    );
    assert!(
        !view.on_detail_click(9, 1),
        "a different playlist starts a new arm"
    );
    assert_eq!(view.selected_detail_track, Some((9, 1)));
}

#[test]
fn reordered_entries_round_trip_through_m3u_file() {
    let mut view = detail_view();
    assert!(view.move_detail_entry(2, 0));

    let path = std::env::temp_dir().join(format!(
        "melody-playlist-reorder-{}.m3u8",
        std::process::id()
    ));
    write_m3u(&path, &view.detail_entries).expect("write reordered playlist");
    let parsed = parse_m3u(&path).expect("parse reordered playlist");
    let _ = std::fs::remove_file(&path);
    // write_m3u relativizes paths against the playlist directory (joined
    // lexically, not normalized), so compare file names: order must survive
    // the round trip.
    assert_eq!(
        parsed
            .entries
            .iter()
            .map(|e| e
                .resolved_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string())
            .collect::<Vec<_>>(),
        vec!["3.flac", "1.flac", "x.flac"]
    );
}

#[test]
fn shared_track_row_renders_without_panicking() {
    use components::TrackRow;
    use message::Message;
    use theme::ThemeConfig;

    let theme_cfg = ThemeConfig::default();

    // Open-playlist context: clickable row.
    let _ = TrackRow {
        theme: &theme_cfg,
        index: 0,
        title: "One",
        artist: "Artist",
        duration: "3:00",
        highlighted: true,
        dragging: false,
        show_handle: true,
        on_press: Some(Message::PlaylistDetailClicked {
            playlist_id: 7,
            index: 0,
        }),
        drag_start: Message::PlaylistDetailDragStarted {
            playlist_id: 7,
            index: 0,
        },
        drag_release: Message::PlaylistDetailDragReleased,
        hover_enter: Message::PlaylistDetailHovered(Some((7, 0))),
        hover_exit: Message::PlaylistDetailHovered(None),
        on_remove: Message::PlaylistRemoveTrack {
            playlist_id: 7,
            index: 0,
        },
    }
    .view();

    // Open-playlist context: unmatched (unclickable) entry.
    let _ = TrackRow {
        theme: &theme_cfg,
        index: 1,
        title: "Missing",
        artist: "Artist",
        duration: "3:00",
        highlighted: false,
        dragging: false,
        show_handle: false,
        on_press: None,
        drag_start: Message::PlaylistDetailDragStarted {
            playlist_id: 7,
            index: 1,
        },
        drag_release: Message::PlaylistDetailDragReleased,
        hover_enter: Message::PlaylistDetailHovered(Some((7, 1))),
        hover_exit: Message::PlaylistDetailHovered(None),
        on_remove: Message::PlaylistRemoveTrack {
            playlist_id: 7,
            index: 1,
        },
    }
    .view();

    // Queue context.
    let _ = TrackRow {
        theme: &theme_cfg,
        index: 2,
        title: "Three",
        artist: "Artist",
        duration: "4:12",
        highlighted: false,
        dragging: false,
        show_handle: false,
        on_press: Some(Message::QueueClicked {
            queue_id: 1,
            index: 2,
        }),
        drag_start: Message::QueueDragStarted {
            queue_id: 1,
            index: 2,
        },
        drag_release: Message::QueueDragReleased,
        hover_enter: Message::QueueRowHovered(Some((1, 2))),
        hover_exit: Message::QueueRowHovered(None),
        on_remove: Message::QueueRemoveTrack {
            queue_id: 1,
            index: 2,
        },
    }
    .view();
}

#[test]
fn detail_state_resets_with_detail() {
    let mut view = detail_view();
    view.on_detail_click(7, 0);
    view.hovered_detail_row = Some((7, 0));
    view.clear_detail();
    assert!(view.detail_entries.is_empty());
    assert!(view.selected_detail_track.is_none());
    assert!(view.hovered_detail_row.is_none());
    assert!(view.last_detail_click.is_none());
    assert_eq!(view.section, PlaylistSection::Playlists);
}

#[test]
fn remove_detail_entry_keeps_items_and_entries_in_sync() {
    let mut view = detail_view();
    assert!(view.remove_detail_entry(7, 1));
    assert_eq!(
        view.detail_items
            .iter()
            .map(|i| i.title.as_str())
            .collect::<Vec<_>>(),
        vec!["One", "Three"]
    );
    assert_eq!(
        view.detail_entries
            .iter()
            .map(|e| e.raw_path.as_str())
            .collect::<Vec<_>>(),
        vec!["/music/1.flac", "/music/3.flac"],
        "raw file entries (including unmatched lines) follow the same removal"
    );
}

#[test]
fn remove_detail_entry_rejects_bad_removals() {
    let mut view = detail_view();
    assert!(!view.remove_detail_entry(7, 3), "out of bounds is a no-op");
    assert!(!view.remove_detail_entry(9, 0), "wrong playlist is a no-op");
    assert_eq!(view.detail_items.len(), 3);
    assert_eq!(view.detail_entries.len(), 3);
}

#[test]
fn remove_detail_entry_shifts_selection_state() {
    let mut view = detail_view();
    view.selected_detail_track = Some((7, 2));
    view.hovered_detail_row = Some((7, 2));
    assert!(view.remove_detail_entry(7, 0));
    assert_eq!(view.selected_detail_track, Some((7, 1)));
    assert_eq!(view.hovered_detail_row, Some((7, 1)));

    assert!(view.remove_detail_entry(7, 1));
    assert!(view.selected_detail_track.is_none());
    assert!(view.hovered_detail_row.is_none());
}
