use std::path::Path;
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

fn init_ffmpeg() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = ffmpeg_next::init();
    });
}

pub fn extract_technical_info(path: &Path, duration_ms: u64) -> Option<AudioTechnicalInfo> {
    init_ffmpeg();
    let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let ictx = ffmpeg_next::format::input(path).ok()?;

    let audio_stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Audio)?;
    let codec_params = audio_stream.parameters();
    let raw_par = unsafe { *codec_params.as_ptr() };

    let sample_rate_hz = if raw_par.sample_rate > 0 {
        raw_par.sample_rate as u32
    } else {
        44100
    };

    let channels = if raw_par.ch_layout.nb_channels > 0 {
        raw_par.ch_layout.nb_channels as u32
    } else {
        2
    };

    let bit_depth = if raw_par.bits_per_raw_sample > 0 {
        Some(raw_par.bits_per_raw_sample as u32)
    } else if raw_par.bits_per_coded_sample > 0 {
        Some(raw_par.bits_per_coded_sample as u32)
    } else {
        match raw_par.format {
            // AV_SAMPLE_FMT_U8 / U8P
            0 | 5 => Some(8),
            // AV_SAMPLE_FMT_S16 / S16P
            1 | 6 => Some(16),
            // AV_SAMPLE_FMT_S32 / S32P / FLT / FLTP
            2 | 7 | 3 | 8 => Some(32),
            // AV_SAMPLE_FMT_DBL / DBLP / S64 / S64P
            4 | 9 | 10 | 11 => Some(64),
            _ => None,
        }
    };

    let channel_layout = match channels {
        1 => "Mono".to_string(),
        2 => "Stereo".to_string(),
        3 => "2.1 Surround".to_string(),
        4 => "Quadraphonic".to_string(),
        6 => "5.1 Surround".to_string(),
        8 => "7.1 Surround".to_string(),
        ch => format!("{} ch", ch),
    };

    let codec_id = codec_params.id();
    let codec_name = match codec_id {
        ffmpeg_next::codec::Id::FLAC => "FLAC".to_string(),
        ffmpeg_next::codec::Id::MP3 => "MP3 (MPEG Audio Layer III)".to_string(),
        ffmpeg_next::codec::Id::AAC => "AAC (Advanced Audio Coding)".to_string(),
        ffmpeg_next::codec::Id::VORBIS => "Vorbis".to_string(),
        ffmpeg_next::codec::Id::OPUS => "Opus".to_string(),
        ffmpeg_next::codec::Id::ALAC => "ALAC (Apple Lossless)".to_string(),
        ffmpeg_next::codec::Id::PCM_S16LE
        | ffmpeg_next::codec::Id::PCM_S16BE
        | ffmpeg_next::codec::Id::PCM_S24LE
        | ffmpeg_next::codec::Id::PCM_S24BE
        | ffmpeg_next::codec::Id::PCM_S32LE
        | ffmpeg_next::codec::Id::PCM_S32BE
        | ffmpeg_next::codec::Id::PCM_F32LE
        | ffmpeg_next::codec::Id::PCM_F32BE => "PCM (Uncompressed)".to_string(),
        _ => {
            let name = codec_id.name();
            if name.is_empty() {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_uppercase())
                    .unwrap_or_else(|| "Unknown".to_string())
            } else {
                name.to_uppercase()
            }
        }
    };

    let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
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
        let br = ictx.bit_rate();
        if br > 0 {
            Some((br / 1000) as u32)
        } else {
            None
        }
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
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] ffmpeg_next::Error),
    #[error("Unsupported audio format or no tracks found")]
    UnsupportedFormat,
}

pub fn extract_metadata(path: &Path) -> Result<NewTrack, MetadataError> {
    init_ffmpeg();
    let file_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let ictx = ffmpeg_next::format::input(path).map_err(|e| {
        if !path.exists() {
            MetadataError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"))
        } else {
            MetadataError::Ffmpeg(e)
        }
    })?;

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

    let mut parse_tag = |key: &str, val_str: &str| {
        let val = val_str.trim().to_string();
        if val.is_empty() {
            return;
        }

        let key_upper = key.to_uppercase();
        match key_upper.as_str() {
            "TITLE" | "TIT2" if title.is_none() => title = Some(val),
            "ARTIST" | "TPE1" if artist.is_none() => artist = Some(val),
            "ALBUMARTIST" | "ALBUM ARTIST" | "ALBUM_ARTIST" | "TPE2" | "AART"
                if album_artist.is_none() =>
            {
                album_artist = Some(val)
            }
            "ALBUM" | "TALB" if album.is_none() => album = Some(val),
            "GENRE" | "TCON" if genre.is_none() => genre = Some(val),
            "TRACK" | "TRACKNUMBER" | "TRCK" if track_num.is_none() => {
                track_num = parse_index_slash(&val);
            }
            "DISC" | "DISCNUMBER" | "TPOS" if disc_num.is_none() => {
                disc_num = parse_index_slash(&val);
            }
            "DATE" | "YEAR" | "TYER" | "TDRC" | "TDRL" | "ORIGINALDATE" if year.is_none() => {
                year = parse_year(&val);
            }
            "COMPOSER" | "TCOM" if composer.is_none() => composer = Some(val),
            "LABEL" | "PUBLISHER" | "ORGANIZATION" | "TPUB" if label.is_none() => {
                label = Some(val)
            }
            "MUSICBRAINZ_TRACKID" | "MUSICBRAINZ TRACK ID" | "MUSICBRAINZ_RECORDINGID"
                if musicbrainz_track_id.is_none() =>
            {
                musicbrainz_track_id = Some(val);
            }
            "MUSICBRAINZ_ARTISTID" | "MUSICBRAINZ ARTIST ID"
                if musicbrainz_artist_id.is_none() =>
            {
                musicbrainz_artist_id = Some(val);
            }
            "MUSICBRAINZ_ALBUMID" | "MUSICBRAINZ ALBUM ID"
                if musicbrainz_album_id.is_none() =>
            {
                musicbrainz_album_id = Some(val);
            }
            _ => {}
        }
    };

    // 1. Parse container metadata
    for (k, v) in ictx.metadata().iter() {
        parse_tag(k, v);
    }

    // 2. Parse audio stream metadata
    let audio_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio);
    if let Some(ref st) = audio_stream {
        for (k, v) in st.metadata().iter() {
            parse_tag(k, v);
        }
    }

    // Duration calculation
    let mut duration_ms = 0u64;
    let format_dur = ictx.duration();
    if format_dur > 0 {
        // format_dur is in AV_TIME_BASE units (microseconds)
        duration_ms = (format_dur as u64) / 1000;
    } else if let Some(ref st) = audio_stream {
        let dur = st.duration();
        if dur > 0 {
            let tb = st.time_base();
            let secs = (dur as f64) * (tb.numerator() as f64) / (tb.denominator() as f64);
            duration_ms = (secs * 1000.0).round() as u64;
        }
    }

    if duration_ms == 0 && file_len > 0 {
        let br = ictx.bit_rate();
        if br > 0 {
            duration_ms = (file_len * 8 * 1000) / (br as u64);
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
/// (attached pictures / cover art streams) or directory image files
/// (`cover.jpg`, `folder.png`, `front.jpg`, etc.).
pub fn extract_cover_art(path: &Path) -> Option<Vec<u8>> {
    init_ffmpeg();

    // 1. Try reading embedded visual art via FFmpeg attached pictures
    if let Ok(ictx) = ffmpeg_next::format::input(path) {
        for stream in ictx.streams() {
            let is_attached_pic = stream
                .disposition()
                .contains(ffmpeg_next::format::stream::Disposition::ATTACHED_PIC);

            let is_video = stream.parameters().medium() == ffmpeg_next::media::Type::Video;

            if is_attached_pic || is_video {
                unsafe {
                    let av_stream_ptr = stream.as_ptr();
                    let pkt = (*av_stream_ptr).attached_pic;
                    if !pkt.data.is_null() && pkt.size > 0 {
                        let slice = std::slice::from_raw_parts(pkt.data, pkt.size as usize);
                        return Some(slice.to_vec());
                    }
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
