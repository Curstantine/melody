use std::path::PathBuf;
use std::time::Duration;

use core_engine::{
    is_supported_audio_path, LibraryDatabase, LibraryWatcher, NewTrack, SUPPORTED_AUDIO_EXTENSIONS,
};

#[test]
fn test_supported_audio_extensions_and_path_check() {
    assert!(SUPPORTED_AUDIO_EXTENSIONS.contains(&"mp3"));
    assert!(SUPPORTED_AUDIO_EXTENSIONS.contains(&"flac"));
    assert!(SUPPORTED_AUDIO_EXTENSIONS.contains(&"ogg"));

    assert!(is_supported_audio_path("song.mp3"));
    assert!(is_supported_audio_path("artist/track.FLAC"));
    assert!(is_supported_audio_path("/music/album/01.opus"));
    assert!(!is_supported_audio_path("image.jpg"));
    assert!(!is_supported_audio_path("notes.txt"));
    assert!(!is_supported_audio_path("file_without_ext"));
}

#[tokio::test]
async fn test_filter_new_paths() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("in-memory db should initialize");

    let p1 = "/music/existing1.flac";
    let p2 = "/music/existing2.mp3";
    let p3 = "/music/brand_new.opus";

    let track1 = NewTrack {
        file_path: p1.to_string(),
        title: "Track 1".to_string(),
        artist: "Artist".to_string(),
        album_artist: None,
        album: "Album".to_string(),
        genre: None,
        duration_ms: 180_000,
        track_num: Some(1),
        disc_num: Some(1),
        year: Some(2025),
        composer: None,
        label: None,
        musicbrainz_track_id: None,
        musicbrainz_artist_id: None,
        musicbrainz_album_id: None,
    };

    let track2 = NewTrack {
        file_path: p2.to_string(),
        title: "Track 2".to_string(),
        artist: "Artist".to_string(),
        album_artist: None,
        album: "Album".to_string(),
        genre: None,
        duration_ms: 200_000,
        track_num: Some(2),
        disc_num: Some(1),
        year: Some(2025),
        composer: None,
        label: None,
        musicbrainz_track_id: None,
        musicbrainz_artist_id: None,
        musicbrainz_album_id: None,
    };

    db.insert_tracks_batch(&[track1, track2])
        .await
        .expect("batch insert");

    let test_paths = vec![PathBuf::from(p1), PathBuf::from(p2), PathBuf::from(p3)];

    let new_only = db
        .filter_new_paths(&test_paths)
        .await
        .expect("filter_new_paths");
    assert_eq!(new_only.len(), 1);
    assert_eq!(new_only[0], PathBuf::from(p3));
}

#[tokio::test]
async fn test_scan_directories_new_skips_existing_files() {
    let base_temp = std::env::temp_dir().join(format!(
        "melody_test_scan_new_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&base_temp);

    let song1 = base_temp.join("song1.mp3");
    let song2 = base_temp.join("song2.mp3");
    std::fs::write(&song1, b"audio 1").unwrap();

    let db = LibraryDatabase::in_memory().await.unwrap();

    let track1 = NewTrack {
        file_path: song1.to_string_lossy().to_string(),
        title: "Song 1".to_string(),
        artist: "Artist".to_string(),
        album_artist: None,
        album: "Album".to_string(),
        genre: None,
        duration_ms: 120_000,
        track_num: Some(1),
        disc_num: Some(1),
        year: Some(2025),
        composer: None,
        label: None,
        musicbrainz_track_id: None,
        musicbrainz_artist_id: None,
        musicbrainz_album_id: None,
    };
    db.insert_tracks_batch(&[track1]).await.unwrap();

    // First scan: song1 is already in DB, so 0 new files are scanned
    let summary1 = db
        .scan_directories_new(&[&base_temp], None::<fn(usize, usize)>)
        .await
        .unwrap();
    assert_eq!(summary1.scanned_files, 0);
    assert_eq!(summary1.inserted_tracks, 0);

    // Add song2 and scan again
    std::fs::write(&song2, b"audio 2").unwrap();
    let summary2 = db
        .scan_directories_new(&[&base_temp], None::<fn(usize, usize)>)
        .await
        .unwrap();
    // Only the new song2 is scanned!
    assert_eq!(summary2.scanned_files, 1);

    let _ = std::fs::remove_dir_all(&base_temp);
}

#[test]
fn test_library_watcher_folder_management() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut watcher = LibraryWatcher::new(tx, Duration::from_millis(50)).expect("watcher init");

    let temp = std::env::temp_dir().join(format!(
        "melody_watcher_mgmt_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let dir1 = temp.join("dir1");
    let dir2 = temp.join("dir2");
    let dir3 = temp.join("dir3");
    let _ = std::fs::create_dir_all(&dir1);
    let _ = std::fs::create_dir_all(&dir2);
    let _ = std::fs::create_dir_all(&dir3);

    watcher.watch_folder(&dir1).unwrap();
    watcher.watch_folder(&dir2).unwrap();
    assert!(watcher.is_watching(&dir1));
    assert!(watcher.is_watching(&dir2));
    assert!(!watcher.is_watching(&dir3));

    // Unwatch dir1
    watcher.unwatch_folder(&dir1).unwrap();
    assert!(!watcher.is_watching(&dir1));
    assert!(watcher.is_watching(&dir2));

    // Update folders to dir2 and dir3
    watcher.update_folders(&[&dir2, &dir3]).unwrap();
    assert!(!watcher.is_watching(&dir1));
    assert!(watcher.is_watching(&dir2));
    assert!(watcher.is_watching(&dir3));

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_library_watcher_detects_new_file() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut watcher = LibraryWatcher::new(tx, Duration::from_millis(100)).expect("watcher init");

    let temp = std::env::temp_dir().join(format!(
        "melody_watcher_detect_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp);

    watcher.watch_folder(&temp).unwrap();

    let new_file = temp.join("test_song.flac");
    std::fs::write(&new_file, b"flac content").unwrap();

    // Wait for debounced notification
    let received = rx.recv_timeout(Duration::from_millis(2000));
    assert!(received.is_ok(), "Watcher should emit debounced event");
    let paths = received.unwrap();
    assert!(
        paths.iter().any(|p| p.ends_with("test_song.flac")),
        "Emitted paths {:?} should contain test_song.flac",
        paths
    );

    let _ = std::fs::remove_dir_all(&temp);
}
