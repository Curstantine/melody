#![allow(dead_code)]

use std::sync::Arc;

#[path = "../src/queue.rs"]
mod queue;

use core_engine::{QueueRecord, TrackRow};
use queue::{
    queue_window_capacity, queue_window_for_start, queue_window_start, Queue, QueueSource,
    QueueStore, DEFAULT_QUEUE_NAME, QUEUE_WINDOW_BUFFER_ROWS, QUEUE_WINDOW_ROWS,
};

fn track(id: i64) -> TrackRow {
    TrackRow {
        id,
        title: format!("Track {}", id),
        artist: Arc::from("Artist"),
        album: Arc::from("Album"),
        duration_ms: 1000,
        file_path: format!("/music/{}.flac", id),
        duration_label: "0:01".to_string(),
    }
}

fn record(id: i64, name: &str, count: usize) -> QueueRecord {
    QueueRecord {
        id,
        name: name.to_string(),
        current: 0,
        source_kind: "manual".to_string(),
        source_value: None,
        source_playlist: None,
        track_count: count,
    }
}

#[test]
fn queue_navigation_and_shuffle_boundaries() {
    let mut q = Queue::new(1, "Test");
    assert!(q.is_empty());
    assert_eq!(q.current_index(), None);
    assert_eq!(q.next_index(), None);

    q.append_many((1..=3).map(track));
    assert_eq!(q.len(), 3);
    assert_eq!(q.current_index(), Some(0));
    assert_eq!(q.next_index(), Some(1));

    q.set_current(2);
    assert_eq!(q.next_index(), None, "at the end there is no next track");
    assert_eq!(q.previous_index(), Some(1));

    // insert_next places right after the cursor
    q.set_current(0);
    q.insert_next(track(99));
    assert_eq!(q.tracks[1].id, 99);

    // insert_many_next places multiple tracks in order right after the cursor
    q.insert_many_next(vec![track(100), track(101)]);
    assert_eq!(q.tracks[1].id, 100);
    assert_eq!(q.tracks[2].id, 101);
    assert_eq!(q.tracks[3].id, 99);

    // removing the current track clamps the cursor back onto a valid slot
    q.set_current(1);
    q.remove(1);
    assert!(q.current_index().unwrap() < q.len());

    // move_track_to by one slot follows the moved track
    q.set_current(0);
    let before = q.tracks[0].id;
    assert!(q.move_track_to(0, 1));
    assert_eq!(q.tracks[1].id, before);
    assert_eq!(q.current_index(), Some(1));
    assert!(
        !q.move_track_to(q.len() - 1, q.len()),
        "cannot move past the end"
    );
}

#[test]
fn queue_move_track_to_shifts_and_follows_cursor() {
    let mut q = Queue::new(1, "DnD");
    q.append_many((1..=5).map(track));

    // Drag first track to the end: cursor follows the moved track.
    q.set_current(0);
    assert!(q.move_track_to(0, 4));
    assert_eq!(
        q.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
        vec![2, 3, 4, 5, 1]
    );
    assert_eq!(q.current_index(), Some(4));

    // Drag last track back to the front: tracks after it shift down.
    assert!(q.move_track_to(4, 0));
    assert_eq!(
        q.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(q.current_index(), Some(0));

    // Moving a track across the cursor shifts the cursor to track it.
    q.set_current(2);
    assert!(q.move_track_to(0, 4));
    assert_eq!(
        q.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
        vec![2, 3, 4, 5, 1]
    );
    assert_eq!(q.tracks[q.current_index().unwrap()].id, 3);
    assert!(q.move_track_to(4, 0));
    assert_eq!(q.tracks[q.current_index().unwrap()].id, 3);

    // No-ops.
    assert!(!q.move_track_to(1, 1), "same slot is a no-op");
    assert!(!q.move_track_to(0, 5), "out of bounds is a no-op");
    assert!(!q.move_track_to(9, 0), "out of bounds is a no-op");
}

#[test]
fn queue_store_activation_and_removal() {
    let mut store = QueueStore::default();
    assert!(store.is_empty());
    assert!(store.active_queue().is_none());

    let a = store.create("A");
    let b = store.create("B");
    assert_eq!(store.active, Some(b));
    assert_eq!(store.active_queue().map(|q| q.id), Some(b));

    // Switching active invalidates the stale materialized queue until reloaded.
    store.set_active(a);
    assert_eq!(store.active, Some(a));
    assert!(store.active_queue().is_none());

    // Removing a non-active queue leaves the active selection untouched.
    assert_eq!(store.remove(b), None);
    assert_eq!(store.active, Some(a));
    assert_eq!(
        store.remove(b),
        None,
        "removing an unknown queue is a no-op"
    );
    assert_eq!(store.remove(a), None);
    assert_eq!(store.active, None, "removing the last queue clears active");

    // Default name is used when no explicit name is provided
    let mut store = QueueStore::default();
    store.create(DEFAULT_QUEUE_NAME);
    assert_eq!(store.summaries[0].name, "Queue");
}

#[test]
fn queue_store_remove_active_selects_neighbor() {
    let mut store = QueueStore::default();
    let a = store.create("A");
    let b = store.create("B");
    let c = store.create("C");

    // Active is C; switch to middle (B) and delete it -> falls back to C
    // (same-index neighbor), clearing the stale materialized queue.
    store.set_active(b);
    assert_eq!(store.remove(b), Some(c));
    assert_eq!(store.active, Some(c));
    assert!(store.active_queue().is_none());
    assert_eq!(store.summaries.len(), 2);

    // Delete the last active (C) -> falls back to previous (A's neighbor).
    store.set_active(c);
    assert_eq!(store.remove(c), Some(a));
    assert_eq!(store.active, Some(a));

    // Deleting an unknown id is a no-op with no fallback.
    assert_eq!(store.remove(999), None);
    assert_eq!(store.active, Some(a));
}

#[test]
fn queue_window_math_covers_large_queues() {
    let capacity = queue_window_capacity();
    assert_eq!(capacity, QUEUE_WINDOW_ROWS + 2 * QUEUE_WINDOW_BUFFER_ROWS);

    // Small queues render whole.
    assert_eq!(queue_window_for_start(0, 0), (0, 0));
    assert_eq!(queue_window_for_start(50, 0), (0, 50));
    assert_eq!(queue_window_for_start(capacity, 0), (0, capacity));

    // Scrolling pulls the window back by the buffer and clamps to the tail.
    assert_eq!(queue_window_start(100_000, 0), 0);
    assert_eq!(
        queue_window_start(100_000, 500),
        500 - QUEUE_WINDOW_BUFFER_ROWS
    );
    assert_eq!(
        queue_window_start(100_000, 99_999),
        100_000 - capacity,
        "window start clamps to the tail"
    );
    assert_eq!(
        queue_window_for_start(100_000, usize::MAX),
        (100_000 - capacity, 100_000),
        "out-of-range starts clamp to the tail"
    );
    let (start, end) = queue_window_for_start(100_000, 9_900);
    assert_eq!(start, 9_900);
    assert_eq!(end - start, capacity);
}

#[test]
fn queue_store_reuse_and_replacement() {
    let mut store = QueueStore::default();
    store.load_summaries(vec![record(3, "Mixtape", 2)]);
    assert_eq!(store.next_id, 4);

    let id = store.find_id_by_name("Mixtape").expect("found by name");
    assert_eq!(id, 3);

    assert!(store.replace_with_tracks(id, (1..=4).map(track).collect()));
    assert_eq!(store.active, Some(3));
    assert_eq!(store.active_queue().map(|q| q.tracks.len()), Some(4));
    assert_eq!(store.summaries[0].track_count, 4);
}

#[test]
fn queue_store_activate_with_tracks_uses_summary() {
    let mut store = QueueStore::default();
    store.load_summaries(vec![QueueRecord {
        id: 7,
        name: "Road Trip".to_string(),
        current: 2,
        source_kind: "album".to_string(),
        source_value: Some("Album X".to_string()),
        source_playlist: None,
        track_count: 5,
    }]);

    store.activate_with_tracks(7, (1..=5).map(track).collect());
    let active = store.active_queue().expect("active queue");
    assert_eq!(active.source, QueueSource::Album);
    assert_eq!(active.source_value.as_deref(), Some("Album X"));
    assert_eq!(active.current, 2);
    assert_eq!(active.len(), 5);
}

#[test]
fn queue_source_roundtrips() {
    for source in [
        QueueSource::Manual,
        QueueSource::Album,
        QueueSource::Artist,
        QueueSource::Search,
        QueueSource::Playlist,
        QueueSource::AllSongs,
    ] {
        assert_eq!(QueueSource::from_str(source.as_str()), source);
    }
}
