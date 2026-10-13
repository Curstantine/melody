//! In-memory playback queues.
//!
//! Unlike playlists (which live on disk as M3U files), queues are transient,
//! ordered play lists that drive playback. Multiple queues can be open at once
//! and the whole set is persisted to `queues.json` so a session can be resumed.

use std::path::Path;

use core_engine::TrackRow;
use serde::{Deserialize, Serialize};

pub const DEFAULT_QUEUE_NAME: &str = "Queue";

/// A single ordered playback queue with a cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Queue {
    pub id: u64,
    pub name: String,
    pub tracks: Vec<TrackRow>,
    #[serde(default)]
    pub current: usize,
    /// Path of the playlist file this queue was created from, when applicable.
    #[serde(default)]
    pub source_playlist: Option<String>,
}

impl Queue {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            tracks: Vec::new(),
            current: 0,
            source_playlist: None,
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

    /// Moves a track one slot up or down.
    pub fn move_track(&mut self, index: usize, up: bool) -> bool {
        if index >= self.tracks.len() {
            return false;
        }
        let target = if up {
            if index == 0 {
                return false;
            }
            index - 1
        } else {
            if index + 1 >= self.tracks.len() {
                return false;
            }
            index + 1
        };
        self.tracks.swap(index, target);
        if self.current == index {
            self.current = target;
        } else if self.current == target {
            self.current = index;
        }
        true
    }

    pub fn set_current(&mut self, index: usize) {
        if !self.tracks.is_empty() {
            self.current = index.min(self.tracks.len() - 1);
        }
    }
}

/// Serializable container for every open queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStore {
    #[serde(default)]
    pub queues: Vec<Queue>,
    #[serde(default)]
    pub active: Option<u64>,
    #[serde(default)]
    pub next_id: u64,
}

impl QueueStore {
    /// Loads the persisted queue store, falling back to an empty store.
    pub fn load(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut store: Self = serde_json::from_str(&content).unwrap_or_default();
        store.normalize();
        store
    }

    /// Persists the queue store, creating parent directories as needed.
    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    fn normalize(&mut self) {
        let max_id = self.queues.iter().map(|q| q.id).max().unwrap_or(0);
        self.next_id = self.next_id.max(max_id + 1);
        if self
            .active
            .is_some_and(|id| !self.queues.iter().any(|q| q.id == id))
        {
            self.active = self.queues.first().map(|q| q.id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queues.is_empty()
    }

    pub fn active_queue(&self) -> Option<&Queue> {
        let id = self.active?;
        self.queues.iter().find(|q| q.id == id)
    }

    pub fn active_queue_mut(&mut self) -> Option<&mut Queue> {
        let id = self.active?;
        self.queues.iter_mut().find(|q| q.id == id)
    }

    /// Creates a fresh queue, makes it active, and returns its id.
    pub fn create(&mut self, name: impl Into<String>) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        self.queues.push(Queue::new(id, name));
        self.active = Some(id);
        id
    }

    /// Removes a queue. Returns `true` when a queue was removed.
    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.queues.len();
        self.queues.retain(|q| q.id != id);
        if self.queues.len() == before {
            return false;
        }
        if self.active == Some(id) {
            self.active = self.queues.first().map(|q| q.id);
        }
        true
    }

    pub fn set_active(&mut self, id: u64) {
        if self.queues.iter().any(|q| q.id == id) {
            self.active = Some(id);
        }
    }

    /// Clears the active selection, returning playback to the library flow.
    /// Open queues are retained so the user can resume them later.
    pub fn deactivate(&mut self) {
        self.active = None;
    }
}
