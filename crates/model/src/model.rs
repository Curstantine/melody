use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
    path::PathBuf,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::intern::InternedStr;

/// Inline, stack-allocated duration label (`"m:ss"` or `"h:mm:ss"`, up to 15 ASCII bytes).
///
/// Eliminates a 24-byte `String` header and separate heap allocation on every
/// [`TrackRow`] while remaining `Copy` and dereferencing directly to `&str`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DurationStr {
    buf: [u8; 15],
    len: u8,
}

impl Default for DurationStr {
    fn default() -> Self {
        Self::new()
    }
}

impl DurationStr {
    pub const fn new() -> Self {
        Self {
            buf: [0; 15],
            len: 0,
        }
    }

    /// Formats `duration_ms` as `m:ss` or `h:mm:ss` directly onto the stack with zero heap allocations.
    pub fn from_ms(duration_ms: u64) -> Self {
        let total_seconds = duration_ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = (total_seconds % 60) as u8;
        let hours = minutes / 60;

        let mut out = Self::new();
        if hours > 0 {
            out.push_u64(hours);
            out.push_byte(b':');
            let mins = (minutes % 60) as u8;
            out.push_two_digits(mins);
            out.push_byte(b':');
            out.push_two_digits(seconds);
        } else {
            out.push_u64(minutes);
            out.push_byte(b':');
            out.push_two_digits(seconds);
        }
        out
    }

    pub fn from_str_slice(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut copy_len = bytes.len().min(15);
        while copy_len > 0 && !s.is_char_boundary(copy_len) {
            copy_len -= 1;
        }
        let mut buf = [0u8; 15];
        buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
        Self {
            buf,
            len: copy_len as u8,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: only valid UTF-8 (or ASCII digits/colons) are written up to `self.len`.
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }

    #[inline]
    fn push_byte(&mut self, b: u8) {
        let idx = self.len as usize;
        if idx < 15 {
            self.buf[idx] = b;
            self.len += 1;
        }
    }

    #[inline]
    fn push_two_digits(&mut self, val: u8) {
        self.push_byte(b'0' + (val / 10));
        self.push_byte(b'0' + (val % 10));
    }

    #[inline]
    fn push_u64(&mut self, mut val: u64) {
        if val == 0 {
            self.push_byte(b'0');
            return;
        }
        let mut tmp = [0u8; 20];
        let mut i = 20;
        while val > 0 {
            i -= 1;
            tmp[i] = b'0' + (val % 10) as u8;
            val /= 10;
        }
        for &b in &tmp[i..] {
            self.push_byte(b);
        }
    }
}

impl Deref for DurationStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for DurationStr {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for DurationStr {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for DurationStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for DurationStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for DurationStr {
    fn from(s: &str) -> Self {
        Self::from_str_slice(s)
    }
}

impl From<String> for DurationStr {
    fn from(s: String) -> Self {
        Self::from_str_slice(&s)
    }
}

impl From<&String> for DurationStr {
    fn from(s: &String) -> Self {
        Self::from_str_slice(s)
    }
}

impl PartialEq<str> for DurationStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for DurationStr {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for DurationStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Serialize for DurationStr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DurationStr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <Cow<'de, str>>::deserialize(deserializer)?;
        Ok(Self::from_str_slice(&s))
    }
}

/// Memory-compact track path that interns the shared parent directory prefix
/// (`dir`, including its trailing separator) and stores only the file name in a
/// tightly-sized `Box<str>`.
///
/// In a 100k-track library organized into ~8,000 album directories, ~80% of
/// every file path string is the shared album directory prefix. Sharing `dir`
/// via [`InternedStr`] saves ~5–6 MB of duplicated path heap memory in large queues.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TrackPath {
    dir: InternedStr,
    file_name: Box<str>,
}

impl Default for TrackPath {
    fn default() -> Self {
        Self {
            dir: crate::intern::intern(""),
            file_name: Box::from(""),
        }
    }
}

impl TrackPath {
    pub fn new(path: &str) -> Self {
        if let Some(sep_idx) = path.rfind(['/', '\\']) {
            let (dir_part, file_part) = path.split_at(sep_idx + 1);
            Self {
                dir: crate::intern::intern(dir_part),
                file_name: Box::from(file_part),
            }
        } else {
            Self {
                dir: crate::intern::intern(""),
                file_name: Box::from(path),
            }
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dir.is_empty() && self.file_name.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dir.len() + self.file_name.len()
    }

    #[inline]
    pub fn dir(&self) -> &str {
        &self.dir
    }

    #[inline]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// Appends the full path directly into `out` without an intermediate allocation.
    #[inline]
    pub fn write_into(&self, out: &mut String) {
        out.push_str(&self.dir);
        out.push_str(&self.file_name);
    }

    /// Compares the full path against `other` without allocating a joined string.
    #[inline]
    pub fn eq_str(&self, other: &str) -> bool {
        other.len() == self.len()
            && other.starts_with(&*self.dir)
            && other[self.dir.len()..] == *self.file_name
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.to_string())
    }

    pub fn as_cow(&self) -> Cow<'_, str> {
        if self.dir.is_empty() {
            Cow::Borrowed(&self.file_name)
        } else {
            Cow::Owned(self.to_string())
        }
    }
}

impl fmt::Display for TrackPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::with_capacity(self.len());
        self.write_into(&mut s);
        f.write_str(&s)
    }
}

impl fmt::Debug for TrackPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}{}\"", self.dir, self.file_name)
    }
}

impl From<&str> for TrackPath {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TrackPath {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<&String> for TrackPath {
    fn from(s: &String) -> Self {
        Self::new(s)
    }
}

impl From<TrackPath> for String {
    fn from(p: TrackPath) -> Self {
        p.to_string()
    }
}

impl From<&TrackPath> for String {
    fn from(p: &TrackPath) -> Self {
        p.to_string()
    }
}

impl From<TrackPath> for PathBuf {
    fn from(p: TrackPath) -> Self {
        p.to_path_buf()
    }
}

impl From<&TrackPath> for PathBuf {
    fn from(p: &TrackPath) -> Self {
        p.to_path_buf()
    }
}

impl PartialEq<str> for TrackPath {
    fn eq(&self, other: &str) -> bool {
        self.eq_str(other)
    }
}

impl PartialEq<&str> for TrackPath {
    fn eq(&self, other: &&str) -> bool {
        self.eq_str(other)
    }
}

impl PartialEq<String> for TrackPath {
    fn eq(&self, other: &String) -> bool {
        self.eq_str(other)
    }
}

impl PartialEq<Cow<'_, str>> for TrackPath {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.eq_str(other)
    }
}

impl PartialEq<TrackPath> for Cow<'_, str> {
    fn eq(&self, other: &TrackPath) -> bool {
        other.eq_str(self)
    }
}

impl PartialEq<TrackPath> for String {
    fn eq(&self, other: &TrackPath) -> bool {
        other.eq_str(self)
    }
}

impl PartialEq<TrackPath> for str {
    fn eq(&self, other: &TrackPath) -> bool {
        other.eq_str(self)
    }
}

impl PartialEq<TrackPath> for &str {
    fn eq(&self, other: &TrackPath) -> bool {
        other.eq_str(self)
    }
}

impl Serialize for TrackPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TrackPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <Cow<'de, str>>::deserialize(deserializer)?;
        Ok(Self::new(&s))
    }
}

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

/// Formats `;`-separated multi-value metadata tags as `, `-separated values for display.
pub fn format_multi_value(raw: &str) -> std::borrow::Cow<'_, str> {
    if !raw.contains(';') {
        return std::borrow::Cow::Borrowed(raw);
    }
    let parts: Vec<&str> = raw
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    std::borrow::Cow::Owned(parts.join(", "))
}

pub fn format_duration(duration_ms: u64) -> String {
    DurationStr::from_ms(duration_ms).to_string()
}

pub fn format_duration_stack(duration_ms: u64) -> DurationStr {
    DurationStr::from_ms(duration_ms)
}

/// A narrow projection of a track used by the virtualized library list and playback queues.
///
/// Only the columns actually rendered in a list row are fetched. Combined with
/// the bounded sliding window in the UI, resident memory stays proportional to
/// the viewport size rather than the size of the library. `artist`/`album` are
/// interned to share repeated metadata strings, `file_path` interns its shared
/// parent directory prefix via [`TrackPath`], and `duration_label` lives inline
/// on the stack via [`DurationStr`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackRow {
    pub id: i64,
    pub title: String,
    pub artist: InternedStr,
    pub album: InternedStr,
    pub duration_ms: u64,
    pub file_path: TrackPath,
    /// Pre-rendered duration (`m:ss` / `h:mm:ss`) stored inline on the stack so
    /// list rendering and row construction perform zero heap allocations.
    #[serde(default)]
    pub duration_label: DurationStr,
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
            file_path: TrackPath::new(&t.file_path),
            duration_label: DurationStr::from_ms(t.duration_ms),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlbumSortField {
    #[default]
    Title,
    Artist,
    Year,
    TrackCount,
    Duration,
    DateAdded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArtistSortField {
    #[default]
    Artist,
    TrackCount,
    AlbumCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlaylistSortField {
    #[default]
    Name,
    TrackCount,
    DateAdded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewMode {
    #[default]
    List,
    Tile,
}

/// Queue and track repeat behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

impl RepeatMode {
    pub const ALL: [RepeatMode; 3] = [RepeatMode::Off, RepeatMode::All, RepeatMode::One];

    /// Cycles through `Off -> All -> One -> Off`.
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    /// Human-readable label for UI selectors and indicators.
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::All => "Repeat all",
            Self::One => "Repeat one",
        }
    }
}

impl std::fmt::Display for RepeatMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(500), "0:00");
        assert_eq!(format_duration(65000), "1:05");
        assert_eq!(format_duration(3665000), "1:01:05");

        let stack = DurationStr::from_ms(3665000);
        assert_eq!(stack.as_str(), "1:01:05");
        assert_eq!(&*stack, "1:01:05");
        assert_eq!(stack, "1:01:05");
    }

    #[test]
    fn test_track_path_interns_shared_directory() {
        let p1 = TrackPath::new("/music/Artist/Album/01 - Intro.flac");
        let p2 = TrackPath::new("/music/Artist/Album/02 - Finale.flac");
        assert!(std::sync::Arc::ptr_eq(&p1.dir, &p2.dir));
        assert_eq!(p1.dir(), "/music/Artist/Album/");
        assert_eq!(p1.file_name(), "01 - Intro.flac");
        assert_eq!(p1.to_string(), "/music/Artist/Album/01 - Intro.flac");
        assert!(p1.eq_str("/music/Artist/Album/01 - Intro.flac"));
        assert!(!p1.eq_str("/music/Artist/Album/02 - Finale.flac"));
    }

    #[test]
    fn test_repeat_mode_cycle() {
        assert_eq!(RepeatMode::Off.next(), RepeatMode::All);
        assert_eq!(RepeatMode::All.next(), RepeatMode::One);
        assert_eq!(RepeatMode::One.next(), RepeatMode::Off);
    }
}
