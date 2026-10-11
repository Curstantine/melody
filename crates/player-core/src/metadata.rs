use std::fs::File;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey, Value};
use symphonia::core::probe::Hint;
use thiserror::Error;

use crate::model::NewTrack;

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

    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;

    let format = &mut probed.format;

    let mut title: Option<String> = None;
    let mut artist: Option<String> = None;
    let mut album: Option<String> = None;
    let mut genre: Option<String> = None;
    let mut track_num: Option<u32> = None;

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
                StandardTagKey::Album if album.is_none() => album = Some(val_str),
                StandardTagKey::Genre if genre.is_none() => genre = Some(val_str),
                StandardTagKey::TrackNumber if track_num.is_none() => {
                    if let Ok(num) = val_str.parse::<u32>() {
                        track_num = Some(num);
                    } else if let Some(first) = val_str.split('/').next() {
                        track_num = first.trim().parse::<u32>().ok();
                    }
                }
                _ => {}
            }
        } else {
            let key_upper = key.to_uppercase();
            match key_upper.as_str() {
                "TITLE" | "TIT2" if title.is_none() => title = Some(val_str),
                "ARTIST" | "TPE1" if artist.is_none() => artist = Some(val_str),
                "ALBUM" | "TALB" if album.is_none() => album = Some(val_str),
                "GENRE" | "TCON" if genre.is_none() => genre = Some(val_str),
                "TRACK" | "TRACKNUMBER" | "TRCK" if track_num.is_none() => {
                    if let Ok(num) = val_str.parse::<u32>() {
                        track_num = Some(num);
                    } else if let Some(first) = val_str.split('/').next() {
                        track_num = first.trim().parse::<u32>().ok();
                    }
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
    let track = format.tracks().iter().find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL);

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
            // Some formats store bitrate in bits per second
            // e.g. MP3 / AAC
            if let Some(br) = params.bits_per_sample {
                if let Some(sr) = params.sample_rate {
                    bit_rate = Some((br as u64) * (sr as u64) * (params.channels.map(|c| c.count() as u64).unwrap_or(2)));
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
        album,
        genre,
        duration_ms,
        track_num,
    })
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
