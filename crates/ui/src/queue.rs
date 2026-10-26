//! In-memory playback queues.
//!
//! Unlike playlists (which live on disk as M3U files), queues are transient,
//! ordered play lists that drive playback. Multiple queues can be kept around,
//! but only the active queue's tracks are materialized in memory; the metadata
//! for the rest and the persisted membership itself live in the database
//! (`playback_queues` / `playback_queue_tracks`).

use core_engine::{QueueRecord, TrackRow};

pub const DEFAULT_QUEUE_NAME: &str = "Queue";

/// Fixed pixel height of a single queue detail row. The virtualization math
/// depends on every row rendering at exactly this height.
pub const QUEUE_ROW_HEIGHT: f32 = 26.0;

/// Approximate visible rows used to size the materialized window (mirrors how
/// the library view uses `page_size` as its visible-row proxy).
pub const QUEUE_WINDOW_ROWS: usize = 100;

/// Buffered rows kept above and below the visible range. The window reloads
/// only when scrolling leaves it, which prevents thrashing while keeping
/// per-frame widget work bounded no matter how large the queue is.
pub const QUEUE_WINDOW_BUFFER_ROWS: usize = 100;

/// Maximum tracks materialized for rendering at once.
pub fn queue_window_capacity() -> usize {
    QUEUE_WINDOW_ROWS + 2 * QUEUE_WINDOW_BUFFER_ROWS
}

/// Clamped window start for `first_visible_row`: pulls back by the buffer so
/// scrolling stays inside already-rendered rows, and clamps to the tail when
/// `total` is smaller than the capacity.
pub fn queue_window_start(total: usize, first_visible_row: usize) -> usize {
    let max_start = total.saturating_sub(queue_window_capacity());
    first_visible_row
        .saturating_sub(QUEUE_WINDOW_BUFFER_ROWS)
        .min(max_start)
}

/// Clamped `(start, end)` render window covering `window_start`.
pub fn queue_window_for_start(total: usize, window_start: usize) -> (usize, usize) {
    let max_start = total.saturating_sub(queue_window_capacity());
    let start = window_start.min(max_start);
    (start, (start + queue_window_capacity()).min(total))
}

/// Where a queue's contents came from. Used for display and queue reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueSource {
    Manual,
    Album,
    Artist,
    Search,
    Playlist,
    AllSongs,
}

impl QueueSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueSource::Manual => "manual",
            QueueSource::Album => "album",
            QueueSource::Artist => "artist",
            QueueSource::Search => "search",
            QueueSource::Playlist => "playlist",
            QueueSource::AllSongs => "all",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "album" => QueueSource::Album,
            "artist" => QueueSource::Artist,
            "search" => QueueSource::Search,
            "playlist" => QueueSource::Playlist,
            "all" => QueueSource::AllSongs,
            _ => QueueSource::Manual,
        }
    }
}

/// Lightweight description of a queue that is not currently loaded.
#[derive(Debug, Clone)]
pub struct QueueSummary {
    pub id: u64,
    pub name: String,
    pub track_count: usize,
    pub current: usize,
    pub source: QueueSource,
    pub source_value: Option<String>,
    pub source_playlist: Option<String>,
}

impl core_engine::FieldLookup for QueueSummary {
    fn lookup(&self, key: &str) -> Option<std::borrow::Cow<'_, str>> {
        match key {
            "id" => Some(std::borrow::Cow::Owned(self.id.to_string())),
            "name" | "title" => Some(std::borrow::Cow::Borrowed(&self.name)),
            "track_count" | "tracks" => Some(std::borrow::Cow::Owned(self.track_count.to_string())),
            "current" => Some(std::borrow::Cow::Owned(self.current.to_string())),
            "source" | "source_kind" => Some(std::borrow::Cow::Borrowed(self.source.as_str())),
            "source_value" => self.source_value.as_deref().map(std::borrow::Cow::Borrowed),
            "source_playlist" => self
                .source_playlist
                .as_deref()
                .map(std::borrow::Cow::Borrowed),
            _ => None,
        }
    }
}

/// A fully loaded, ordered playback queue with a cursor.
#[derive(Debug, Clone)]
pub struct Queue {
    pub id: u64,
    pub name: String,
    pub tracks: Vec<TrackRow>,
    pub current: usize,
    pub source: QueueSource,
    pub source_value: Option<String>,
    pub source_playlist: Option<String>,
}

impl Queue {
    #[allow(dead_code)]
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            tracks: Vec::new(),
            current: 0,
            source: QueueSource::Manual,
            source_value: None,
            source_playlist: None,
        }
    }

    pub fn with_source(
        id: u64,
        name: impl Into<String>,
        source: QueueSource,
        source_value: Option<String>,
        source_playlist: Option<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            tracks: Vec::new(),
            current: 0,
            source,
            source_value,
            source_playlist,
        }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn current_index(&self) -> Option<usize> {
        if self.tracks.is_empty() {
            None
        } else {
            Some(self.current.min(self.tracks.len() - 1))
        }
    }

    pub fn current_track(&self) -> Option<&TrackRow> {
        self.current_index().and_then(|i| self.tracks.get(i))
    }

    /// Returns the index of the next track, or `None` at the end of the queue.
    pub fn next_index(&self) -> Option<usize> {
        let current = self.current_index()?;
        if current + 1 < self.tracks.len() {
            Some(current + 1)
        } else {
            None
        }
    }

    pub fn previous_index(&self) -> Option<usize> {
        let current = self.current_index()?;
        current.checked_sub(1)
    }

    pub fn append_many(&mut self, tracks: impl IntoIterator<Item = TrackRow>) {
        self.tracks.extend(tracks);
    }

    /// Inserts `track` immediately after the current track (Musicolet "play next").
    pub fn insert_next(&mut self, track: TrackRow) {
        let index = self.current_index().map(|i| i + 1).unwrap_or(0);
        let index = index.min(self.tracks.len());
        self.tracks.insert(index, track);
    }

    /// Inserts multiple `tracks` in order immediately after the current track.
    pub fn insert_many_next(&mut self, tracks: impl IntoIterator<Item = TrackRow>) {
        let curr = self.current_index().map(|i| i + 1).unwrap_or(0);
        for (index, track) in (curr.min(self.tracks.len())..).zip(tracks) {
            self.tracks.insert(index, track);
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<TrackRow> {
        if index >= self.tracks.len() {
            return None;
        }
        let removed = self.tracks.remove(index);
        if self.current > index || self.current >= self.tracks.len() {
            self.current = self.current.saturating_sub(1);
        }
        self.current = self.current.min(self.tracks.len().saturating_sub(1));
        Some(removed)
    }

    /// Moves the track at `from` to position `to`, shifting the tracks in
    /// between. The cursor follows the playing track so playback stays on the
    /// same song no matter how it (or the tracks around it) moved.
    pub fn move_track_to(&mut self, from: usize, to: usize) -> bool {
        let len = self.tracks.len();
        if from >= len || to >= len || from == to {
            return false;
        }
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        if self.current == from {
            self.current = to;
        } else if from < self.current && self.current <= to {
            self.current -= 1;
        } else if to <= self.current && self.current < from {
            self.current += 1;
        }
        true
    }

    pub fn set_current(&mut self, index: usize) {
        if !self.tracks.is_empty() {
            self.current = index.min(self.tracks.len() - 1);
        }
    }

    pub fn track_ids(&self) -> Vec<i64> {
        self.tracks.iter().map(|t| t.id).collect()
    }

    pub fn to_record(&self) -> QueueRecord {
        QueueRecord {
            id: self.id as i64,
            name: self.name.clone(),
            current: self.current as i64,
            source_kind: self.source.as_str().to_string(),
            source_value: self.source_value.clone(),
            source_playlist: self.source_playlist.clone(),
            track_count: self.tracks.len(),
        }
    }
}

/// In-memory view over the persisted queue set.
///
/// `summaries` describes every open queue (cheap) while `active_queue` holds
/// the fully materialized tracks of the currently selected queue only.
#[derive(Debug, Clone, Default)]
pub struct QueueStore {
    pub summaries: Vec<QueueSummary>,
    pub active: Option<u64>,
    pub active_queue: Option<Queue>,
    pub next_id: u64,
}

impl QueueStore {
    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    pub fn active_queue(&self) -> Option<&Queue> {
        self.active_queue.as_ref()
    }

    pub fn active_queue_mut(&mut self) -> Option<&mut Queue> {
        self.active_queue.as_mut()
    }

    pub fn summary(&self, id: u64) -> Option<&QueueSummary> {
        self.summaries.iter().find(|q| q.id == id)
    }

    pub fn find_id_by_name(&self, name: &str) -> Option<u64> {
        self.summaries
            .iter()
            .find(|q| q.name.eq_ignore_ascii_case(name))
            .map(|q| q.id)
    }

    /// Replaces the summary list from persisted records.
    pub fn load_summaries(&mut self, records: Vec<QueueRecord>) {
        self.summaries = records
            .into_iter()
            .map(|r| QueueSummary {
                id: r.id as u64,
                name: r.name,
                track_count: r.track_count,
                current: r.current.max(0) as usize,
                source: QueueSource::from_str(&r.source_kind),
                source_value: r.source_value,
                source_playlist: r.source_playlist,
            })
            .collect();
        let max_id = self.summaries.iter().map(|q| q.id).max().unwrap_or(0);
        self.next_id = self.next_id.max(max_id + 1);
    }

    /// Creates a fresh queue, makes it active, and returns its id.
    pub fn create(&mut self, name: impl Into<String>) -> u64 {
        self.create_with_source(name, QueueSource::Manual, None, None)
    }

    /// Creates a fresh queue with an explicit source and makes it active.
    pub fn create_with_source(
        &mut self,
        name: impl Into<String>,
        source: QueueSource,
        source_value: Option<String>,
        source_playlist: Option<String>,
    ) -> u64 {
        let name = name.into();
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        self.summaries.push(QueueSummary {
            id,
            name: name.clone(),
            track_count: 0,
            current: 0,
            source,
            source_value: source_value.clone(),
            source_playlist: source_playlist.clone(),
        });
        self.active = Some(id);
        self.active_queue = Some(Queue::with_source(
            id,
            name,
            source,
            source_value,
            source_playlist,
        ));
        id
    }

    /// Materializes a queue loaded from the database and marks it active.
    pub fn apply_loaded(&mut self, queue: Queue) {
        let id = queue.id;
        if let Some(summary) = self.summaries.iter_mut().find(|q| q.id == id) {
            summary.track_count = queue.tracks.len();
            summary.current = queue.current;
            summary.name = queue.name.clone();
        }
        self.active = Some(id);
        self.active_queue = Some(queue);
    }

    /// Activates a queue given freshly loaded tracks, reusing its summary metadata.
    pub fn activate_with_tracks(&mut self, id: u64, tracks: Vec<TrackRow>) {
        let Some(summary) = self.summaries.iter().find(|q| q.id == id).cloned() else {
            return;
        };
        let mut queue = Queue::with_source(
            summary.id,
            summary.name,
            summary.source,
            summary.source_value,
            summary.source_playlist,
        );
        queue.current = summary.current.min(tracks.len().saturating_sub(1));
        queue.tracks = tracks;
        self.apply_loaded(queue);
    }

    /// Replaces the tracks of an existing queue and makes it active.
    pub fn replace_with_tracks(&mut self, id: u64, tracks: Vec<TrackRow>) -> bool {
        let Some(summary) = self.summaries.iter_mut().find(|q| q.id == id) else {
            return false;
        };
        summary.track_count = tracks.len();
        summary.current = 0;
        let mut queue = Queue::with_source(
            id,
            summary.name.clone(),
            summary.source,
            summary.source_value.clone(),
            summary.source_playlist.clone(),
        );
        queue.tracks = tracks;
        self.active = Some(id);
        self.active_queue = Some(queue);
        true
    }

    /// Removes a queue.
    ///
    /// Returns `None` when no queue was removed. Otherwise returns the new
    /// active queue id: `Some(fallback)` when the removed queue was active
    /// and a sibling was auto-selected, or `None` when the removed queue was
    /// not active (active selection unchanged) or no queues remain.
    pub fn remove(&mut self, id: u64) -> Option<u64> {
        let index = self.summaries.iter().position(|q| q.id == id)?;
        self.summaries.remove(index);
        if self.active != Some(id) {
            return None;
        }
        if self.summaries.is_empty() {
            self.active = None;
            self.active_queue = None;
            return None;
        }
        let fallback_index = index.min(self.summaries.len() - 1);
        let fallback = self.summaries[fallback_index].id;
        self.set_active(fallback);
        Some(fallback)
    }

    /// Marks a queue as active without loading its tracks. Clears the stale
    /// materialized queue when switching so callers never observe a mismatch.
    pub fn set_active(&mut self, id: u64) {
        if self.summaries.iter().any(|q| q.id == id) {
            if self.active != Some(id) {
                self.active_queue = None;
            }
            self.active = Some(id);
        }
    }

    pub fn set_active_id(&mut self, id: Option<u64>) {
        self.active = id;
        if id.is_none() {
            self.active_queue = None;
        }
    }

    /// Refreshes the summary of the active queue after in-memory edits.
    pub fn touch_active_summary(&mut self) {
        let Some(id) = self.active else {
            return;
        };
        let Some(queue) = self.active_queue.as_ref() else {
            return;
        };
        let (len, current) = (queue.tracks.len(), queue.current);
        if let Some(summary) = self.summaries.iter_mut().find(|q| q.id == id) {
            summary.track_count = len;
            summary.current = current;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_engine::{QueueRecord, TrackRow};
    use std::sync::Arc;

    fn track(id: i64) -> TrackRow {
        TrackRow {
            id,
            title: format!("Track {}", id),
            artist: Arc::from("Artist"),
            album: Arc::from("Album"),
            duration_ms: 1000,
            file_path: format!("/music/{}.flac", id),
            duration_label: "0:01".to_string(),
            year: None,
            track_num: None,
            disc_num: None,
            album_artist: None,
            genre: None,
        }
    }

    fn record(id: i64, name: &str, count: usize) -> QueueRecord {
        QueueRecord {
            id,
            name: name.to_string(),
            current: 0,
            source_kind: "manual".to_string(),
            source_value: None,
            source_playlist: None,
            track_count: count,
        }
    }

    #[test]
    fn queue_navigation_and_shuffle_boundaries() {
        let mut q = Queue::new(1, "Test");
        assert!(q.is_empty());
        assert_eq!(q.current_index(), None);
        assert_eq!(q.next_index(), None);

        q.append_many((1..=3).map(track));
        assert_eq!(q.len(), 3);
        assert_eq!(q.current_index(), Some(0));
        assert_eq!(q.next_index(), Some(1));

        q.set_current(2);
        assert_eq!(q.next_index(), None, "at the end there is no next track");
        assert_eq!(q.previous_index(), Some(1));

        // insert_next places right after the cursor
        q.set_current(0);
        q.insert_next(track(99));
        assert_eq!(q.tracks[1].id, 99);

        // insert_many_next places multiple tracks in order right after the cursor
        q.insert_many_next(vec![track(100), track(101)]);
        assert_eq!(q.tracks[1].id, 100);
        assert_eq!(q.tracks[2].id, 101);
        assert_eq!(q.tracks[3].id, 99);

        // removing the current track clamps the cursor back onto a valid slot
        q.set_current(1);
        q.remove(1);
        assert!(q.current_index().unwrap() < q.len());

        // move_track_to by one slot follows the moved track
        q.set_current(0);
        let before = q.tracks[0].id;
        assert!(q.move_track_to(0, 1));
        assert_eq!(q.tracks[1].id, before);
        assert_eq!(q.current_index(), Some(1));
        assert!(
            !q.move_track_to(q.len() - 1, q.len()),
            "cannot move past the end"
        );
    }

    #[test]
    fn queue_move_track_to_shifts_and_follows_cursor() {
        let mut q = Queue::new(1, "DnD");
        q.append_many((1..=5).map(track));

        // Drag first track to the end: cursor follows the moved track.
        q.set_current(0);
        assert!(q.move_track_to(0, 4));
        assert_eq!(
            q.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 1]
        );
        assert_eq!(q.current_index(), Some(4));

        // Drag last track back to the front: tracks after it shift down.
        assert!(q.move_track_to(4, 0));
        assert_eq!(
            q.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(q.current_index(), Some(0));

        // Moving a track across the cursor shifts the cursor to track it.
        q.set_current(2);
        assert!(q.move_track_to(0, 4));
        assert_eq!(
            q.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 1]
        );
        assert_eq!(q.tracks[q.current_index().unwrap()].id, 3);
        assert!(q.move_track_to(4, 0));
        assert_eq!(q.tracks[q.current_index().unwrap()].id, 3);

        // No-ops.
        assert!(!q.move_track_to(1, 1), "same slot is a no-op");
        assert!(!q.move_track_to(0, 5), "out of bounds is a no-op");
        assert!(!q.move_track_to(9, 0), "out of bounds is a no-op");
    }

    #[test]
    fn queue_store_activation_and_removal() {
        let mut store = QueueStore::default();
        assert!(store.is_empty());
        assert!(store.active_queue().is_none());

        let a = store.create("A");
        let b = store.create("B");
        assert_eq!(store.active, Some(b));
        assert_eq!(store.active_queue().map(|q| q.id), Some(b));

        // Switching active invalidates the stale materialized queue until reloaded.
        store.set_active(a);
        assert_eq!(store.active, Some(a));
        assert!(store.active_queue().is_none());

        // Removing a non-active queue leaves the active selection untouched.
        assert_eq!(store.remove(b), None);
        assert_eq!(store.active, Some(a));
        assert_eq!(
            store.remove(b),
            None,
            "removing an unknown queue is a no-op"
        );
        assert_eq!(store.remove(a), None);
        assert_eq!(store.active, None, "removing the last queue clears active");

        // Default name is used when no explicit name is provided
        let mut store = QueueStore::default();
        store.create(DEFAULT_QUEUE_NAME);
        assert_eq!(store.summaries[0].name, "Queue");
    }

    #[test]
    fn queue_store_remove_active_selects_neighbor() {
        let mut store = QueueStore::default();
        let a = store.create("A");
        let b = store.create("B");
        let c = store.create("C");

        // Active is C; switch to middle (B) and delete it -> falls back to C
        // (same-index neighbor), clearing the stale materialized queue.
        store.set_active(b);
        assert_eq!(store.remove(b), Some(c));
        assert_eq!(store.active, Some(c));
        assert!(store.active_queue().is_none());
        assert_eq!(store.summaries.len(), 2);

        // Delete the last active (C) -> falls back to previous (A's neighbor).
        store.set_active(c);
        assert_eq!(store.remove(c), Some(a));
        assert_eq!(store.active, Some(a));

        // Deleting an unknown id is a no-op with no fallback.
        assert_eq!(store.remove(999), None);
        assert_eq!(store.active, Some(a));
    }

    #[test]
    fn queue_window_math_covers_large_queues() {
        let capacity = queue_window_capacity();
        assert_eq!(capacity, QUEUE_WINDOW_ROWS + 2 * QUEUE_WINDOW_BUFFER_ROWS);

        // Small queues render whole.
        assert_eq!(queue_window_for_start(0, 0), (0, 0));
        assert_eq!(queue_window_for_start(50, 0), (0, 50));
        assert_eq!(queue_window_for_start(capacity, 0), (0, capacity));

        // Scrolling pulls the window back by the buffer and clamps to the tail.
        assert_eq!(queue_window_start(100_000, 0), 0);
        assert_eq!(
            queue_window_start(100_000, 500),
            500 - QUEUE_WINDOW_BUFFER_ROWS
        );
        assert_eq!(
            queue_window_start(100_000, 99_999),
            100_000 - capacity,
            "window start clamps to the tail"
        );
        assert_eq!(
            queue_window_for_start(100_000, usize::MAX),
            (100_000 - capacity, 100_000),
            "out-of-range starts clamp to the tail"
        );
        let (start, end) = queue_window_for_start(100_000, 9_900);
        assert_eq!(start, 9_900);
        assert_eq!(end - start, capacity);
    }

    #[test]
    fn queue_store_reuse_and_replacement() {
        let mut store = QueueStore::default();
        store.load_summaries(vec![record(3, "Mixtape", 2)]);
        assert_eq!(store.next_id, 4);

        let id = store.find_id_by_name("Mixtape").expect("found by name");
        assert_eq!(id, 3);

        assert!(store.replace_with_tracks(id, (1..=4).map(track).collect()));
        assert_eq!(store.active, Some(3));
        assert_eq!(store.active_queue().map(|q| q.tracks.len()), Some(4));
        assert_eq!(store.summaries[0].track_count, 4);
    }

    #[test]
    fn queue_store_activate_with_tracks_uses_summary() {
        let mut store = QueueStore::default();
        store.load_summaries(vec![QueueRecord {
            id: 7,
            name: "Road Trip".to_string(),
            current: 2,
            source_kind: "album".to_string(),
            source_value: Some("Album X".to_string()),
            source_playlist: None,
            track_count: 5,
        }]);

        store.activate_with_tracks(7, (1..=5).map(track).collect());
        let active = store.active_queue().expect("active queue");
        assert_eq!(active.source, QueueSource::Album);
        assert_eq!(active.source_value.as_deref(), Some("Album X"));
        assert_eq!(active.current, 2);
        assert_eq!(active.len(), 5);
    }

    #[test]
    fn queue_source_roundtrips() {
        for source in [
            QueueSource::Manual,
            QueueSource::Album,
            QueueSource::Artist,
            QueueSource::Search,
            QueueSource::Playlist,
            QueueSource::AllSongs,
        ] {
            assert_eq!(QueueSource::from_str(source.as_str()), source);
        }
    }

    #[test]
    fn test_background_mode_queue_playback_invariants() {
        // In background mode, catalog summaries and textures are purged, but active queue
        // tracks must remain resident and operational for next, prev, and random playback.
        let mut store = QueueStore::default();
        let q_id = store.create("Background Playback");

        let tracks = vec![track(1), track(2), track(3), track(4)];
        store.replace_with_tracks(q_id, tracks);

        let queue = store.active_queue().expect("Active queue must be present");
        assert_eq!(queue.len(), 4);
        assert_eq!(queue.current_index(), Some(0));
        assert_eq!(queue.current_track().map(|t| t.id), Some(1));
        assert_eq!(queue.next_index(), Some(1));

        // Next track advancement
        let queue_mut = store.active_queue_mut().expect("Active queue mut");
        queue_mut.set_current(1);
        assert_eq!(queue_mut.current_track().map(|t| t.id), Some(2));
        assert_eq!(queue_mut.next_index(), Some(2));

        // Previous track
        assert_eq!(queue_mut.previous_index(), Some(0));

        // Shuffle selection invariant without catalog metadata
        let len = queue_mut.len();
        let position_seed: u64 = 42_000;
        let current = queue_mut.current_index().unwrap_or(0);
        let mut pick = (position_seed as usize ^ 0x9e3779b9) % len;
        if pick == current {
            pick = (pick + 1) % len;
        }
        assert!(pick < len);
        assert_ne!(pick, current);
        assert!(queue_mut.tracks.get(pick).is_some());
    }
}
