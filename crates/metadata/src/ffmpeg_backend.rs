use std::path::Path;

use model::NewTrack;

use crate::{
    channel_layout_label, format_name_for_path, is_lyrics_tag_key, sanitize_tag_bytes,
    AudioTechnicalInfo, MetadataError, ParsedTags,
};

fn init_ffmpeg() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = ffmpeg_next::init();
    });
}

pub(crate) fn extract_technical_info(path: &Path, duration_ms: u64) -> Option<AudioTechnicalInfo> {
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

    let bitrate_kbps = if ictx.bit_rate() > 0 {
        Some((ictx.bit_rate() / 1000) as u32)
    } else if duration_ms > 0 && file_size_bytes > 0 {
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

    let mut tags = ParsedTags::default();

    for (k, v) in ictx.metadata().iter() {
        tags.parse_tag(k, v);
    }

    let audio_stream = ictx.streams().best(ffmpeg_next::media::Type::Audio);
    if let Some(ref st) = audio_stream {
        for (k, v) in st.metadata().iter() {
            tags.parse_tag(k, v);
        }
    }

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

    Ok(tags.into_new_track(path, duration_ms))
}

pub(crate) fn extract_lyrics(path: &Path) -> Option<String> {
    init_ffmpeg();
    let ictx = ffmpeg_next::format::input(path).ok()?;

    let extract_from_dict = |dict: ffmpeg_next::DictionaryRef<'_>| -> Option<String> {
        for (k, v) in dict.iter() {
            if is_lyrics_tag_key(k) {
                let cleaned = sanitize_tag_bytes(v.as_bytes())
                    .replace("\r\n", "\n")
                    .replace('\r', "\n");
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
        None
    };

    if let Some(lyrics) = extract_from_dict(ictx.metadata()) {
        return Some(lyrics);
    }

    if let Some(st) = ictx.streams().best(ffmpeg_next::media::Type::Audio) {
        if let Some(lyrics) = extract_from_dict(st.metadata()) {
            return Some(lyrics);
        }
    }

    None
}

pub(crate) fn extract_embedded_cover_art(path: &Path) -> Option<Vec<u8>> {
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

    None
}
