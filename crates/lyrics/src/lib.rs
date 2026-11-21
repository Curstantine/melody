use std::path::{Path, PathBuf};
use tracing::debug;

/// Finds a sidecar file in `track_path.parent()` matching `track_path.file_stem()` and `ext`
/// (case-insensitive).
fn find_sidecar_file(track_path: &Path, ext: &str) -> Option<PathBuf> {
    let direct = track_path.with_extension(ext);
    if direct.is_file() {
        return Some(direct);
    }

    let parent = track_path.parent()?;
    let stem = track_path.file_stem()?.to_str()?;
    let entries = std::fs::read_dir(parent).ok()?;

    for entry in entries.flatten() {
        let candidate = entry.path();
        if !candidate.is_file() {
            continue;
        }
        let Some(cand_ext) = candidate.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !cand_ext.eq_ignore_ascii_case(ext) {
            continue;
        }
        let Some(cand_stem) = candidate.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if cand_stem.eq_ignore_ascii_case(stem) {
            return Some(candidate);
        }
    }

    None
}

/// Reads and decodes a lyrics file, supporting UTF-8/UTF-16 BOMs and legacy codepages.
fn read_lyrics_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }

    let text = if let Some((encoding, _bom_len)) = encoding_rs::Encoding::for_bom(&bytes) {
        let (decoded, _) = encoding.decode_with_bom_removal(&bytes);
        decoded.trim().to_string()
    } else {
        metadata::sanitize_tag_bytes(&bytes)
    };

    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.trim().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Resolves local lyrics for `track_path` in priority order:
/// 1. Same-directory `.lrc` sidecar
/// 2. Embedded lyrics tag inside the audio file
/// 3. Same-directory `.txt` sidecar fallback
pub fn fetch_local_lyrics(track_path: &Path, artist: &str, title: &str) -> Option<String> {
    debug!(
        "Fetching local/embedded lyrics for {} - {} ({})",
        artist,
        title,
        track_path.display()
    );

    if let Some(lrc_path) = find_sidecar_file(track_path, "lrc") {
        if let Some(lyrics) = read_lyrics_file(&lrc_path) {
            return Some(lyrics);
        }
    }

    if let Some(lyrics) = metadata::extract_lyrics(track_path) {
        return Some(lyrics);
    }

    if let Some(txt_path) = find_sidecar_file(track_path, "txt") {
        if let Some(lyrics) = read_lyrics_file(&txt_path) {
            return Some(lyrics);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("melody_lyrics_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn test_loads_same_dir_lrc_and_prefers_over_txt() {
        let dir = temp_test_dir("lrc_over_txt");
        let audio = dir.join("01 - Neon.flac");
        let lrc = dir.join("01 - Neon.lrc");
        let txt = dir.join("01 - Neon.txt");

        std::fs::write(&audio, b"not-a-real-flac").unwrap();
        std::fs::write(&lrc, "[00:01.00] Hello LRC\r\n[00:05.00] Line 2\r\n").unwrap();
        std::fs::write(&txt, "Plain text fallback").unwrap();

        let res = fetch_local_lyrics(&audio, "Artist", "Neon");
        assert_eq!(
            res,
            Some("[00:01.00] Hello LRC\n[00:05.00] Line 2".to_string())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_case_insensitive_lrc_and_utf8_bom() {
        let dir = temp_test_dir("case_bom");
        let audio = dir.join("Track.mp3");
        let lrc = dir.join("track.LRC");

        std::fs::write(&audio, b"not-a-real-mp3").unwrap();
        let mut content = vec![0xEF, 0xBB, 0xBF];
        content.extend_from_slice(b"[00:00.00] BOM line");
        std::fs::write(&lrc, content).unwrap();

        let res = fetch_local_lyrics(&audio, "Artist", "Track");
        assert_eq!(res, Some("[00:00.00] BOM line".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_txt_fallback_and_missing() {
        let dir = temp_test_dir("txt_fallback");
        let audio = dir.join("Ballad.ogg");
        let txt = dir.join("Ballad.txt");

        std::fs::write(&audio, b"not-a-real-ogg").unwrap();
        std::fs::write(&txt, "First line\nSecond line").unwrap();

        let res = fetch_local_lyrics(&audio, "Artist", "Ballad");
        assert_eq!(res, Some("First line\nSecond line".to_string()));

        let missing = dir.join("Missing.mp3");
        assert_eq!(fetch_local_lyrics(&missing, "Artist", "Missing"), None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
