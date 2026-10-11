#![allow(dead_code)]

#[path = "../src/components/mod.rs"]
mod components;
#[path = "../src/icons.rs"]
mod icons;
#[path = "../src/message.rs"]
mod message;
#[path = "../src/theme/mod.rs"]
mod theme;
#[path = "../src/views/lyrics.rs"]
mod lyrics;

use player_core::{AudioTechnicalInfo, Track};
use theme::ThemeConfig;

#[test]
fn test_now_playing_empty_state_renders() {
    let theme_cfg = ThemeConfig::default();

    let _element = lyrics::view(
        &theme_cfg,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        None,
    );
}

#[test]
fn test_now_playing_with_track_and_next_track() {
    let theme_cfg = ThemeConfig::default();

    let track = Track {
        id: 1,
        file_path: "/music/track1.flac".to_string(),
        title: "Neon Dreams".to_string(),
        artist: "Synthwave Master".to_string(),
        album_artist: None,
        album: "Future Echoes".to_string(),
        genre: Some("Synthwave".to_string()),
        duration_ms: 215_000,
        track_num: Some(3),
        disc_num: Some(1),
        year: Some(2025),
        composer: Some("Composer X".to_string()),
        label: Some("Vibe Records".to_string()),
        musicbrainz_track_id: None,
        musicbrainz_artist_id: None,
        musicbrainz_album_id: None,
        added_timestamp: 1700000000,
    };

    let next_track = Track {
        id: 2,
        file_path: "/music/track2.flac".to_string(),
        title: "Night Cruise".to_string(),
        artist: "Synthwave Master".to_string(),
        album_artist: None,
        album: "Future Echoes".to_string(),
        genre: Some("Synthwave".to_string()),
        duration_ms: 198_000,
        track_num: Some(4),
        disc_num: Some(1),
        year: Some(2025),
        composer: None,
        label: None,
        musicbrainz_track_id: None,
        musicbrainz_artist_id: None,
        musicbrainz_album_id: None,
        added_timestamp: 1700000001,
    };

    let tech_info = AudioTechnicalInfo {
        format_name: "FLAC Lossless".to_string(),
        codec_name: "FLAC".to_string(),
        sample_rate_hz: 48000,
        bit_depth: Some(24),
        channels: 2,
        channel_layout: "Stereo".to_string(),
        bitrate_kbps: Some(960),
        file_size_bytes: 25_800_000,
    };

    let lyrics_text = "[00:00.00] Cruising down the neon street\n[00:15.00] Bass in the night";

    let _element = lyrics::view(
        &theme_cfg,
        Some(&track),
        None,
        None,
        Some(lyrics_text),
        Some(next_track),
        None,
        true,
        true,
        true,
        Some(&tech_info),
    );
}
