use std::{fs::File, path::Path};

use model::NewTrack;
use symphonia::core::{
    codecs::{
        CODEC_TYPE_AAC, CODEC_TYPE_ALAC, CODEC_TYPE_FLAC, CODEC_TYPE_MP3, CODEC_TYPE_NULL,
        CODEC_TYPE_OPUS, CODEC_TYPE_PCM_F32BE, CODEC_TYPE_PCM_F32LE, CODEC_TYPE_PCM_S16BE,
        CODEC_TYPE_PCM_S16LE, CODEC_TYPE_PCM_S24BE, CODEC_TYPE_PCM_S24LE, CODEC_TYPE_PCM_S32BE,
        CODEC_TYPE_PCM_S32LE, CODEC_TYPE_VORBIS,
    },
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::{MetadataOptions, MetadataRevision, StandardTagKey, StandardVisualKey, Tag},
    probe::{Hint, ProbeResult},
};

use crate::{
    channel_layout_label, format_name_for_path, is_lyrics_tag_key, sanitize_tag_bytes,
    AudioTechnicalInfo, MetadataError, ParsedTags,
};

fn probe_path(path: &Path) -> Result<ProbeResult, MetadataError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    Ok(probed)
}

fn apply_symphonia_tag(parsed: &mut ParsedTags, tag: &Tag) {
    let val_str = tag.value.to_string();
    match tag.std_key {
        Some(StandardTagKey::TrackTitle) => parsed.parse_tag("TITLE", &val_str),
        Some(StandardTagKey::Artist) => parsed.parse_tag("ARTIST", &val_str),
        Some(StandardTagKey::AlbumArtist) => parsed.parse_tag("ALBUMARTIST", &val_str),
        Some(StandardTagKey::Album) => parsed.parse_tag("ALBUM", &val_str),
        Some(StandardTagKey::Genre) => parsed.parse_tag("GENRE", &val_str),
        Some(StandardTagKey::TrackNumber) => parsed.parse_tag("TRACKNUMBER", &val_str),
        Some(StandardTagKey::DiscNumber) => parsed.parse_tag("DISCNUMBER", &val_str),
        Some(StandardTagKey::Date | StandardTagKey::ReleaseDate | StandardTagKey::OriginalDate) => {
            parsed.parse_tag("DATE", &val_str);
        }
        Some(StandardTagKey::Composer) => parsed.parse_tag("COMPOSER", &val_str),
        Some(StandardTagKey::Label) => parsed.parse_tag("LABEL", &val_str),
        Some(StandardTagKey::MusicBrainzTrackId) => {
            parsed.parse_tag("MUSICBRAINZ_TRACKID", &val_str);
        }
        Some(StandardTagKey::MusicBrainzArtistId) => {
            parsed.parse_tag("MUSICBRAINZ_ARTISTID", &val_str);
        }
        Some(StandardTagKey::MusicBrainzAlbumId) => {
            parsed.parse_tag("MUSICBRAINZ_ALBUMID", &val_str);
        }
        _ => parsed.parse_tag(&tag.key, &val_str),
    }
}

fn extract_lyrics_from_rev(rev: &MetadataRevision) -> Option<String> {
    for tag in rev.tags() {
        let is_lyrics =
            matches!(tag.std_key, Some(StandardTagKey::Lyrics)) || is_lyrics_tag_key(&tag.key);
        if is_lyrics {
            let val_str = tag.value.to_string();
            let cleaned = sanitize_tag_bytes(val_str.as_bytes())
                .replace("\r\n", "\n")
                .replace('\r', "\n");
            if !cleaned.is_empty() {
                return Some(cleaned);
            }
        }
    }
    None
}

fn extract_visual_from_rev(rev: &MetadataRevision) -> Option<Vec<u8>> {
    let visuals = rev.visuals();
    if visuals.is_empty() {
        return None;
    }

    if let Some(front) = visuals
        .iter()
        .find(|v| v.usage == Some(StandardVisualKey::FrontCover) && !v.data.is_empty())
    {
        return Some(front.data.to_vec());
    }

    visuals
        .iter()
        .find(|v| !v.data.is_empty())
        .map(|v| v.data.to_vec())
}

pub(crate) fn extract_technical_info(path: &Path, duration_ms: u64) -> Option<AudioTechnicalInfo> {
    let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let probed = probe_path(path).ok()?;

    let track = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;

    let params = &track.codec_params;
    let sample_rate_hz = params.sample_rate.unwrap_or(44100);
    let channels = params.channels.map(|c| c.count() as u32).unwrap_or(2);
    let bit_depth = params.bits_per_sample.or(params.bits_per_coded_sample);

    let codec_name = match params.codec {
        CODEC_TYPE_FLAC => "FLAC".to_string(),
        CODEC_TYPE_MP3 => "MP3 (MPEG Audio Layer III)".to_string(),
        CODEC_TYPE_AAC => "AAC (Advanced Audio Coding)".to_string(),
        CODEC_TYPE_VORBIS => "Vorbis".to_string(),
        CODEC_TYPE_OPUS => "Opus".to_string(),
        CODEC_TYPE_ALAC => "ALAC (Apple Lossless)".to_string(),
        CODEC_TYPE_PCM_S16LE | CODEC_TYPE_PCM_S16BE | CODEC_TYPE_PCM_S24LE
        | CODEC_TYPE_PCM_S24BE | CODEC_TYPE_PCM_S32LE | CODEC_TYPE_PCM_S32BE
        | CODEC_TYPE_PCM_F32LE | CODEC_TYPE_PCM_F32BE => "PCM (Uncompressed)".to_string(),
        _ => path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "Unknown".to_string()),
    };

    let bitrate_kbps = if duration_ms > 0 && file_size_bytes > 0 {
        Some(((file_size_bytes * 8) / duration_ms) as u32)
    } else {
        None
    };

    Some(AudioTechnicalInfo {
        format_name: format_name_for_path(path),
        codec_name,
        sample_rate_hz,
        bit_depth,
        channels,
        channel_layout: channel_layout_label(channels),
        bitrate_kbps,
        file_size_bytes,
    })
}

pub(crate) fn extract_metadata(path: &Path) -> Result<NewTrack, MetadataError> {
    let mut probed = probe_path(path)?;
    let mut tags = ParsedTags::default();

    if let Some(rev) = probed.format.metadata().current() {
        for tag in rev.tags() {
            apply_symphonia_tag(&mut tags, tag);
        }
    }

    if let Some(meta) = probed.metadata.get() {
        if let Some(rev) = meta.current() {
            for tag in rev.tags() {
                apply_symphonia_tag(&mut tags, tag);
            }
        }
    }

    let duration_ms = probed
        .format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .and_then(|track| {
            let n_frames = track.codec_params.n_frames?;
            if let Some(tb) = track.codec_params.time_base {
                let time = tb.calc_time(n_frames);
                Some(time.seconds * 1000 + (time.frac * 1000.0).round() as u64)
            } else {
                let sr = track.codec_params.sample_rate? as u64;
                (n_frames * 1000).checked_div(sr)
            }
        })
        .unwrap_or(0);

    Ok(tags.into_new_track(path, duration_ms))
}

pub(crate) fn extract_lyrics(path: &Path) -> Option<String> {
    let mut probed = probe_path(path).ok()?;

    if let Some(rev) = probed.format.metadata().current() {
        if let Some(lyrics) = extract_lyrics_from_rev(rev) {
            return Some(lyrics);
        }
    }

    if let Some(meta) = probed.metadata.get() {
        if let Some(rev) = meta.current() {
            if let Some(lyrics) = extract_lyrics_from_rev(rev) {
                return Some(lyrics);
            }
        }
    }

    None
}

pub(crate) fn extract_embedded_cover_art(path: &Path) -> Option<Vec<u8>> {
    let mut probed = probe_path(path).ok()?;

    if let Some(rev) = probed.format.metadata().current() {
        if let Some(art) = extract_visual_from_rev(rev) {
            return Some(art);
        }
    }

    if let Some(meta) = probed.metadata.get() {
        if let Some(rev) = meta.current() {
            if let Some(art) = extract_visual_from_rev(rev) {
                return Some(art);
            }
        }
    }

    None
}
