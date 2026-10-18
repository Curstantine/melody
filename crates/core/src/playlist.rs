//! M3U / M3U8 playlist parsing and serialization.
//!
//! Playlists are stored as plain `.m3u` / `.m3u8` files on disk and are the
//! single source of truth for their contents; the database only records a
//! reference (path + cached metadata). This module contains no database or UI
//! dependencies so it can be unit-tested in isolation.
//!
//! Supported syntax:
//! - Optional `#EXTM3U` header.
//! - Extended `#EXTINF:<seconds>,<title>` metadata lines preceding a path.
//! - `#` comment lines and blank lines.
//! - Relative paths resolved against the playlist file's directory.
//!
//! `M3U8` is UTF-8 by definition; legacy `M3U` files are decoded as UTF-8 when
//! valid and fall back to Windows-1252 (a superset of Latin-1) otherwise.

use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use crate::model::TrackRow;

#[derive(Error, Debug)]
pub enum PlaylistError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Playlist path has no parent directory: {0:?}")]
    NoParent(PathBuf),
}

/// A single resolved entry inside a playlist file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistEntry {
    /// The path exactly as written in the playlist file (may be relative).
    pub raw_path: String,
    /// Absolute path after resolving relative entries against the playlist dir.
    pub resolved_path: PathBuf,
    /// Title from `#EXTINF` when present.
    pub title: Option<String>,
    /// Duration in milliseconds from `#EXTINF` when present and non-negative.
    pub duration_ms: Option<u64>,
}

/// Parsed representation of a playlist file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPlaylist {
    /// Human-readable name derived from the file stem.
    pub name: String,
    pub entries: Vec<PlaylistEntry>,
    /// Whether an `#EXTM3U` header (extended playlist) was present.
    pub extended: bool,
}

/// Returns `true` when the path has an `.m3u` or `.m3u8` extension.
pub fn is_supported_playlist_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("m3u") | Some("m3u8")
    )
}

/// Derives a display name for a playlist from its file stem.
pub fn playlist_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Playlist".to_string())
}

/// Produces a filesystem-safe playlist file name (without extension).
pub fn sanitize_playlist_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "Playlist".to_string()
    } else {
        sanitized_truncate(trimmed, 120)
    }
}

fn sanitized_truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars()
        .take(max)
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Resolves a raw playlist path (absolute or relative) against `playlist_dir`.
pub fn resolve_entry_path(playlist_dir: &Path, raw: &str) -> PathBuf {
    let candidate = Path::new(raw);
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        playlist_dir.join(candidate)
    };
    normalize_path(&path)
}

/// Decodes playlist file bytes, tolerating a UTF-8 BOM and falling back to
/// Windows-1252 for legacy `M3U` files that are not valid UTF-8.
fn decode_bytes(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            cow.into_owned()
        }
    }
}

/// Parses an M3U/M3U8 playlist file from disk.
pub fn parse_m3u<P: AsRef<Path>>(path: P) -> Result<ParsedPlaylist, PlaylistError> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    Ok(parse_m3u_str(&decode_bytes(&bytes), path))
}

/// Parses playlist text. `origin` is used for the name and for resolving
/// relative entries; it does not need to exist on disk.
pub fn parse_m3u_str(content: &str, origin: &Path) -> ParsedPlaylist {
    let playlist_dir: PathBuf = origin
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut entries = Vec::new();
    let mut extended = false;
    let mut pending: Option<(Option<String>, Option<u64>)> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            pending = Some(parse_extinf(rest));
            continue;
        }
        if line.eq_ignore_ascii_case("#EXTM3U") {
            extended = true;
            continue;
        }
        if line.starts_with('#') {
            // Unknown directive (`#EXTALB`, `#PLAYLIST`, comments, ...).
            continue;
        }

        // Network streams are not supported by the local decoder; skip them.
        if is_remote_url(line) {
            pending = None;
            continue;
        }

        let (title, duration_ms) = pending.take().unwrap_or((None, None));
        entries.push(PlaylistEntry {
            raw_path: line.to_string(),
            resolved_path: resolve_entry_path(&playlist_dir, line),
            title,
            duration_ms,
        });
    }

    ParsedPlaylist {
        name: playlist_name_from_path(origin),
        entries,
        extended,
    }
}

fn parse_extinf(rest: &str) -> (Option<String>, Option<u64>) {
    // Format: <seconds>,<title>  (title may be omitted)
    let (secs_str, title) = match rest.split_once(',') {
        Some((s, t)) => (s, Some(t.trim().to_string())),
        None => (rest, None),
    };

    let duration_ms = secs_str
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|secs| *secs >= 0.0)
        .map(|secs| (secs * 1000.0).round() as u64);

    let title = title.filter(|t| !t.is_empty());
    (title, duration_ms)
}

fn is_remote_url(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.starts_with("rtsp://")
        || lower.starts_with("mms://")
}

/// Writes entries to an extended M3U8 (UTF-8) playlist file.
///
/// Paths are written using absolute paths. The parent directory is
/// created if it does not exist.
pub fn write_m3u<P: AsRef<Path>>(path: P, entries: &[PlaylistEntry]) -> Result<(), PlaylistError> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .ok_or_else(|| PlaylistError::NoParent(path.to_path_buf()))?;
    std::fs::create_dir_all(parent)?;

    let mut out = String::from("#EXTM3U\n");
    for entry in entries {
        let original = Path::new(&entry.raw_path);
        let target = if entry.resolved_path.as_os_str().is_empty() {
            original.to_path_buf()
        } else {
            entry.resolved_path.clone()
        };

        let target = if is_remote_url(&entry.raw_path) {
            PathBuf::from(&entry.raw_path)
        } else if target.is_absolute() {
            normalize_path(&target)
        } else {
            let resolved = resolve_entry_path(parent, &target.to_string_lossy());
            if resolved.is_absolute() {
                resolved
            } else {
                std::path::absolute(&resolved)
                    .map(|p| normalize_path(&p))
                    .unwrap_or(resolved)
            }
        };

        let written_str = target.to_string_lossy().to_string();

        let secs = entry
            .duration_ms
            .map(|ms| ms as f64 / 1000.0)
            .map(|s| {
                if s.fract() == 0.0 {
                    format!("{}", s as i64)
                } else {
                    format!("{:.3}", s)
                }
            })
            .unwrap_or_else(|| "-1".to_string());
        let title = entry
            .title
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()
            });
        out.push_str(&format!("#EXTINF:{},{}\n", secs, title));
        out.push_str(&written_str);
        out.push('\n');
    }

    std::fs::write(path, out)?;
    Ok(())
}

/// Appends tracks to a playlist file on disk (creating the file if it does not exist).
/// Returns the new total count of playlist entries.
pub fn append_tracks_to_m3u<P: AsRef<Path>>(
    path: P,
    tracks: &[TrackRow],
) -> Result<usize, PlaylistError> {
    let path = path.as_ref();
    let mut entries = if path.exists() {
        parse_m3u(path)?.entries
    } else {
        Vec::new()
    };

    for track in tracks {
        entries.push(PlaylistEntry {
            raw_path: track.file_path.clone(),
            resolved_path: PathBuf::from(&track.file_path),
            title: Some(track.title.clone()),
            duration_ms: Some(track.duration_ms),
        });
    }

    write_m3u(path, &entries)?;
    Ok(entries.len())
}

/// Normalizes path components lexically without touching the filesystem.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(
                    normalized.components().next_back(),
                    Some(Component::Normal(_))
                ) {
                    normalized.pop();
                } else if !matches!(
                    normalized.components().next_back(),
                    Some(Component::RootDir | Component::Prefix(_))
                ) {
                    normalized.push(component);
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extended_playlist() {
        let text = "#EXTM3U\n#EXTINF:123,Test Song\nmusic/song.mp3\n# comment\n#EXTINF:45.5,Other\n/tmp/other.flac\n";
        let parsed = parse_m3u_str(text, Path::new("/playlists/list.m3u8"));
        assert!(parsed.extended);
        assert_eq!(parsed.name, "list");
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].title.as_deref(), Some("Test Song"));
        assert_eq!(parsed.entries[0].duration_ms, Some(123_000));
        assert_eq!(
            parsed.entries[0].resolved_path,
            PathBuf::from("/playlists/music/song.mp3")
        );
        assert_eq!(parsed.entries[1].duration_ms, Some(45_500));
        assert_eq!(
            parsed.entries[1].resolved_path,
            PathBuf::from("/tmp/other.flac")
        );
    }

    #[test]
    fn skips_remote_and_comments() {
        let text = "#EXTM3U\nhttp://example.com/stream\n#EXTINF:1,x\n/song.mp3\n";
        let parsed = parse_m3u_str(text, Path::new("/p.m3u"));
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].resolved_path, PathBuf::from("/song.mp3"));
    }

    #[test]
    fn handles_extinf_without_title() {
        let text = "#EXTM3U\n#EXTINF:-1,\n/song.mp3\n";
        let parsed = parse_m3u_str(text, Path::new("/p.m3u"));
        assert_eq!(parsed.entries[0].duration_ms, None);
        assert_eq!(parsed.entries[0].title, None);
    }

    #[test]
    fn write_then_parse_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "melody_m3u_roundtrip_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let playlist_path = dir.join("roundtrip.m3u8");

        let track_path = dir.join("tracks").join("a.flac");
        std::fs::create_dir_all(track_path.parent().unwrap()).unwrap();
        std::fs::write(&track_path, b"x").unwrap();

        let entries = vec![PlaylistEntry {
            raw_path: track_path.to_string_lossy().to_string(),
            resolved_path: track_path.clone(),
            title: Some("A Track".to_string()),
            duration_ms: Some(90_000),
        }];
        write_m3u(&playlist_path, &entries).unwrap();

        let written = std::fs::read_to_string(&playlist_path).unwrap();
        assert!(written.starts_with("#EXTM3U"));
        assert!(written.contains(&track_path.to_string_lossy().to_string()));

        let parsed = parse_m3u(&playlist_path).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].resolved_path, track_path);
        assert_eq!(parsed.entries[0].title.as_deref(), Some("A Track"));
        assert_eq!(parsed.entries[0].duration_ms, Some(90_000));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_playlist_filename("My/List"), "My_List");
        assert_eq!(sanitize_playlist_filename("   "), "Playlist");
    }

    #[test]
    fn normalizes_paths_lexically() {
        assert_eq!(normalize_path(Path::new("/a/b/c")), PathBuf::from("/a/b/c"));
        assert_eq!(
            normalize_path(Path::new("/a/b/../c")),
            PathBuf::from("/a/c")
        );
        assert_eq!(
            normalize_path(Path::new("/a/./b/./c")),
            PathBuf::from("/a/b/c")
        );
        assert_eq!(normalize_path(Path::new("/../../a")), PathBuf::from("/a"));
        assert_eq!(normalize_path(Path::new("a/b/../../c")), PathBuf::from("c"));
        assert_eq!(
            normalize_path(Path::new("a/b/../../../c")),
            PathBuf::from("../c")
        );
    }

    #[test]
    fn resolves_relative_path_with_parent_traversal() {
        let resolved = resolve_entry_path(
            Path::new("/home/user/.local/share/melody/playlists"),
            "../../../../Music/song.mp3",
        );
        assert_eq!(resolved, PathBuf::from("/home/user/Music/song.mp3"));
    }

    #[test]
    fn appends_tracks_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "melody-test-append-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_playlist.m3u8");
        let track1 = TrackRow {
            id: 1,
            title: "First Song".to_string(),
            artist: "Artist 1".into(),
            album: "Album 1".into(),
            duration_ms: 180000,
            file_path: "/music/song1.mp3".to_string(),
            duration_label: "3:00".to_string(),
        };
        let track2 = TrackRow {
            id: 2,
            title: "Second Song".to_string(),
            artist: "Artist 2".into(),
            album: "Album 2".into(),
            duration_ms: 240000,
            file_path: "/music/song2.mp3".to_string(),
            duration_label: "4:00".to_string(),
        };

        // First append creates file with 1 track
        let count1 = append_tracks_to_m3u(&path, &[track1]).unwrap();
        assert_eq!(count1, 1);

        // Second append adds track2 so count is 2
        let count2 = append_tracks_to_m3u(&path, &[track2]).unwrap();
        assert_eq!(count2, 2);

        let parsed = parse_m3u(&path).unwrap();
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[0].title.as_deref(), Some("First Song"));
        assert_eq!(parsed.entries[1].title.as_deref(), Some("Second Song"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
