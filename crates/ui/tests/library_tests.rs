#![allow(dead_code)]

#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/views/library.rs"]
mod library;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/theme/mod.rs"]
mod theme;

use core_engine::TrackRow;
use library::{LibraryView, ROW_HEIGHT};
use theme::ThemeConfig;

/// Simulated catalog size for the virtualization tests.
const LARGE_LIBRARY_TRACK_COUNT: usize = 100_000;

fn make_rows(start: usize, count: usize) -> Vec<TrackRow> {
    (start..start + count)
        .map(|i| TrackRow {
            id: i as i64 + 1,
            title: format!("Track {:06}", i),
            artist: format!("Artist {:04}", i % 5_000).into(),
            album: format!("Album {:05}", i % 12_500).into(),
            duration_ms: 180_000,
            file_path: format!("/music/track_{:06}.flac", i),
            duration_label: "3:00".to_string(),
        })
        .collect()
}

#[test]
fn window_capacity_is_bounded_by_page_size() {
    let view = LibraryView::new(50);
    assert_eq!(
        view.window_capacity(),
        50 * (1 + 2 * library::WINDOW_BUFFER_PAGES)
    );
}

#[test]
fn scrolling_a_large_library_keeps_memory_bounded() {
    let total = LARGE_LIBRARY_TRACK_COUNT;

    let mut view = LibraryView::new(50);
    let capacity = view.window_capacity();

    // Initial load at offset 0.
    let initial = make_rows(0, capacity);
    view.set_window(initial, total);
    assert_eq!(view.total_tracks, total);
    assert!(view.rows.len() <= capacity);

    // Simulate a user scrolling all the way to the bottom. Every time the view
    // requests a window reload we emulate the DB returning exactly one window.
    let mut y = 0.0_f32;
    let bottom = total as f32 * ROW_HEIGHT;
    let mut loads = 0usize;
    let mut peak_loaded = view.rows.len();

    while y < bottom {
        if let Some(offset) = view.on_scroll_offset(y) {
            let count = capacity.min(total - offset);
            view.replace_window(offset, make_rows(offset, count));
            loads += 1;
            peak_loaded = peak_loaded.max(view.rows.len());
            assert!(
                view.rows.len() <= capacity,
                "loaded {} rows, capacity is {capacity}",
                view.rows.len()
            );
        }
        y += ROW_HEIGHT * 5.0;
    }

    assert!(
        loads > 1,
        "expected multiple window reloads while scrolling"
    );
    assert!(
        peak_loaded <= capacity,
        "resident rows grew beyond the window capacity"
    );
}

#[test]
fn no_reload_within_buffer_region() {
    let mut view = LibraryView::new(50);
    let capacity = view.window_capacity();
    view.set_window(make_rows(0, capacity), 100_000);

    // The viewport sits comfortably inside the buffered window.
    assert_eq!(view.on_scroll_offset(0.0), None);
    assert_eq!(view.on_scroll_offset(ROW_HEIGHT * 40.0), None);
}

#[test]
fn empty_library_never_requests_a_window() {
    let mut view = LibraryView::new(50);
    assert_eq!(view.on_scroll_offset(0.0), None);
    assert_eq!(view.on_scroll_offset(10_000.0), None);
}

#[test]
fn navigation_and_lookup_use_the_window() {
    let mut view = LibraryView::new(50);
    view.set_window(make_rows(0, 120), 100_000);

    assert_eq!(view.find_row(1).map(|r| r.id), Some(1));
    assert_eq!(view.get_next_row(Some(1), false, 0).map(|r| r.id), Some(2));
    assert_eq!(view.get_previous_row(2).map(|r| r.id), Some(1));
    assert_eq!(view.get_previous_row(1).map(|r| r.id), None);
}

#[test]
fn renders_virtualized_window_with_spacers() {
    let theme_cfg = ThemeConfig::default();
    let mut view = LibraryView::new(50);
    view.set_window(make_rows(0, view.window_capacity()), 100_000);
    // Shifting the window deep into the library must still render.
    view.replace_window(50_000, make_rows(50_000, view.window_capacity()));

    let _element = view.view(&theme_cfg, None, false);
}
