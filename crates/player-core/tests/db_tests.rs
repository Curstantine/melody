use player_core::{format_duration, LibraryDatabase, NewTrack, SortDirection, SortField};

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
            ..Default::default()
        };
        db.insert_or_update_track(&track)
            .expect("Failed to insert track");
    }

    // Test total count
    let total = db.get_total_track_count(None).expect("Count failed");
    assert_eq!(total, 105);

    // Test filtered count
    let alpha_count = db
        .get_total_track_count(Some("Alpha"))
        .expect("Filter count failed");
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

#[test]
fn test_album_grouping_and_compilations() {
    let db = LibraryDatabase::in_memory().expect("Failed to create in-memory db");

    // Compilation album with different track artists, no album_artist set
    let compilation_tracks = vec![
        NewTrack {
            file_path: "/music/comp_01.mp3".to_string(),
            title: "Summer Beat".to_string(),
            artist: "DJ Sol".to_string(),
            album_artist: None,
            album: "Summer Vibes 2024".to_string(),
            duration_ms: 180000,
            track_num: Some(1),
            year: Some(2024),
            ..Default::default()
        },
        NewTrack {
            file_path: "/music/comp_02.mp3".to_string(),
            title: "Ocean Waves".to_string(),
            artist: "Aqua Sound".to_string(),
            album_artist: None,
            album: "Summer Vibes 2024".to_string(),
            duration_ms: 210000,
            track_num: Some(2),
            year: Some(2024),
            ..Default::default()
        },
        NewTrack {
            file_path: "/music/comp_03.mp3".to_string(),
            title: "Sunset Glow".to_string(),
            artist: "Chill Master".to_string(),
            album_artist: None,
            album: "Summer Vibes 2024".to_string(),
            duration_ms: 195000,
            track_num: Some(3),
            year: Some(2024),
            ..Default::default()
        },
    ];

    for t in &compilation_tracks {
        db.insert_or_update_track(t).expect("Insert track failed");
    }

    // Single-artist album
    let solo_tracks = vec![
        NewTrack {
            file_path: "/music/solo_01.mp3".to_string(),
            title: "Intro".to_string(),
            artist: "Luna".to_string(),
            album_artist: Some("Luna".to_string()),
            album: "Moonlight".to_string(),
            duration_ms: 120000,
            track_num: Some(1),
            year: Some(2022),
            ..Default::default()
        },
        NewTrack {
            file_path: "/music/solo_02.mp3".to_string(),
            title: "Starry Night".to_string(),
            artist: "Luna".to_string(),
            album_artist: Some("Luna".to_string()),
            album: "Moonlight".to_string(),
            duration_ms: 240000,
            track_num: Some(2),
            year: Some(2022),
            ..Default::default()
        },
    ];

    for t in &solo_tracks {
        db.insert_or_update_track(t).expect("Insert track failed");
    }

    let albums = db.get_albums().expect("get_albums failed");
    assert_eq!(albums.len(), 2, "Should have exactly 2 aggregated albums");

    let comp_album = albums
        .iter()
        .find(|a| a.album == "Summer Vibes 2024")
        .unwrap();
    assert_eq!(comp_album.track_count, 3);
    assert_eq!(comp_album.artist, "Various Artists");
    assert_eq!(comp_album.year, Some(2024));
    assert!(comp_album.cover_track_path.is_some());

    let solo_album = albums.iter().find(|a| a.album == "Moonlight").unwrap();
    assert_eq!(solo_album.track_count, 2);
    assert_eq!(solo_album.artist, "Luna");
    assert_eq!(solo_album.year, Some(2022));
}

#[test]
fn test_extra_metadata_tags_persistence() {
    let db = LibraryDatabase::in_memory().expect("Failed to create in-memory db");

    let track = NewTrack {
        file_path: "/music/classical_01.flac".to_string(),
        title: "Symphony No. 5 in C Minor, Op. 67: I. Allegro con brio".to_string(),
        artist: "Vienna Philharmonic".to_string(),
        album_artist: Some("Ludwig van Beethoven".to_string()),
        album: "Beethoven: The Symphonies".to_string(),
        genre: Some("Classical".to_string()),
        duration_ms: 432000,
        track_num: Some(1),
        disc_num: Some(2),
        year: Some(1975),
        composer: Some("Ludwig van Beethoven".to_string()),
        label: Some("Deutsche Grammophon".to_string()),
        musicbrainz_track_id: Some("9b48c4fa-4d64-4e78-9e6b-0b5c1ec824c1".to_string()),
        musicbrainz_artist_id: Some("1f9a2795-2ec3-4412-b250-9d0071582216".to_string()),
        musicbrainz_album_id: Some("d8329ecb-3e58-450f-a4ec-4f123c52a925".to_string()),
    };

    let id = db
        .insert_or_update_track(&track)
        .expect("Insert track failed");
    let fetched = db
        .get_track_by_id(id)
        .expect("Query failed")
        .expect("Track not found");

    assert_eq!(fetched.title, track.title);
    assert_eq!(
        fetched.album_artist,
        Some("Ludwig van Beethoven".to_string())
    );
    assert_eq!(fetched.disc_num, Some(2));
    assert_eq!(fetched.year, Some(1975));
    assert_eq!(fetched.composer, Some("Ludwig van Beethoven".to_string()));
    assert_eq!(fetched.label, Some("Deutsche Grammophon".to_string()));
    assert_eq!(
        fetched.musicbrainz_track_id,
        Some("9b48c4fa-4d64-4e78-9e6b-0b5c1ec824c1".to_string())
    );
    assert_eq!(
        fetched.musicbrainz_artist_id,
        Some("1f9a2795-2ec3-4412-b250-9d0071582216".to_string())
    );
    assert_eq!(
        fetched.musicbrainz_album_id,
        Some("d8329ecb-3e58-450f-a4ec-4f123c52a925".to_string())
    );
}

#[test]
fn test_library_folders_and_clear_tracks() {
    let db = LibraryDatabase::in_memory().expect("Failed to create in-memory db");

    // Initially empty
    let folders = db.get_library_folders().expect("get folders failed");
    assert!(folders.is_empty());

    // Add folders
    let added = db
        .add_library_folder("/home/user/Music")
        .expect("add folder failed");
    assert!(added);
    let added_dup = db
        .add_library_folder("/home/user/Music")
        .expect("add dup failed");
    assert!(!added_dup); // Ignored due to UNIQUE constraint
    db.add_library_folder("/mnt/external/Audio")
        .expect("add 2nd folder failed");

    let folders = db.get_library_folders().expect("get folders failed");
    assert_eq!(folders.len(), 2);
    assert!(folders.contains(&"/home/user/Music".to_string()));
    assert!(folders.contains(&"/mnt/external/Audio".to_string()));

    // Remove folder
    let removed = db
        .remove_library_folder("/home/user/Music")
        .expect("remove failed");
    assert!(removed);
    let folders = db.get_library_folders().expect("get folders failed");
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0], "/mnt/external/Audio");

    // Insert track and verify clear_all_tracks
    let track = NewTrack {
        file_path: "/mnt/external/Audio/test.mp3".to_string(),
        title: "Test".to_string(),
        artist: "Test Artist".to_string(),
        album: "Test Album".to_string(),
        genre: None,
        duration_ms: 60000,
        ..Default::default()
    };
    db.insert_or_update_track(&track)
        .expect("insert track failed");
    assert_eq!(db.get_total_track_count(None).unwrap(), 1);

    let cleared = db.clear_all_tracks().expect("clear failed");
    assert_eq!(cleared, 1);
    assert_eq!(db.get_total_track_count(None).unwrap(), 0);
}
