use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use model::NewTrack;

pub mod error;
pub mod watcher;

pub use error::MetadataError;
pub use watcher::{LibraryWatcher, WatcherError};

/// Supported audio file extensions for library indexing.
pub const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "alac", "opus", "aiff",
];

/// Returns `true` if the given path has a supported audio file extension.
pub fn is_supported_audio_path<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            SUPPORTED_AUDIO_EXTENSIONS
                .iter()
                .any(|&s| ext.eq_ignore_ascii_case(s))
        })
        .unwrap_or(false)
}

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
        if self.sample_rate_hz.is_multiple_of(1000) {
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

    let audio_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio)?;
    let codec_params = audio_stream.parameters();
    let raw_par_ptr = unsafe { codec_params.as_ptr() };
    if raw_par_ptr.is_null() {
        return None;
    }
    let raw_par = unsafe { &*raw_par_ptr };

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

    let channel_layout = match channels {
        1 => "Mono".to_string(),
        2 => "Stereo".to_string(),
        6 => "5.1 Surround".to_string(),
        8 => "7.1 Surround".to_string(),
        n => format!("{n} Channels"),
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
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

    let bitrate_kbps = if ictx.bit_rate() > 0 {
        Some((ictx.bit_rate() / 1000) as u32)
    } else if duration_ms > 0 && file_size_bytes > 0 {
        Some(((file_size_bytes * 8) / duration_ms) as u32)
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

pub fn extract_metadata(path: &Path) -> Result<NewTrack, MetadataError> {
    init_ffmpeg();
    let file_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let ictx = ffmpeg_next::format::input(path).map_err(|e| {
        if !path.exists() {
            MetadataError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ))
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
        let val = sanitize_tag_bytes(val_str.as_bytes());
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
            "LABEL" | "PUBLISHER" | "ORGANIZATION" | "TPUB" if label.is_none() => label = Some(val),
            "MUSICBRAINZ_TRACKID" | "MUSICBRAINZ TRACK ID" | "MUSICBRAINZ_RECORDINGID"
                if musicbrainz_track_id.is_none() =>
            {
                musicbrainz_track_id = Some(val);
            }
            "MUSICBRAINZ_ARTISTID" | "MUSICBRAINZ ARTIST ID" if musicbrainz_artist_id.is_none() => {
                musicbrainz_artist_id = Some(val);
            }
            "MUSICBRAINZ_ALBUMID" | "MUSICBRAINZ ALBUM ID" if musicbrainz_album_id.is_none() => {
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
        .map(|s| s.to_string_lossy().to_string())
        .map(|s| s.replace('\0', ""))
        .unwrap_or_else(|| "Unknown Title".to_string());

    let title = title.unwrap_or(default_title);
    let artist = artist.unwrap_or_else(|| "Unknown Artist".to_string());
    let album = album.unwrap_or_else(|| "Unknown Album".to_string());

    Ok(NewTrack {
        file_path: path.to_string_lossy().replace('\0', ""),
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

pub fn sanitize_tag_bytes(bytes: &[u8]) -> String {
    if !bytes.contains(&0) {
        if let Ok(valid) = std::str::from_utf8(bytes) {
            return valid.trim().to_string();
        }
    }

    let filtered: Vec<u8> = bytes.iter().copied().filter(|&b| b != 0).collect();

    if let Ok(valid) = std::str::from_utf8(&filtered) {
        return valid.trim().to_string();
    }

    let has_consecutive_non_ascii = filtered.windows(2).any(|w| w[0] >= 0x80 && w[1] >= 0x80);

    if has_consecutive_non_ascii {
        let (cow_gbk, had_errors_gbk) = encoding_rs::GB18030.decode_without_bom_handling(&filtered);
        if !had_errors_gbk {
            return cow_gbk.trim().to_string();
        }

        let (cow_sjis, had_errors_sjis) =
            encoding_rs::SHIFT_JIS.decode_without_bom_handling(&filtered);
        if !had_errors_sjis {
            return cow_sjis.trim().to_string();
        }
    }

    let (cow_1252, had_errors_1252) =
        encoding_rs::WINDOWS_1252.decode_without_bom_handling(&filtered);
    if !had_errors_1252 {
        return cow_1252.trim().to_string();
    }

    if !has_consecutive_non_ascii {
        let (cow_gbk, had_errors_gbk) = encoding_rs::GB18030.decode_without_bom_handling(&filtered);
        if !had_errors_gbk {
            return cow_gbk.trim().to_string();
        }
    }

    String::from_utf8_lossy(&filtered).trim().to_string()
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

pub fn extract_cover_art(path: &Path) -> Option<Vec<u8>> {
    let is_flac = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("flac"))
        .unwrap_or(false);

    if is_flac {
        if let Some(art) = extract_flac_picture(path) {
            return Some(art);
        }
    }

    init_ffmpeg();

    if let Ok(ictx) = ffmpeg_next::format::input(path) {
        for stream in ictx.streams() {
            let is_attached_pic = stream
                .disposition()
                .contains(ffmpeg_next::format::stream::Disposition::ATTACHED_PIC);

            let is_video = stream.parameters().medium() == ffmpeg_next::media::Type::Video;

            if is_attached_pic || is_video {
                let av_stream_ptr = unsafe { stream.as_ptr() };
                if !av_stream_ptr.is_null() {
                    unsafe {
                        let pkt = (*av_stream_ptr).attached_pic;
                        if !pkt.data.is_null() && pkt.size > 0 {
                            let slice = std::slice::from_raw_parts(pkt.data, pkt.size as usize);
                            return Some(slice.to_vec());
                        }
                    }
                }
            }
        }
    }

    if !is_flac {
        if let Some(art) = extract_flac_picture(path) {
            return Some(art);
        }
    }

    if let Some(art) = extract_id3_picture(path) {
        return Some(art);
    }

    extract_directory_cover_art(path)
}

pub fn extract_flac_picture(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 4];
    file.read_exact(&mut header).ok()?;

    if &header[0..3] == b"ID3" {
        let mut id3_rest = [0u8; 6];
        file.read_exact(&mut id3_rest).ok()?;
        let flags = id3_rest[1];
        let tag_size = ((id3_rest[2] as u64 & 0x7F) << 21)
            | ((id3_rest[3] as u64 & 0x7F) << 14)
            | ((id3_rest[4] as u64 & 0x7F) << 7)
            | (id3_rest[5] as u64 & 0x7F);
        let mut total_skip = 10 + tag_size;
        if (flags & 0x10) != 0 {
            total_skip += 10;
        }
        file.seek(SeekFrom::Start(total_skip)).ok()?;
        file.read_exact(&mut header).ok()?;
    }

    if &header != b"fLaC" {
        return None;
    }

    let mut fallback_picture: Option<Vec<u8>> = None;

    loop {
        let mut block_header = [0u8; 4];
        if file.read_exact(&mut block_header).is_err() {
            break;
        }

        let is_last = (block_header[0] & 0x80) != 0;
        let block_type = block_header[0] & 0x7F;
        let block_len = ((block_header[1] as u64) << 16)
            | ((block_header[2] as u64) << 8)
            | (block_header[3] as u64);

        if block_type == 6 {
            if let Some((pic_type, data)) = parse_flac_picture_block(&mut file, block_len) {
                if pic_type == 3 {
                    return Some(data);
                }
                if fallback_picture.is_none() {
                    fallback_picture = Some(data);
                }
            }
        } else if file.seek(SeekFrom::Current(block_len as i64)).is_err() {
            break;
        }

        if is_last {
            break;
        }
    }

    fallback_picture
}

fn parse_flac_picture_block<R: Read + Seek>(
    reader: &mut R,
    block_len: u64,
) -> Option<(u32, Vec<u8>)> {
    if block_len < 32 {
        let _ = reader.seek(SeekFrom::Current(block_len as i64));
        return None;
    }

    let start_pos = reader.stream_position().ok()?;
    let mut u32_buf = [0u8; 4];

    reader.read_exact(&mut u32_buf).ok()?;
    let pic_type = u32::from_be_bytes(u32_buf);

    reader.read_exact(&mut u32_buf).ok()?;
    let mime_len = u32::from_be_bytes(u32_buf) as u64;

    if mime_len > block_len.saturating_sub(8) {
        let _ = reader.seek(SeekFrom::Start(start_pos + block_len));
        return None;
    }
    reader.seek(SeekFrom::Current(mime_len as i64)).ok()?;

    reader.read_exact(&mut u32_buf).ok()?;
    let desc_len = u32::from_be_bytes(u32_buf) as u64;

    if desc_len > block_len.saturating_sub(8 + mime_len + 4) {
        let _ = reader.seek(SeekFrom::Start(start_pos + block_len));
        return None;
    }
    reader.seek(SeekFrom::Current(desc_len as i64)).ok()?;

    reader.seek(SeekFrom::Current(16)).ok()?;

    reader.read_exact(&mut u32_buf).ok()?;
    let data_len = u32::from_be_bytes(u32_buf) as u64;

    if data_len == 0 || data_len > 35 * 1024 * 1024 {
        let _ = reader.seek(SeekFrom::Start(start_pos + block_len));
        return None;
    }

    let mut data = vec![0u8; data_len as usize];
    reader.read_exact(&mut data).ok()?;

    let _ = reader.seek(SeekFrom::Start(start_pos + block_len));

    Some((pic_type, data))
}

pub fn extract_id3_picture(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 10];
    if file.read_exact(&mut header).is_err() {
        return None;
    }

    if &header[0..3] != b"ID3" {
        return None;
    }

    let major_version = header[3];
    if !(2..=4).contains(&major_version) {
        return None;
    }

    let tag_size = ((header[6] as u64 & 0x7F) << 21)
        | ((header[7] as u64 & 0x7F) << 14)
        | ((header[8] as u64 & 0x7F) << 7)
        | (header[9] as u64 & 0x7F);

    let end_pos = 10 + tag_size;
    let mut fallback_data: Option<Vec<u8>> = None;

    while let Ok(current_pos) = file.stream_position() {
        if current_pos + 10 >= end_pos {
            break;
        }

        let mut frame_header = [0u8; 10];
        if file.read_exact(&mut frame_header).is_err() {
            break;
        }

        if frame_header[0] == 0 {
            break;
        }

        let frame_id = &frame_header[0..4];
        let frame_len = if major_version == 4 {
            ((frame_header[4] as u64 & 0x7F) << 21)
                | ((frame_header[5] as u64 & 0x7F) << 14)
                | ((frame_header[6] as u64 & 0x7F) << 7)
                | (frame_header[7] as u64 & 0x7F)
        } else {
            ((frame_header[4] as u64) << 24)
                | ((frame_header[5] as u64) << 16)
                | ((frame_header[6] as u64) << 8)
                | (frame_header[7] as u64)
        };

        if frame_len == 0 || current_pos + 10 + frame_len > end_pos {
            break;
        }

        if frame_id == b"APIC" {
            if frame_len <= 35 * 1024 * 1024 {
                let mut frame_data = vec![0u8; frame_len as usize];
                if file.read_exact(&mut frame_data).is_ok() {
                    if let Some((pic_type, img_data)) = parse_apic_frame(&frame_data) {
                        if pic_type == 3 {
                            return Some(img_data);
                        }
                        if fallback_data.is_none() {
                            fallback_data = Some(img_data);
                        }
                    }
                }
            } else {
                let _ = file.seek(SeekFrom::Current(frame_len as i64));
            }
        } else {
            let _ = file.seek(SeekFrom::Current(frame_len as i64));
        }
    }

    fallback_data
}

fn parse_apic_frame(data: &[u8]) -> Option<(u8, Vec<u8>)> {
    if data.len() < 5 {
        return None;
    }
    let encoding = data[0];
    let mut idx = 1;

    while idx < data.len() && data[idx] != 0 {
        idx += 1;
    }
    if idx >= data.len() {
        return None;
    }
    idx += 1;

    if idx >= data.len() {
        return None;
    }
    let pic_type = data[idx];
    idx += 1;

    if encoding == 1 || encoding == 2 {
        while idx + 1 < data.len() && !(data[idx] == 0 && data[idx + 1] == 0) {
            idx += 2;
        }
        idx = (idx + 2).min(data.len());
    } else {
        while idx < data.len() && data[idx] != 0 {
            idx += 1;
        }
        idx = (idx + 1).min(data.len());
    }

    if idx < data.len() {
        Some((pic_type, data[idx..].to_vec()))
    } else {
        None
    }
}

fn extract_directory_cover_art(path: &Path) -> Option<Vec<u8>> {
    let parent = path.parent()?;

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
                if !bytes.is_empty() {
                    return Some(bytes);
                }
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy().to_lowercase();
                    let is_standard_name = candidates.iter().any(|c| name_str == *c)
                        || name_str.starts_with("cover.")
                        || name_str.starts_with("folder.")
                        || name_str.starts_with("front.")
                        || name_str.starts_with("album.")
                        || name_str.starts_with("art.");

                    let has_img_ext = name_str.ends_with(".jpg")
                        || name_str.ends_with(".jpeg")
                        || name_str.ends_with(".png");

                    if is_standard_name && has_img_ext {
                        if let Ok(bytes) = std::fs::read(entry.path()) {
                            if !bytes.is_empty() {
                                return Some(bytes);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
            file_size_bytes: 52_428_800,
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

    #[test]
    fn test_cover_art_directory_fallback() {
        let temp_dir = std::env::temp_dir().join("melody_test_cover_art");
        let _ = std::fs::create_dir_all(&temp_dir);

        let song_path = temp_dir.join("test_song.flac");
        let cover_path = temp_dir.join("cover.jpg");
        let fake_img_data = b"\xFF\xD8\xFF\xE0dummyjpegdata";

        std::fs::write(&song_path, b"dummy audio content").expect("Write song failed");
        std::fs::write(&cover_path, fake_img_data).expect("Write cover failed");

        let extracted = extract_cover_art(&song_path);
        assert_eq!(extracted, Some(fake_img_data.to_vec()));

        let isolated_dir = temp_dir.join("no_cover_sub");
        let _ = std::fs::create_dir_all(&isolated_dir);
        let isolated_song = isolated_dir.join("song2.mp3");
        std::fs::write(&isolated_song, b"dummy audio").expect("Write failed");

        let no_cover = extract_cover_art(&isolated_song);
        assert_eq!(no_cover, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
