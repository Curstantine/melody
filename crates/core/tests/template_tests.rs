use core_engine::{
    format_template, AlbumSummary, ArtistSummary, PlaylistSummary, QueueRecord, TrackRow,
};

#[test]
fn test_template_formatting_placeholders() {
    let track = TrackRow {
        id: 10,
        title: "Time".to_string(),
        artist: "Pink Floyd".into(),
        album: "The Dark Side of the Moon".into(),
        duration_ms: 425_000,
        file_path: "/music/time.flac".to_string(),
        duration_label: "7:05".to_string(),
        year: Some(1973),
        track_num: Some(4),
        disc_num: Some(1),
        album_artist: Some("Pink Floyd".into()),
        genre: Some("Progressive Rock".into()),
    };

    assert_eq!(format_template("{title}", &track), "Time");
    assert_eq!(format_template("{track_num}. {title}", &track), "4. Time");
    assert_eq!(
        format_template("{title} - {artist} ({year})", &track),
        "Time - Pink Floyd (1973)"
    );
    assert_eq!(
        format_template("{title}[ • {genre}][ • {year}]", &track),
        "Time • Progressive Rock • 1973"
    );
}

#[test]
fn test_template_missing_fields_and_brackets() {
    let track = TrackRow {
        id: 11,
        title: "Unknown Track".to_string(),
        artist: "Unknown Artist".into(),
        album: "Unknown Album".into(),
        duration_ms: 120_000,
        file_path: "/music/track.mp3".to_string(),
        duration_label: "2:00".to_string(),
        year: None,
        track_num: None,
        disc_num: None,
        album_artist: None,
        genre: None,
    };

    // Conditional bracket group [ • {year}] is omitted cleanly because year is None
    assert_eq!(
        format_template("{title}[ • {year}]", &track),
        "Unknown Track"
    );

    // Dangling separator without brackets is cleaned up
    assert_eq!(format_template("{title} • {year}", &track), "Unknown Track");
}

#[test]
fn test_album_summary_formatting() {
    let album = AlbumSummary {
        album: "Wish You Were Here".into(),
        artist: "Pink Floyd".into(),
        track_count: 5,
        total_duration_ms: 2640000,
        year: Some(1975),
        cover_track_id: Some(1),
        added_timestamp: 1700000000,
        year_label: "1975".to_string(),
        tracks_label: "5 tracks".to_string(),
        duration_label: "44:00".to_string(),
    };

    // Both exact field name (album) and alias (title) work
    assert_eq!(format_template("{album}", &album), "Wish You Were Here");
    assert_eq!(format_template("{title}", &album), "Wish You Were Here");
    assert_eq!(
        format_template("{artist}[ • {year_label}]", &album),
        "Pink Floyd • 1975"
    );
    assert_eq!(
        format_template("{tracks_label} • {duration_label}", &album),
        "5 tracks • 44:00"
    );
}

#[test]
fn test_artist_summary_formatting() {
    let artist = ArtistSummary {
        artist: "Pink Floyd".into(),
        track_count: 120,
        album_count: 15,
        stats_label: "15 albums • 120 tracks".to_string(),
    };

    assert_eq!(format_template("{artist}", &artist), "Pink Floyd");
    assert_eq!(format_template("{name}", &artist), "Pink Floyd");
    assert_eq!(
        format_template("{stats_label}", &artist),
        "15 albums • 120 tracks"
    );
    assert_eq!(
        format_template("{artist} ({albums} albums, {tracks} tracks)", &artist),
        "Pink Floyd (15 albums, 120 tracks)"
    );
}

#[test]
fn test_playlist_summary_formatting() {
    let playlist = PlaylistSummary {
        id: 42,
        name: "Chill Vibes".to_string(),
        file_path: "/home/user/playlists/chill.m3u8".to_string(),
        track_count: 24,
        created_timestamp: 1700000000,
        modified_timestamp: 1700000000,
        managed: true,
        tracks_label: "24 tracks".to_string(),
    };

    assert_eq!(format_template("{name}", &playlist), "Chill Vibes");
    assert_eq!(format_template("{title}", &playlist), "Chill Vibes");
    assert_eq!(
        format_template("{name} ({tracks_label}) - {managed}", &playlist),
        "Chill Vibes (24 tracks) - Managed"
    );
}

#[test]
fn test_queue_record_formatting() {
    let queue = QueueRecord {
        id: 1,
        name: "Album: The Wall".to_string(),
        current: 2,
        source_kind: "album".to_string(),
        source_value: Some("The Wall".to_string()),
        source_playlist: None,
        track_count: 26,
    };

    assert_eq!(
        format_template("{name} ({track_count})", &queue),
        "Album: The Wall (26)"
    );
}
