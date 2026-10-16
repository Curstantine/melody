use core_engine::{
    format_duration, AudioTechnicalInfo, LibraryDatabase, NewTrack, QueueRecord, SortDirection,
    SortField, TrackFilter,
};

#[test]
fn test_audio_technical_info_formatting() {
    let info = AudioTechnicalInfo {
        format_name: "FLAC Lossless".to_string(),
        codec_name: "FLAC".to_string(),
        sample_rate_hz: 96000,
        bit_depth: Some(24),
        channels: 2,
        channel_layout: "Stereo".to_string(),
        bitrate_kbps: Some(1536),
        file_size_bytes: 52_428_800, // 50.0 MB
    };

    assert_eq!(info.formatted_file_size(), "50.0 MB");
    assert_eq!(info.formatted_sample_rate(), "96 kHz");
    assert_eq!(info.formatted_quality(), "24-bit / 96 kHz");

    let info_44k = AudioTechnicalInfo {
        format_name: "MP3 Audio".to_string(),
        codec_name: "MP3".to_string(),
        sample_rate_hz: 44100,
        bit_depth: None,
        channels: 1,
        channel_layout: "Mono".to_string(),
        bitrate_kbps: Some(320),
        file_size_bytes: 8_500_000,
    };

    assert_eq!(info_44k.formatted_sample_rate(), "44.1 kHz");
    assert_eq!(info_44k.formatted_quality(), "44.1 kHz");
    assert_eq!(info_44k.formatted_file_size(), "8.1 MB");
}

#[tokio::test]
async fn test_database_lifecycle_and_pagination() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

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
            year: Some(2000),
            ..Default::default()
        };
        db.insert_or_update_track(&track)
            .await
            .expect("Failed to insert track");
    }

    // Test total count
    let total = db.get_total_track_count(None).await.expect("Count failed");
    assert_eq!(total, 105);

    // Test filtered count
    let alpha_filter = TrackFilter::parse("Alpha");
    let alpha_count = db
        .get_total_track_count(Some(&alpha_filter))
        .await
        .expect("Filter count failed");
    assert_eq!(alpha_count, 53); // odd numbers from 1 to 105

    // Scoped album filter is exact (not a substring of title/artist)
    let album_one = TrackFilter::parse("album:\"Album One\"");
    let album_one_count = db
        .get_total_track_count(Some(&album_one))
        .await
        .expect("Album filter count failed");
    assert_eq!(album_one_count, 50);

    let album_rows = db
        .get_all_track_rows_filtered(
            Some(&album_one),
            SortField::TrackNum,
            SortDirection::Ascending,
        )
        .await
        .expect("Album rows failed");
    assert_eq!(album_rows.len(), 50);
    assert!(album_rows.iter().all(|t| t.album.as_ref() == "Album One"));

    // Year filter binds an integer column through a text placeholder.
    let year_filter = TrackFilter::parse("year:2000");
    assert_eq!(
        db.get_total_track_count(Some(&year_filter)).await.unwrap(),
        105
    );

    // Test window pagination (limit 20, offset 0)
    let page1 = db
        .get_tracks_window(0, 20, None, SortField::Title, SortDirection::Ascending)
        .await
        .expect("Window failed");
    assert_eq!(page1.len(), 20);
    assert_eq!(page1[0].title, "Song 001");
    assert_eq!(page1[19].title, "Song 020");

    // Test second page (limit 20, offset 20)
    let page2 = db
        .get_tracks_window(20, 20, None, SortField::Title, SortDirection::Ascending)
        .await
        .expect("Window failed");
    assert_eq!(page2.len(), 20);
    assert_eq!(page2[0].title, "Song 021");

    // Test last partial page (offset 100, limit 20 -> 5 tracks)
    let page6 = db
        .get_tracks_window(100, 20, None, SortField::Title, SortDirection::Ascending)
        .await
        .expect("Window failed");
    assert_eq!(page6.len(), 5);

    // Test lightweight headers
    let headers = db.get_all_track_headers().await.expect("Headers failed");
    assert_eq!(headers.len(), 105);
    assert_eq!(headers[0].id, 1);

    // Test album aggregation
    let albums = db.get_albums().await.expect("Albums failed");
    assert!(!albums.is_empty());

    // Test artist aggregation
    let artists = db.get_artists().await.expect("Artists failed");
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
    let temp_dir = std::env::temp_dir().join("melody_test_cover_art");
    let _ = std::fs::create_dir_all(&temp_dir);

    let song_path = temp_dir.join("test_song.flac");
    let cover_path = temp_dir.join("cover.jpg");
    let fake_img_data = b"\xFF\xD8\xFF\xE0dummyjpegdata";

    std::fs::write(&song_path, b"dummy audio content").expect("Write song failed");
    std::fs::write(&cover_path, fake_img_data).expect("Write cover failed");

    let extracted = core_engine::extract_cover_art(&song_path);
    assert_eq!(extracted, Some(fake_img_data.to_vec()));

    // Test non-existent cover
    let isolated_dir = temp_dir.join("no_cover_sub");
    let _ = std::fs::create_dir_all(&isolated_dir);
    let isolated_song = isolated_dir.join("song2.mp3");
    std::fs::write(&isolated_song, b"dummy audio").expect("Write failed");

    let no_cover = core_engine::extract_cover_art(&isolated_song);
    assert_eq!(no_cover, None);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_album_grouping_and_compilations() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

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
        db.insert_or_update_track(t)
            .await
            .expect("Insert track failed");
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
        db.insert_or_update_track(t)
            .await
            .expect("Insert track failed");
    }

    let albums = db.get_albums().await.expect("get_albums failed");
    assert_eq!(albums.len(), 2, "Should have exactly 2 aggregated albums");

    let comp_album = albums
        .iter()
        .find(|a| a.album.as_ref() == "Summer Vibes 2024")
        .unwrap();
    assert_eq!(comp_album.track_count, 3);
    assert_eq!(comp_album.artist.as_ref(), "Various Artists");
    assert_eq!(comp_album.year, Some(2024));
    assert!(comp_album.cover_track_id.is_some());

    let solo_album = albums
        .iter()
        .find(|a| a.album.as_ref() == "Moonlight")
        .unwrap();
    assert_eq!(solo_album.track_count, 2);
    assert_eq!(solo_album.artist.as_ref(), "Luna");
    assert_eq!(solo_album.year, Some(2022));
}

#[tokio::test]
async fn test_extra_metadata_tags_persistence() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

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
        .await
        .expect("Insert track failed");
    let fetched = db
        .get_track_by_id(id)
        .await
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

#[tokio::test]
async fn test_library_folders_and_clear_tracks() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

    // Initially empty
    let folders = db.get_library_folders().await.expect("get folders failed");
    assert!(folders.is_empty());

    // Add folders
    let added = db
        .add_library_folder("/home/user/Music")
        .await
        .expect("add folder failed");
    assert!(added);
    let added_dup = db
        .add_library_folder("/home/user/Music")
        .await
        .expect("add dup failed");
    assert!(!added_dup); // Ignored due to UNIQUE constraint
    db.add_library_folder("/mnt/external/Audio")
        .await
        .expect("add 2nd folder failed");

    let folders = db.get_library_folders().await.expect("get folders failed");
    assert_eq!(folders.len(), 2);
    assert!(folders.contains(&"/home/user/Music".to_string()));
    assert!(folders.contains(&"/mnt/external/Audio".to_string()));

    // Remove folder
    let removed = db
        .remove_library_folder("/home/user/Music")
        .await
        .expect("remove failed");
    assert!(removed);
    let folders = db.get_library_folders().await.expect("get folders failed");
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
        .await
        .expect("insert track failed");
    assert_eq!(db.get_total_track_count(None).await.unwrap(), 1);

    let cleared = db.clear_all_tracks().await.expect("clear failed");
    assert_eq!(cleared, 1);
    assert_eq!(db.get_total_track_count(None).await.unwrap(), 0);
}

#[tokio::test]
async fn test_scan_directory_symlink_traversal() {
    let base_temp = std::env::temp_dir().join(format!(
        "melody_test_scan_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let real_dir = base_temp.join("real_music");
    let scan_dir = base_temp.join("library_root");
    let _ = std::fs::create_dir_all(&real_dir);
    let _ = std::fs::create_dir_all(&scan_dir);

    // Create a symlink to real_dir inside scan_dir
    let symlinked_folder = scan_dir.join("symlinked_artists");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_dir, &symlinked_folder).expect("symlink creation failed");

    // Put a dummy audio file in real_dir
    let track_path = real_dir.join("song.mp3");
    std::fs::write(&track_path, b"test audio content").expect("write failed");

    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

    let summary = db
        .scan_directory(&scan_dir, None::<fn(usize, usize)>)
        .await
        .expect("Scan directory failed");

    // The file inside the symlinked directory was traversed and processed
    assert!(summary.scanned_files >= 1);

    let _ = std::fs::remove_dir_all(&base_temp);
}

#[tokio::test]
async fn test_scan_directories_progress_callback() {
    let base_temp = std::env::temp_dir().join(format!(
        "melody_test_progress_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let dir1 = base_temp.join("folder1");
    let dir2 = base_temp.join("folder2");
    let _ = std::fs::create_dir_all(&dir1);
    let _ = std::fs::create_dir_all(&dir2);

    std::fs::write(dir1.join("song1.mp3"), b"audio").unwrap();
    std::fs::write(dir1.join("song2.flac"), b"audio").unwrap();
    std::fs::write(dir2.join("song3.ogg"), b"audio").unwrap();

    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

    let progress_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_clone = progress_events.clone();

    let summary = db
        .scan_directories(
            &[&dir1, &dir2],
            Some(move |cur, tot| {
                progress_clone.lock().unwrap().push((cur, tot));
            }),
        )
        .await
        .expect("scan_directories failed");

    assert_eq!(summary.scanned_files, 3);
    let events = progress_events.lock().unwrap().clone();
    assert!(!events.is_empty());
    // Initial event reports (0, 3)
    assert_eq!(events[0], (0, 3));
    // Final event reports (3, 3)
    assert_eq!(events.last().copied(), Some((3, 3)));

    let _ = std::fs::remove_dir_all(&base_temp);
}

#[tokio::test]
async fn test_playlist_file_references() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

    // Initially empty
    assert!(db.get_playlists().await.unwrap().is_empty());

    let id = db
        .upsert_playlist_file("/playlists/Chill.m3u8", "Chill", 3)
        .await
        .expect("upsert failed");
    assert!(id > 0);

    // Upsert same path refreshes count/name rather than duplicating
    db.upsert_playlist_file("/playlists/Chill.m3u8", "Chill Vibes", 4)
        .await
        .expect("re-upsert failed");
    let playlists = db.get_playlists().await.unwrap();
    assert_eq!(playlists.len(), 1);
    assert_eq!(playlists[0].name, "Chill Vibes");
    assert_eq!(playlists[0].track_count, 4);
    assert_eq!(playlists[0].tracks_label, "4 tracks");

    // Rename by id
    assert!(db.rename_playlist_file(id, "Focus").await.unwrap());
    assert_eq!(db.get_playlists().await.unwrap()[0].name, "Focus");

    // Path -> id resolution (only indexed tracks match)
    let track = NewTrack {
        file_path: "/music/a.flac".to_string(),
        title: "A".to_string(),
        artist: "Artist".to_string(),
        album: "Album".to_string(),
        duration_ms: 1000,
        ..Default::default()
    };
    db.insert_or_update_track(&track).await.unwrap();

    let map = db
        .get_track_ids_by_paths(&[
            "/music/a.flac".to_string(),
            "/music/missing.flac".to_string(),
        ])
        .await
        .unwrap();
    assert_eq!(map.len(), 1);
    assert!(map.contains_key("/music/a.flac"));

    // Remove reference
    assert!(db.remove_playlist_file(id).await.unwrap());
    assert!(db.get_playlists().await.unwrap().is_empty());

    // Removing a track does not touch playlist references
    db.upsert_playlist_file("/playlists/Keep.m3u8", "Keep", 1)
        .await
        .unwrap();
    db.clear_all_tracks().await.unwrap();
    assert_eq!(db.get_playlists().await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_playback_queue_persistence() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

    let mut ids = Vec::new();
    for i in 1..=3 {
        let track = NewTrack {
            file_path: format!("/music/queue_{}.flac", i),
            title: format!("Queue Song {}", i),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 1000 * i as u64,
            ..Default::default()
        };
        ids.push(db.insert_or_update_track(&track).await.unwrap());
    }

    let record = QueueRecord {
        id: 1,
        name: "My Queue".to_string(),
        current: 0,
        source_kind: "manual".to_string(),
        source_value: None,
        source_playlist: None,
        track_count: 0,
    };
    db.upsert_queue(&record).await.unwrap();
    db.replace_queue_tracks(1, &ids).await.unwrap();
    db.set_active_queue_id(Some(1)).await.unwrap();

    let queues = db.list_queues().await.unwrap();
    assert_eq!(queues.len(), 1);
    assert_eq!(queues[0].name, "My Queue");
    assert_eq!(queues[0].track_count, 3);

    let tracks = db.load_queue_tracks(1).await.unwrap();
    assert_eq!(tracks.len(), 3);
    assert_eq!(tracks[0].id, ids[0]);
    assert_eq!(tracks[2].id, ids[2]);

    assert_eq!(db.get_active_queue_id().await.unwrap(), Some(1));

    // Cursor updates persist.
    db.set_queue_current(1, 2).await.unwrap();
    let queues = db.list_queues().await.unwrap();
    assert_eq!(queues[0].current, 2);

    // Replacing membership rewrites ordering.
    db.replace_queue_tracks(1, &[ids[2], ids[1], ids[0]])
        .await
        .unwrap();
    let tracks = db.load_queue_tracks(1).await.unwrap();
    assert_eq!(tracks[0].id, ids[2]);

    // Large memberships span multiple insert chunks without losing order.
    let big: Vec<i64> = (0..650).map(|i| ids[i % ids.len()]).collect();
    db.replace_queue_tracks(1, &big).await.unwrap();
    let tracks = db.load_queue_tracks(1).await.unwrap();
    assert_eq!(tracks.len(), big.len());
    for (pos, track) in tracks.iter().enumerate() {
        assert_eq!(track.id, big[pos], "order preserved at position {pos}");
    }

    // Deletion clears the queue and can clear the active selection.
    db.delete_queue(1).await.unwrap();
    db.set_active_queue_id(None).await.unwrap();
    assert!(db.list_queues().await.unwrap().is_empty());
    assert_eq!(db.get_active_queue_id().await.unwrap(), None);
}

#[tokio::test]
async fn test_insert_track_with_non_utf8_and_null_bytes() {
    let db = LibraryDatabase::in_memory()
        .await
        .expect("Failed to create in-memory db");

    // Construct a title with CP1252 / invalid UTF-8 byte (0x92 = right single quote in CP1252) and null byte
    let raw_bytes = b"the 8 o\x92clock\x00 news is going crazy";
    let sanitized = core_engine::metadata::sanitize_tag_bytes(raw_bytes);
    assert_eq!(sanitized, "the 8 o’clock news is going crazy");

    let track = NewTrack {
        file_path: "/music/test_track\x00.wav".to_string(),
        title: sanitized,
        artist: "KRAL".to_string(),
        album: "全部大コンピ, Vol.05".to_string(),
        duration_ms: 180000,
        ..Default::default()
    };

    // Both single insert and batch insert must succeed without "TEXT value contains invalid UTF-8"
    let id = db
        .insert_or_update_track(&track)
        .await
        .expect("Single insert failed");
    assert!(id > 0);

    let batch = vec![track.clone()];
    let inserted = db
        .insert_tracks_batch(&batch)
        .await
        .expect("Batch insert failed");
    assert_eq!(inserted, 1);
}
