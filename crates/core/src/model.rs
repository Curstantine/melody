use serde::{Deserialize, Serialize};

use crate::intern::InternedStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album_artist: Option<String>,
    pub album: String,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub track_num: Option<u32>,
    pub disc_num: Option<u32>,
    pub year: Option<u32>,
    pub composer: Option<String>,
    pub label: Option<String>,
    pub musicbrainz_track_id: Option<String>,
    pub musicbrainz_artist_id: Option<String>,
    pub musicbrainz_album_id: Option<String>,
    pub added_timestamp: i64,
}

impl Track {
    pub fn formatted_duration(&self) -> String {
        format_duration(self.duration_ms)
    }

    pub fn display_artist(&self) -> &str {
        if !self.artist.is_empty() {
            &self.artist
        } else if let Some(aa) = self.album_artist.as_deref() {
            aa
        } else {
            "Unknown Artist"
        }
    }
}

pub fn format_duration(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let hours = minutes / 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes % 60, seconds)
    } else {
        format!("{}:{:02}", minutes, seconds)
    }
}

/// Lightweight in-memory index struct to keep memory footprint minimal (~30-50MB)
/// even for 100k+ library entries. Used for shuffle, totals, and ID lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackIdHeader {
    pub id: i64,
    pub duration_ms: u32,
}

/// A narrow projection of a track used by the virtualized library list.
///
/// Only the columns actually rendered in a list row are fetched. Combined with
/// the bounded sliding window in the UI, resident memory stays proportional to
/// the viewport size rather than the size of the library. `artist`/`album` are
/// interned to share the repeated metadata strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackRow {
    pub id: i64,
    pub title: String,
    pub artist: InternedStr,
    pub album: InternedStr,
    pub duration_ms: u64,
    pub file_path: String,
    /// Pre-rendered duration (`m:ss` / `h:mm:ss`) so list rendering performs no
    /// `format!` allocation on every frame.
    #[serde(default)]
    pub duration_label: String,
    #[serde(default)]
    pub year: Option<u32>,
    #[serde(default)]
    pub track_num: Option<u32>,
    #[serde(default)]
    pub disc_num: Option<u32>,
    #[serde(default)]
    pub album_artist: Option<InternedStr>,
    #[serde(default)]
    pub genre: Option<InternedStr>,
}

impl TrackRow {
    pub fn formatted_duration(&self) -> &str {
        &self.duration_label
    }

    pub fn display_artist(&self) -> &str {
        if self.artist.is_empty() {
            "Unknown Artist"
        } else {
            &self.artist
        }
    }
}

impl From<&Track> for TrackRow {
    fn from(t: &Track) -> Self {
        TrackRow {
            id: t.id,
            title: t.title.clone(),
            artist: crate::intern::intern(t.display_artist()),
            album: crate::intern::intern(t.album.as_str()),
            duration_ms: t.duration_ms,
            file_path: t.file_path.clone(),
            duration_label: t.formatted_duration(),
            year: t.year,
            track_num: t.track_num,
            disc_num: t.disc_num,
            album_artist: t.album_artist.as_deref().map(crate::intern::intern),
            genre: t.genre.as_deref().map(crate::intern::intern),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NewTrack {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album_artist: Option<String>,
    pub album: String,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub track_num: Option<u32>,
    pub disc_num: Option<u32>,
    pub year: Option<u32>,
    pub composer: Option<String>,
    pub label: Option<String>,
    pub musicbrainz_track_id: Option<String>,
    pub musicbrainz_artist_id: Option<String>,
    pub musicbrainz_album_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumSummary {
    pub album: InternedStr,
    pub artist: InternedStr,
    pub track_count: usize,
    pub total_duration_ms: u64,
    pub year: Option<u32>,
    pub cover_track_id: Option<i64>,
    pub added_timestamp: i64,
    /// Pre-rendered display strings so catalog rendering allocates nothing per frame.
    #[serde(default)]
    pub year_label: String,
    #[serde(default)]
    pub tracks_label: String,
    #[serde(default)]
    pub duration_label: String,
}

/// A referenced M3U/M3U8 playlist file.
///
/// The playlist's contents live in the file itself; this summary only carries
/// the reference and cached metadata used to render the catalog without
/// re-reading every playlist from disk each frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub file_path: String,
    pub track_count: usize,
    pub created_timestamp: i64,
    pub modified_timestamp: i64,
    /// True when the file lives inside Melody's managed playlists directory.
    #[serde(default)]
    pub managed: bool,
    /// Pre-rendered "N tracks" label for allocation-free rendering.
    #[serde(default)]
    pub tracks_label: String,
}

/// Persisted metadata for a single playback queue.
///
/// Queue membership itself lives in `playback_queue_tracks` (ordered track
/// ids); this record only carries the descriptive/state fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueRecord {
    pub id: i64,
    pub name: String,
    pub current: i64,
    /// One of `manual`, `album`, `artist`, `search`, `playlist`, `all`.
    pub source_kind: String,
    /// Scope value the queue was built from (album name, query, …).
    pub source_value: Option<String>,
    /// Path of the playlist file this queue was created from, when applicable.
    pub source_playlist: Option<String>,
    pub track_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistSummary {
    pub artist: InternedStr,
    pub track_count: usize,
    pub album_count: usize,
    /// Pre-rendered "N albums • M tracks" label for allocation-free rendering.
    #[serde(default)]
    pub stats_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortField {
    #[default]
    Title,
    Artist,
    Album,
    Duration,
    TrackNum,
    AddedDate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}
