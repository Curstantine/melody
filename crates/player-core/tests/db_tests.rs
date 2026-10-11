use player_core::{
    format_duration, LibraryDatabase, NewTrack, SortDirection, SortField,
};

#[test]
fn test_database_lifecycle_and_pagination() {
    let db = LibraryDatabase::in_memory().expect("Failed to create in-memory db");

    // Insert test tracks
    for i in 1..=105 {
        let track = NewTrack {
            file_path: format!("/music/track_{:03}.flac", i),
            title: format!("Song {:03}", i),
            artist: if i % 2 == 0 {
                "Artist Beta".to_string()
            } else {
                "Artist Alpha".to_string()
            },
            album: if i <= 50 {
                "Album One".to_string()
            } else {
                "Album Two".to_string()
            },
            genre: Some("Electronic".to_string()),
            duration_ms: (i as u64) * 2000,
            track_num: Some(i as u32),
        };
        db.insert_or_update_track(&track).expect("Failed to insert track");
    }

    // Test total count
    let total = db.get_total_track_count(None).expect("Count failed");
    assert_eq!(total, 105);

    // Test filtered count
    let alpha_count = db.get_total_track_count(Some("Alpha")).expect("Filter count failed");
    assert_eq!(alpha_count, 53); // odd numbers from 1 to 105

    // Test window pagination (limit 20, offset 0)
    let page1 = db
        .get_tracks_window(0, 20, None, SortField::Title, SortDirection::Ascending)
        .expect("Window failed");
    assert_eq!(page1.len(), 20);
    assert_eq!(page1[0].title, "Song 001");
    assert_eq!(page1[19].title, "Song 020");

    // Test second page (limit 20, offset 20)
    let page2 = db
        .get_tracks_window(20, 20, None, SortField::Title, SortDirection::Ascending)
        .expect("Window failed");
    assert_eq!(page2.len(), 20);
    assert_eq!(page2[0].title, "Song 021");

    // Test last partial page (offset 100, limit 20 -> 5 tracks)
    let page6 = db
        .get_tracks_window(100, 20, None, SortField::Title, SortDirection::Ascending)
        .expect("Window failed");
    assert_eq!(page6.len(), 5);

    // Test lightweight headers
    let headers = db.get_all_track_headers().expect("Headers failed");
    assert_eq!(headers.len(), 105);
    assert_eq!(headers[0].id, 1);

    // Test album aggregation
    let albums = db.get_albums().expect("Albums failed");
    assert!(!albums.is_empty());

    // Test artist aggregation
    let artists = db.get_artists().expect("Artists failed");
    assert_eq!(artists.len(), 2);
}

#[test]
fn test_duration_formatting() {
    assert_eq!(format_duration(500), "0:00");
    assert_eq!(format_duration(65000), "1:05");
    assert_eq!(format_duration(3665000), "1:01:05");
}

#[test]
fn test_cover_art_directory_fallback() {
    let temp_dir = std::env::temp_dir().join("vibecore_test_cover_art");
    let _ = std::fs::create_dir_all(&temp_dir);

    let song_path = temp_dir.join("test_song.flac");
    let cover_path = temp_dir.join("cover.jpg");
    let fake_img_data = b"\xFF\xD8\xFF\xE0dummyjpegdata";

    std::fs::write(&song_path, b"dummy audio content").expect("Write song failed");
    std::fs::write(&cover_path, fake_img_data).expect("Write cover failed");

    let extracted = player_core::extract_cover_art(&song_path);
    assert_eq!(extracted, Some(fake_img_data.to_vec()));

    // Test non-existent cover
    let isolated_dir = temp_dir.join("no_cover_sub");
    let _ = std::fs::create_dir_all(&isolated_dir);
    let isolated_song = isolated_dir.join("song2.mp3");
    std::fs::write(&isolated_song, b"dummy audio").expect("Write failed");

    let no_cover = player_core::extract_cover_art(&isolated_song);
    assert_eq!(no_cover, None);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
