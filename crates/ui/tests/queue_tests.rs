#![allow(dead_code)]

use std::sync::Arc;

#[path = "../src/queue.rs"]
mod queue;

use core_engine::TrackRow;
use queue::{Queue, QueueStore, DEFAULT_QUEUE_NAME};

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

    // removing the current track clamps the cursor back onto a valid slot
    q.set_current(1);
    q.remove(1);
    assert!(q.current_index().unwrap() < q.len());

    // move_track swaps and follows the moved track
    q.set_current(0);
    let before = q.tracks[0].id;
    assert!(q.move_track(0, false));
    assert_eq!(q.tracks[1].id, before);
    assert_eq!(q.current_index(), Some(1));
    assert!(!q.move_track(q.len() - 1, false), "cannot move past the end");
}

#[test]
fn queue_store_activation_and_removal() {
    let mut store = QueueStore::default();
    assert!(store.is_empty());
    assert!(store.active_queue().is_none());

    let a = store.create("A");
    let b = store.create("B");
    assert_eq!(store.active, Some(b));
    store.set_active(a);
    assert_eq!(store.active_queue().map(|q| q.id), Some(a));

    assert!(store.remove(b));
    assert!(!store.remove(b), "removing an unknown queue is a no-op");
    store.remove(a);
    assert_eq!(store.active, None, "removing the last queue clears active");

    // Default name is used when no explicit name is provided
    let mut store = QueueStore::default();
    store.create(DEFAULT_QUEUE_NAME);
    assert_eq!(store.active_queue().unwrap().name, "Queue");
}

#[test]
fn queue_store_persistence_roundtrip() {
    let mut store = QueueStore::default();
    let id = store.create("Persisted");
    if let Some(q) = store.active_queue_mut() {
        q.append_many((1..=4).map(track));
        q.set_current(2);
        q.source_playlist = Some("/playlists/foo.m3u8".to_string());
    }

    let path = std::env::temp_dir().join(format!(
        "melody_queue_test_{}.json",
        std::process::id()
    ));
    store.save(&path);

    let loaded = QueueStore::load(&path);
    let q = loaded.active_queue().expect("queue restored");
    assert_eq!(q.id, id);
    assert_eq!(q.name, "Persisted");
    assert_eq!(q.len(), 4);
    assert_eq!(q.current_index(), Some(2));
    assert_eq!(
        q.source_playlist.as_deref(),
        Some("/playlists/foo.m3u8")
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn queue_store_load_missing_file_is_empty() {
    let path = std::env::temp_dir().join("melody_queue_does_not_exist_xyz.json");
    let _ = std::fs::remove_file(&path);
    let store = QueueStore::load(&path);
    assert!(store.is_empty());
    assert!(store.active_queue().is_none());
}
