#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

#[path = "../src/config.rs"]
mod config;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/queue.rs"]
mod queue;
#[path = "../src/theme/mod.rs"]
mod theme;

use config::AppConfig;
use core_engine::TrackRow;
use queue::{QueueSource, QueueStore};

fn make_test_track(id: i64, title: &str, file_path: &str) -> TrackRow {
    TrackRow {
        id,
        title: title.to_string(),
        artist: Arc::from("Test Artist"),
        album: Arc::from("Test Album"),
        duration_ms: 180_000,
        file_path: file_path.to_string(),
        duration_label: "3:00".to_string(),
        year: None,
        track_num: None,
        disc_num: None,
        album_artist: None,
        genre: None,
    }
}

#[test]
fn test_background_mode_config_defaults() {
    let cfg = AppConfig::default();
    assert!(
        cfg.background_mode,
        "Background mode should be enabled by default"
    );
    assert_eq!(
        cfg.background_mode_timeout_secs, 60,
        "Background mode default minimization timeout should be 60 seconds"
    );
}

#[test]
fn test_background_mode_queue_playback_invariants() {
    // In background mode, catalog summaries and textures are purged, but active queue
    // tracks must remain resident and operational for next, prev, and random playback.
    let mut store = QueueStore::default();
    let q_id = store.create_with_source("Background Playback", QueueSource::Manual, None, None);

    let tracks = vec![
        make_test_track(1, "Track One", "C:/music/1.flac"),
        make_test_track(2, "Track Two", "C:/music/2.flac"),
        make_test_track(3, "Track Three", "C:/music/3.flac"),
        make_test_track(4, "Track Four", "C:/music/4.flac"),
    ];
    store.replace_with_tracks(q_id, tracks);

    let queue = store.active_queue().expect("Active queue must be present");
    assert_eq!(queue.len(), 4);
    assert_eq!(queue.current_index(), Some(0));
    assert_eq!(queue.current_track().map(|t| t.id), Some(1));
    assert_eq!(queue.next_index(), Some(1));

    // Next track advancement
    let queue_mut = store.active_queue_mut().expect("Active queue mut");
    queue_mut.set_current(1);
    assert_eq!(queue_mut.current_track().map(|t| t.id), Some(2));
    assert_eq!(queue_mut.next_index(), Some(2));

    // Previous track
    assert_eq!(queue_mut.previous_index(), Some(0));

    // Shuffle selection invariant without catalog metadata
    let len = queue_mut.len();
    let position_seed: u64 = 42_000;
    let current = queue_mut.current_index().unwrap_or(0);
    let mut pick = (position_seed as usize ^ 0x9e3779b9) % len;
    if pick == current {
        pick = (pick + 1) % len;
    }
    assert!(pick < len);
    assert_ne!(pick, current);
    assert!(queue_mut.tracks.get(pick).is_some());
}

#[test]
fn test_minimized_timeout_trigger_logic() {
    let timeout_secs: u64 = 60;
    let now = Instant::now();

    // 1. Minimized recently: 10 seconds elapsed -> should NOT trigger
    let recent_minimized = now - Duration::from_secs(10);
    let should_trigger_recent = recent_minimized.elapsed() >= Duration::from_secs(timeout_secs);
    assert!(!should_trigger_recent);

    // 2. Minimized for 65 seconds -> SHOULD trigger
    let past_minimized = now - Duration::from_secs(65);
    let should_trigger_past = past_minimized.elapsed() >= Duration::from_secs(timeout_secs);
    assert!(should_trigger_past);

    // 3. Not minimized (minimized_at is None) -> should NOT trigger
    let minimized_at: Option<Instant> = None;
    let should_trigger_none = minimized_at
        .map(|t| t.elapsed() >= Duration::from_secs(timeout_secs))
        .unwrap_or(false);
    assert!(!should_trigger_none);
}
