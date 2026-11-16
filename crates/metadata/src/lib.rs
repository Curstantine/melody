use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use model::NewTrack;

#[cfg(all(feature = "ffmpeg", feature = "symphonia"))]
compile_error!("Features `ffmpeg` and `symphonia` are mutually exclusive in `metadata`.");

pub mod error;
#[cfg(feature = "ffmpeg")]
mod ffmpeg_backend;
#[cfg(feature = "symphonia")]
mod symphonia_backend;
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

    fn formatted_sample_rate(&self) -> String {
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

pub(crate) fn channel_layout_label(channels: u32) -> String {
    match channels {
        1 => "Mono".to_string(),
        2 => "Stereo".to_string(),
        6 => "5.1 Surround".to_string(),
        8 => "7.1 Surround".to_string(),
        n => format!("{n} Channels"),
    }
}

pub(crate) fn format_name_for_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    match ext.as_deref() {
        Some("flac") => "FLAC Lossless",
        Some("mp3") => "MP3 Audio",
        Some("wav") => "WAV Audio",
        Some("ogg") | Some("oga") => "Ogg Container",
        Some("opus") => "Opus Audio",
        Some("m4a") | Some("mp4") | Some("aac") => "AAC / MP4",
        Some("aiff") | Some("aif") => "AIFF Audio",
        _ => "Audio",
    }
    .to_string()
}

#[derive(Default)]
pub(crate) struct ParsedTags {
    title: Option<String>,
    artist: Option<String>,
    album_artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    track_num: Option<u32>,
    disc_num: Option<u32>,
    year: Option<u32>,
    composer: Option<String>,
    label: Option<String>,
    musicbrainz_track_id: Option<String>,
    musicbrainz_artist_id: Option<String>,
    musicbrainz_album_id: Option<String>,
}

impl ParsedTags {
    pub(crate) fn parse_tag(&mut self, key: &str, val_str: &str) {
        let val = sanitize_tag_bytes(val_str.as_bytes());
        if val.is_empty() {
            return;
        }

        let key_upper = key.to_uppercase();
        match key_upper.as_str() {
            "TITLE" | "TIT2" if self.title.is_none() => self.title = Some(val),
            "ARTIST" | "TPE1" if self.artist.is_none() => self.artist = Some(val),
            "ALBUMARTIST" | "ALBUM ARTIST" | "ALBUM_ARTIST" | "TPE2" | "AART"
                if self.album_artist.is_none() =>
            {
                self.album_artist = Some(val)
            }
            "ALBUM" | "TALB" if self.album.is_none() => self.album = Some(val),
            "GENRE" | "TCON" if self.genre.is_none() => self.genre = Some(val),
            "TRACK" | "TRACKNUMBER" | "TRCK" if self.track_num.is_none() => {
                self.track_num = parse_index_slash(&val);
            }
            "DISC" | "DISCNUMBER" | "TPOS" if self.disc_num.is_none() => {
                self.disc_num = parse_index_slash(&val);
            }
            "DATE" | "YEAR" | "TYER" | "TDRC" | "TDRL" | "ORIGINALDATE" if self.year.is_none() => {
                self.year = parse_year(&val);
            }
            "COMPOSER" | "TCOM" if self.composer.is_none() => self.composer = Some(val),
            "LABEL" | "PUBLISHER" | "ORGANIZATION" | "TPUB" if self.label.is_none() => {
                self.label = Some(val)
            }
            "MUSICBRAINZ_TRACKID" | "MUSICBRAINZ TRACK ID" | "MUSICBRAINZ_RECORDINGID"
                if self.musicbrainz_track_id.is_none() =>
            {
                self.musicbrainz_track_id = Some(val);
            }
            "MUSICBRAINZ_ARTISTID" | "MUSICBRAINZ ARTIST ID"
                if self.musicbrainz_artist_id.is_none() =>
            {
                self.musicbrainz_artist_id = Some(val);
            }
            "MUSICBRAINZ_ALBUMID" | "MUSICBRAINZ ALBUM ID"
                if self.musicbrainz_album_id.is_none() =>
            {
                self.musicbrainz_album_id = Some(val);
            }
            _ => {}
        }
    }

    pub(crate) fn into_new_track(self, path: &Path, duration_ms: u64) -> NewTrack {
        let default_title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .map(|s| s.replace('\0', ""))
            .unwrap_or_else(|| "Unknown Title".to_string());

        NewTrack {
            file_path: path.to_string_lossy().replace('\0', ""),
            title: self.title.unwrap_or(default_title),
            artist: self.artist.unwrap_or_else(|| "Unknown Artist".to_string()),
            album_artist: self.album_artist,
            album: self.album.unwrap_or_else(|| "Unknown Album".to_string()),
            genre: self.genre,
            duration_ms,
            track_num: self.track_num,
            disc_num: self.disc_num,
            year: self.year,
            composer: self.composer,
            label: self.label,
            musicbrainz_track_id: self.musicbrainz_track_id,
            musicbrainz_artist_id: self.musicbrainz_artist_id,
            musicbrainz_album_id: self.musicbrainz_album_id,
        }
    }
}

pub fn extract_technical_info(path: &Path, duration_ms: u64) -> Option<AudioTechnicalInfo> {
    #[cfg(feature = "ffmpeg")]
    {
        ffmpeg_backend::extract_technical_info(path, duration_ms)
    }
    #[cfg(feature = "symphonia")]
    {
        symphonia_backend::extract_technical_info(path, duration_ms)
    }
    #[cfg(not(any(feature = "ffmpeg", feature = "symphonia")))]
    {
        let _ = (path, duration_ms);
        None
    }
}

pub fn extract_metadata(path: &Path) -> Result<NewTrack, MetadataError> {
    #[cfg(feature = "ffmpeg")]
    {
        ffmpeg_backend::extract_metadata(path)
    }
    #[cfg(feature = "symphonia")]
    {
        symphonia_backend::extract_metadata(path)
    }
    #[cfg(not(any(feature = "ffmpeg", feature = "symphonia")))]
    {
        let _ = path;
        Err(MetadataError::UnsupportedFormat)
    }
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

fn is_lyrics_tag_key(key: &str) -> bool {
    let upper = key.to_uppercase();
    matches!(
        upper.as_str(),
        "LYRICS" | "UNSYNCEDLYRICS" | "UNSYNCED LYRICS" | "SYNCEDLYRICS" | "USLT" | "©LYR"
    ) || upper.starts_with("LYRICS-")
        || upper.starts_with("LYRICS:")
}

/// Extracts embedded lyrics from audio file container or stream metadata tags.
pub fn extract_lyrics(path: &Path) -> Option<String> {
    #[cfg(feature = "ffmpeg")]
    {
        ffmpeg_backend::extract_lyrics(path)
    }
    #[cfg(feature = "symphonia")]
    {
        symphonia_backend::extract_lyrics(path)
    }
    #[cfg(not(any(feature = "ffmpeg", feature = "symphonia")))]
    {
        let _ = path;
        None
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

    #[cfg(feature = "ffmpeg")]
    if let Some(art) = ffmpeg_backend::extract_embedded_cover_art(path) {
        return Some(art);
    }

    #[cfg(feature = "symphonia")]
    if let Some(art) = symphonia_backend::extract_embedded_cover_art(path) {
        return Some(art);
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

fn extract_flac_picture(path: &Path) -> Option<Vec<u8>> {
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

fn extract_id3_picture(path: &Path) -> Option<Vec<u8>> {
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

    #[test]
    fn test_lyrics_tag_key_matching() {
        assert!(is_lyrics_tag_key("LYRICS"));
        assert!(is_lyrics_tag_key("lyrics"));
        assert!(is_lyrics_tag_key("UNSYNCEDLYRICS"));
        assert!(is_lyrics_tag_key("unsynced lyrics"));
        assert!(is_lyrics_tag_key("SYNCEDLYRICS"));
        assert!(is_lyrics_tag_key("USLT"));
        assert!(is_lyrics_tag_key("©lyr"));
        assert!(is_lyrics_tag_key("lyrics-eng"));
        assert!(is_lyrics_tag_key("lyrics-xxx"));
        assert!(is_lyrics_tag_key("LYRICS:eng"));
        assert!(!is_lyrics_tag_key("TITLE"));
        assert!(!is_lyrics_tag_key("ARTIST"));
    }

    #[test]
    fn test_extract_lyrics_missing_file() {
        let missing = Path::new("/nonexistent/melody_missing_track.mp3");
        assert_eq!(extract_lyrics(missing), None);
    }
}
