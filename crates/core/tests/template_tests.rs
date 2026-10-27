use core_engine::{
    format_template, AlbumSummary, ArtistSummary, PlaylistSummary, QueueRecord, TrackRow,
};

#[test]
fn test_template_formatting_placeholders() {
    let track = TrackRow {
        id: 101,
        title: "Yūaku no Inori - Anima immortalis est.".to_string(),
        artist: "Imperial Circus Dead Decadence".into(),
        album: "Yūaku no Inori - Anima immortalis est.".into(),
        duration_ms: 472_000,
        file_path: "/music/ICDD - Anima.flac".to_string(),
        duration_label: "7:52".to_string(),
        year: Some(2025),
        track_num: Some(1),
        disc_num: Some(1),
        album_artist: Some("Imperial Circus Dead Decadence".into()),
        genre: Some("Symphonic Melodic Death Metal".into()),
    };

    assert_eq!(
        format_template("{title}", &track),
        "Yūaku no Inori - Anima immortalis est."
    );
    assert_eq!(
        format_template("{track_num}. {title}", &track),
        "1. Yūaku no Inori - Anima immortalis est."
    );
    assert_eq!(
        format_template("{title} - {artist} ({year})", &track),
        "Yūaku no Inori - Anima immortalis est. - Imperial Circus Dead Decadence (2025)"
    );
    assert_eq!(
        format_template("{title}[ • {genre}][ • {year}]", &track),
        "Yūaku no Inori - Anima immortalis est. • Symphonic Melodic Death Metal • 2025"
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
        album: "Yūaku no Inori - Anima immortalis est.".into(),
        artist: "Imperial Circus Dead Decadence".into(),
        track_count: 1,
        total_duration_ms: 472_000,
        year: Some(2025),
        cover_track_id: Some(101),
        added_timestamp: 1733529600,
        year_label: "2025".to_string(),
        tracks_label: "1 track".to_string(),
        duration_label: "7:52".to_string(),
    };

    // Both exact field name (album) and alias (title) work
    assert_eq!(
        format_template("{album}", &album),
        "Yūaku no Inori - Anima immortalis est."
    );
    assert_eq!(
        format_template("{title}", &album),
        "Yūaku no Inori - Anima immortalis est."
    );
    assert_eq!(
        format_template("{artist}[ • {year_label}]", &album),
        "Imperial Circus Dead Decadence • 2025"
    );
    assert_eq!(
        format_template("{tracks_label} • {duration_label}", &album),
        "1 track • 7:52"
    );
}

#[test]
fn test_artist_summary_formatting() {
    let artist = ArtistSummary {
        artist: "Imperial Circus Dead Decadence".into(),
        track_count: 48,
        album_count: 5,
        stats_label: "5 albums • 48 tracks".to_string(),
    };

    assert_eq!(
        format_template("{artist}", &artist),
        "Imperial Circus Dead Decadence"
    );
    assert_eq!(
        format_template("{name}", &artist),
        "Imperial Circus Dead Decadence"
    );
    assert_eq!(
        format_template("{stats_label}", &artist),
        "5 albums • 48 tracks"
    );
    assert_eq!(
        format_template("{artist} ({albums} albums, {tracks} tracks)", &artist),
        "Imperial Circus Dead Decadence (5 albums, 48 tracks)"
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
        name: "Album: Yūaku no Inori - Anima immortalis est.".to_string(),
        current: 0,
        source_kind: "album".to_string(),
        source_value: Some("Yūaku no Inori - Anima immortalis est.".to_string()),
        source_playlist: None,
        track_count: 1,
    };

    assert_eq!(
        format_template("{name} ({track_count})", &queue),
        "Album: Yūaku no Inori - Anima immortalis est. (1)"
    );
}
