use std::fs::File;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value};
use symphonia::core::probe::Hint;
use thiserror::Error;

use crate::model::NewTrack;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioTechnicalInfo {
    pub format_name: String,
    pub codec_name: String,
    pub sample_rate_hz: u32,
    pub bit_depth: Option<u32>,
    pub channels: u32,
    pub channel_layout: String,
    pub bitrate_kbps: Option<u32>,
    pub file_size_bytes: u64,
}

impl AudioTechnicalInfo {
    pub fn formatted_file_size(&self) -> String {
        let bytes = self.file_size_bytes;
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn formatted_sample_rate(&self) -> String {
        if self.sample_rate_hz % 1000 == 0 {
            format!("{} kHz", self.sample_rate_hz / 1000)
        } else {
            format!("{:.1} kHz", self.sample_rate_hz as f64 / 1000.0)
        }
    }

    pub fn formatted_quality(&self) -> String {
        match self.bit_depth {
            Some(bd) => format!("{}-bit / {}", bd, self.formatted_sample_rate()),
            None => self.formatted_sample_rate(),
        }
    }
}

pub fn extract_technical_info(path: &Path, duration_ms: u64) -> Option<AudioTechnicalInfo> {
    let file = File::open(path).ok()?;
    let file_size_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    if let Some(ref e) = ext {
        hint.with_extension(e);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)?;

    let params = &track.codec_params;
    let sample_rate_hz = params.sample_rate.unwrap_or(44100);
    let channels = params.channels.map(|c| c.count() as u32).unwrap_or(2);
    let bit_depth = params.bits_per_sample;

    let channel_layout = match channels {
        1 => "Mono".to_string(),
        2 => "Stereo".to_string(),
        3 => "2.1 Surround".to_string(),
        4 => "Quadraphonic".to_string(),
        6 => "5.1 Surround".to_string(),
        8 => "7.1 Surround".to_string(),
        ch => format!("{} ch", ch),
    };

    let codec_name = match params.codec {
        symphonia::core::codecs::CODEC_TYPE_FLAC => "FLAC".to_string(),
        symphonia::core::codecs::CODEC_TYPE_MP3 => "MP3 (MPEG Audio Layer III)".to_string(),
        symphonia::core::codecs::CODEC_TYPE_AAC => "AAC (Advanced Audio Coding)".to_string(),
        symphonia::core::codecs::CODEC_TYPE_VORBIS => "Vorbis".to_string(),
        symphonia::core::codecs::CODEC_TYPE_OPUS => "Opus".to_string(),
        symphonia::core::codecs::CODEC_TYPE_PCM_S16LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S16BE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S24LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S24BE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S32LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_S32BE
        | symphonia::core::codecs::CODEC_TYPE_PCM_F32LE
        | symphonia::core::codecs::CODEC_TYPE_PCM_F32BE => "PCM (Uncompressed)".to_string(),
        symphonia::core::codecs::CODEC_TYPE_ALAC => "ALAC (Apple Lossless)".to_string(),
        _ => ext
            .as_deref()
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "Unknown".to_string()),
    };

    let format_name = match ext.as_deref() {
        Some("flac") => "FLAC Lossless",
        Some("mp3") => "MP3 Audio",
        Some("wav") => "WAV Audio",
        Some("ogg") | Some("oga") => "Ogg Container",
        Some("opus") => "Opus Audio",
        Some("m4a") | Some("mp4") | Some("aac") => "AAC / MP4",
        Some("aiff") | Some("aif") => "AIFF Audio",
        _ => "Audio",
    }
    .to_string();

    let bitrate_kbps = if duration_ms > 0 && file_size_bytes > 0 {
        let dur_secs = duration_ms as f64 / 1000.0;
        let kbps = (file_size_bytes as f64 * 8.0) / dur_secs / 1000.0;
        Some(kbps.round() as u32)
    } else {
        None
    };

    Some(AudioTechnicalInfo {
        format_name,
        codec_name,
        sample_rate_hz,
        bit_depth,
        channels,
        channel_layout,
        bitrate_kbps,
        file_size_bytes,
    })
}

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Symphonia probe/decode error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("Unsupported audio format or no tracks found")]
    UnsupportedFormat,
}

pub fn extract_metadata(path: &Path) -> Result<NewTrack, MetadataError> {
    let file = File::open(path)?;
    let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let format = &mut probed.format;

    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut album_artist: Option<String> = None;
    let mut album: Option<String> = None;
    let mut genre: Option<String> = None;
    let mut track_num: Option<u32> = None;
    let mut disc_num: Option<u32> = None;
    let mut year: Option<u32> = None;
    let mut composer: Option<String> = None;
    let mut label: Option<String> = None;
    let mut musicbrainz_track_id: Option<String> = None;
    let mut musicbrainz_artist_id: Option<String> = None;
    let mut musicbrainz_album_id: Option<String> = None;

    // Helper to extract tag values
    let mut parse_tag = |std_key: Option<StandardTagKey>, key: &str, value: &Value| {
        let val_str = match value {
            Value::String(s) => s.trim().to_string(),
            Value::UnsignedInt(n) => n.to_string(),
            Value::SignedInt(n) => n.to_string(),
            _ => return,
        };

        if let Some(sk) = std_key {
            match sk {
                StandardTagKey::TrackTitle if title.is_none() => title = Some(val_str),
                StandardTagKey::Artist if artist.is_none() => artist = Some(val_str),
                StandardTagKey::AlbumArtist if album_artist.is_none() => {
                    album_artist = Some(val_str)
                }
                StandardTagKey::Album if album.is_none() => album = Some(val_str),
                StandardTagKey::Genre if genre.is_none() => genre = Some(val_str),
                StandardTagKey::TrackNumber if track_num.is_none() => {
                    track_num = parse_index_slash(&val_str);
                }
                StandardTagKey::DiscNumber if disc_num.is_none() => {
                    disc_num = parse_index_slash(&val_str);
                }
                StandardTagKey::Date
                | StandardTagKey::ReleaseDate
                | StandardTagKey::OriginalDate
                    if year.is_none() =>
                {
                    year = parse_year(&val_str);
                }
                StandardTagKey::Composer if composer.is_none() => composer = Some(val_str),
                StandardTagKey::Label if label.is_none() => label = Some(val_str),
                StandardTagKey::MusicBrainzTrackId | StandardTagKey::MusicBrainzRecordingId
                    if musicbrainz_track_id.is_none() =>
                {
                    musicbrainz_track_id = Some(val_str);
                }
                StandardTagKey::MusicBrainzArtistId if musicbrainz_artist_id.is_none() => {
                    musicbrainz_artist_id = Some(val_str);
                }
                StandardTagKey::MusicBrainzAlbumId if musicbrainz_album_id.is_none() => {
                    musicbrainz_album_id = Some(val_str);
                }
                _ => {}
            }
        } else {
            let key_upper = key.to_uppercase();
            match key_upper.as_str() {
                "TITLE" | "TIT2" if title.is_none() => title = Some(val_str),
                "ARTIST" | "TPE1" if artist.is_none() => artist = Some(val_str),
                "ALBUMARTIST" | "ALBUM ARTIST" | "TPE2" | "AART" if album_artist.is_none() => {
                    album_artist = Some(val_str)
                }
                "ALBUM" | "TALB" if album.is_none() => album = Some(val_str),
                "GENRE" | "TCON" if genre.is_none() => genre = Some(val_str),
                "TRACK" | "TRACKNUMBER" | "TRCK" if track_num.is_none() => {
                    track_num = parse_index_slash(&val_str);
                }
                "DISC" | "DISCNUMBER" | "TPOS" if disc_num.is_none() => {
                    disc_num = parse_index_slash(&val_str);
                }
                "DATE" | "YEAR" | "TYER" | "TDRC" | "TDRL" | "ORIGINALDATE" if year.is_none() => {
                    year = parse_year(&val_str);
                }
                "COMPOSER" | "TCOM" if composer.is_none() => composer = Some(val_str),
                "LABEL" | "PUBLISHER" | "ORGANIZATION" | "TPUB" if label.is_none() => {
                    label = Some(val_str)
                }
                "MUSICBRAINZ_TRACKID" | "MUSICBRAINZ TRACK ID" | "MUSICBRAINZ_RECORDINGID"
                    if musicbrainz_track_id.is_none() =>
                {
                    musicbrainz_track_id = Some(val_str);
                }
                "MUSICBRAINZ_ARTISTID" | "MUSICBRAINZ ARTIST ID"
                    if musicbrainz_artist_id.is_none() =>
                {
                    musicbrainz_artist_id = Some(val_str);
                }
                "MUSICBRAINZ_ALBUMID" | "MUSICBRAINZ ALBUM ID"
                    if musicbrainz_album_id.is_none() =>
                {
                    musicbrainz_album_id = Some(val_str);
                }
                _ => {}
            }
        }
    };

    // Check container metadata
    if let Some(metadata) = format.metadata().current() {
        for tag in metadata.tags() {
            parse_tag(tag.std_key, &tag.key, &tag.value);
        }
    }

    // Also check probe sideband metadata if available
    if let Some(metadata) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        for tag in metadata.tags() {
            parse_tag(tag.std_key, &tag.key, &tag.value);
        }
    }

    // Determine duration
    let mut duration_ms = 0u64;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL);

    if let Some(t) = track {
        let params = &t.codec_params;
        if let (Some(n_frames), Some(sample_rate)) = (params.n_frames, params.sample_rate) {
            if sample_rate > 0 {
                duration_ms = (n_frames * 1000) / (sample_rate as u64);
            }
        } else if let Some(tb) = params.time_base {
            if let Some(n_frames) = params.n_frames {
                let time = tb.calc_time(n_frames);
                duration_ms = (time.seconds * 1000) + ((time.frac * 1000.0) as u64);
            }
        }

        // If duration is still 0 and we have a bitrate and file length, estimate duration
        if duration_ms == 0 {
            let mut bit_rate = None;
            if let Some(br) = params.bits_per_sample {
                if let Some(sr) = params.sample_rate {
                    bit_rate = Some(
                        (br as u64)
                            * (sr as u64)
                            * (params.channels.map(|c| c.count() as u64).unwrap_or(2)),
                    );
                }
            }
            if let Some(br) = bit_rate {
                if br > 0 && file_len > 0 {
                    duration_ms = (file_len * 8 * 1000) / br;
                }
            }
        }
    }

    let default_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    let title = title.unwrap_or(default_title);
    let artist = artist.unwrap_or_else(|| "Unknown Artist".to_string());
    let album = album.unwrap_or_else(|| "Unknown Album".to_string());

    Ok(NewTrack {
        file_path: path.to_string_lossy().to_string(),
        title,
        artist,
        album_artist,
        album,
        genre,
        duration_ms,
        track_num,
        disc_num,
        year,
        composer,
        label,
        musicbrainz_track_id,
        musicbrainz_artist_id,
        musicbrainz_album_id,
    })
}

fn parse_year(val: &str) -> Option<u32> {
    let s = val.trim();
    for window in s.as_bytes().windows(4) {
        if window.iter().all(|b| b.is_ascii_digit()) {
            if let Ok(y_str) = std::str::from_utf8(window) {
                if let Ok(y) = y_str.parse::<u32>() {
                    if (1800..=2100).contains(&y) {
                        return Some(y);
                    }
                }
            }
        }
    }
    None
}

fn parse_index_slash(val: &str) -> Option<u32> {
    if let Ok(n) = val.parse::<u32>() {
        Some(n)
    } else {
        val.split('/')
            .next()
            .and_then(|s| s.trim().parse::<u32>().ok())
    }
}

/// Extracts album cover art image data from either embedded audio metadata
/// (ID3, Vorbis Comments, FLAC picture block, MP4 covr) or directory image files
/// (`cover.jpg`, `folder.png`, `front.jpg`, etc.).
pub fn extract_cover_art(path: &Path) -> Option<Vec<u8>> {
    // 1. Try reading embedded visual art via Symphonia
    if let Ok(file) = File::open(path) {
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        if let Ok(mut probed) = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        ) {
            // Check container metadata for visuals
            if let Some(metadata) = probed.format.metadata().current() {
                if let Some(visual) = metadata.visuals().first() {
                    return Some(visual.data.to_vec());
                }
            }

            // Check probe metadata for visuals
            if let Some(metadata) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
                if let Some(visual) = metadata.visuals().first() {
                    return Some(visual.data.to_vec());
                }
            }
        }
    }

    // 2. Fallback: check track parent directory for standard cover art filenames
    if let Some(parent) = path.parent() {
        let candidates = [
            "cover.jpg",
            "cover.jpeg",
            "cover.png",
            "folder.jpg",
            "folder.jpeg",
            "folder.png",
            "front.jpg",
            "front.jpeg",
            "front.png",
            "album.jpg",
            "album.jpeg",
            "album.png",
            "art.jpg",
            "art.jpeg",
            "art.png",
        ];

        for candidate in candidates {
            let img_path = parent.join(candidate);
            if img_path.is_file() {
                if let Ok(bytes) = std::fs::read(&img_path) {
                    return Some(bytes);
                }
            }
        }
    }

    None
}
