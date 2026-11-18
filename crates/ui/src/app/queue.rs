use std::path::PathBuf;

use iced::widget::operation::scrollable::{snap_to, RelativeOffset};
use iced::widget::operation::Animation;
use iced::Task;
use model::{SortDirection, SortField, TrackFilter, TrackRow};
use playlist::{write_m3u, PlaylistEntry};
use tracing::warn;

use crate::message::Message;
use ::queue::{queue_window_for_start, QueueSource, DEFAULT_QUEUE_NAME};
use config::PlaylistSection;

use super::*;

impl App {
    pub(super) fn handle_queue_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::QueueNew => {
                self.queues.create(DEFAULT_QUEUE_NAME);
                self.playlists.reset_queue_window();
                self.playlists.recompute_rendered_queue(
                    &self.layout_manager.current_template.view_formats.queue,
                    &self.queues,
                );
                self.playlists.section = PlaylistSection::Queues;
                Task::batch([self.persist_active_queue_task(), snap_queue_scroll_top()])
            }
            Message::QueueSelect(id) => {
                if self.queues.active == Some(id) {
                    return Task::none();
                }
                self.dragging_queue_track = None;
                self.drag_queue_order_changed = false;
                self.queues.set_active(id);
                self.playlists.reset_queue_window();
                let db = self.db.clone();
                let persist_db = db.clone();
                let persist = Task::perform(
                    async move {
                        let _ = persist_db.set_active_queue_id(Some(id as i64)).await;
                    },
                    |_| Message::QueuesPersisted,
                );
                let load = Task::perform(
                    async move {
                        let tracks = db.load_queue_tracks(id as i64).await.unwrap_or_default();
                        (id, tracks)
                    },
                    |(id, tracks)| Message::QueueActivated { id, tracks },
                );
                let save_state = self.save_state_debounced();
                Task::batch([persist, load, snap_queue_scroll_top(), save_state])
            }
            Message::QueueDelete(id) => {
                let was_active = self.queues.active == Some(id);
                self.dragging_queue_track = None;
                self.drag_queue_order_changed = false;
                let was_playing_deleted = was_active && self.current_track_id.is_some();
                let fallback = self.queues.remove(id);
                self.playlists.reset_queue_window();
                if was_active && was_playing_deleted {
                    self.current_track = None;
                    self.player_bar.update_track(None);
                    self.current_track_id = None;
                    self.current_cover = None;
                    self.current_now_playing_cover = None;
                    self.current_blurred_cover = None;
                    self.current_tech_info = None;
                    self.now_playing.clear();
                    self.sync_now_playing();
                    self.is_playing = false;
                    self.position_ms = 0;
                    self.audio_controller.stop();
                }
                let active = self.queues.active.map(|a| a as i64);
                let db = self.db.clone();
                let persist_db = db.clone();
                let persist = Task::perform(
                    async move {
                        let _ = persist_db.delete_queue(id as i64).await;
                        let _ = persist_db.set_active_queue_id(active).await;
                    },
                    |_| Message::QueuesPersisted,
                );
                let save_state = self.save_state_debounced();
                if let Some(next) = fallback {
                    let load = Task::perform(
                        async move {
                            let tracks =
                                db.load_queue_tracks(next as i64).await.unwrap_or_default();
                            (next, tracks)
                        },
                        |(id, tracks)| Message::QueueActivated { id, tracks },
                    );
                    Task::batch([persist, load, snap_queue_scroll_top(), save_state])
                } else {
                    Task::batch([persist, snap_queue_scroll_top(), save_state])
                }
            }
            Message::QueueActivated { id, tracks } => {
                self.queues.activate_with_tracks(id, tracks);
                self.sync_status_strings();
                self.playlists.reset_queue_window();
                self.playlists.recompute_rendered_queue(
                    &self.layout_manager.current_template.view_formats.queue,
                    &self.queues,
                );
                let snap = self.snap_queue_to_current();
                if self.current_track.is_none() {
                    if let Some(track_row) = self
                        .queues
                        .active_queue()
                        .and_then(|q| q.current_track())
                        .cloned()
                    {
                        let db = self.db.clone();
                        let restore_track = Task::perform(
                            async move { db.get_track_by_id(track_row.id).await.unwrap_or(None) },
                            |res| Message::TrackResolvedForRestore(Box::new(res)),
                        );
                        return Task::batch([snap, restore_track]);
                    }
                }
                snap
            }
            Message::QueueScrolled(viewport) => {
                let scroll_y = viewport.absolute_offset().y;
                if let Some(queue) = self.queues.active_queue() {
                    if self.playlists.on_queue_scroll_offset(scroll_y, queue.len()) {
                        self.playlists.recompute_rendered_queue(
                            &self.layout_manager.current_template.view_formats.queue,
                            &self.queues,
                        );
                    }
                }
                Task::none()
            }
            Message::LibraryQueueBuilt {
                track_id,
                name,
                source_kind,
                tracks,
            } => {
                if tracks.is_empty() {
                    return Task::none();
                }
                let source = QueueSource::from_str(&source_kind);
                self.create_queue_from_tracks(name, source, None, None, tracks);
                let index = self
                    .queues
                    .active_queue()
                    .and_then(|q| q.tracks.iter().position(|t| t.id == track_id))
                    .unwrap_or(0);
                if let Some(queue) = self.queues.active_queue_mut() {
                    queue.set_current(index);
                }
                self.queues.touch_active_summary();
                self.playlists.recompute_rendered_queue(
                    &self.layout_manager.current_template.view_formats.queue,
                    &self.queues,
                );
                let play = self.update(Message::PlayTrack(track_id));
                Task::batch([
                    play,
                    self.persist_active_queue_task(),
                    self.snap_queue_to_current(),
                    self.save_state_debounced(),
                ])
            }
            Message::PlayScope {
                filter,
                name,
                shuffle,
            } => {
                let source = queue_source_for_filter(&filter);
                // Albums play in disc/track order; other scopes follow the
                // library's current ordering.
                let (sort_field, sort_dir) = if filter.album.is_some() {
                    (SortField::TrackNum, SortDirection::Ascending)
                } else {
                    (self.library.sort_field, self.library.sort_dir)
                };
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let mut tracks = db
                            .get_all_track_rows_filtered(Some(&filter), sort_field, sort_dir)
                            .await
                            .unwrap_or_default();
                        if shuffle {
                            ::queue::shuffle_tracks(&mut tracks);
                        }
                        let track_id = tracks.first().map(|t| t.id).unwrap_or(0);
                        Message::LibraryQueueBuilt {
                            track_id,
                            name: name.to_string(),
                            source_kind: source.as_str().to_string(),
                            tracks,
                        }
                    },
                    |message| message,
                )
            }
            Message::QueueRemoveTrack { queue_id, index } => {
                if self.queues.active == Some(queue_id) {
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.remove(index);
                    }
                }
                self.queues.touch_active_summary();
                self.playlists.recompute_rendered_queue(
                    &self.layout_manager.current_template.view_formats.queue,
                    &self.queues,
                );
                Task::batch([
                    self.persist_active_queue_task(),
                    self.save_state_debounced(),
                ])
            }
            Message::QueueClicked { queue_id, index } => {
                // Single click selects (arming double-click to play, like the
                // library); double-click plays. Dragging starts only from the
                // row's drag handle, so plain clicks never disturb the order.
                let is_double_click = self.playlists.on_queue_click(queue_id, index);
                if !is_double_click {
                    return Task::none();
                }
                // Double-click plays; cancel any handle-drag in progress.
                self.dragging_queue_track = None;
                self.drag_queue_order_changed = false;
                if self.queues.active == Some(queue_id) {
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.set_current(index);
                    }
                }
                self.queues.touch_active_summary();
                let track_id = self
                    .queues
                    .active_queue()
                    .and_then(|q| q.current_track())
                    .map(|t| t.id);
                match track_id {
                    Some(id) => {
                        let play = self.update(Message::PlayTrack(id));
                        Task::batch([play, self.persist_active_current_task()])
                    }
                    None => Task::none(),
                }
            }
            Message::QueueDragStarted { queue_id, index } => {
                // Dragging starts only from the revealed handle. A pending
                // double-click arm is cancelled so a handle press never plays.
                self.playlists.last_queue_click = None;
                self.playlists.hovered_queue_row = None;
                let in_range = self
                    .queues
                    .active_queue()
                    .is_some_and(|q| self.queues.active == Some(queue_id) && index < q.len());
                if in_range {
                    self.dragging_queue_track = Some((queue_id, index));
                    self.drag_queue_order_changed = false;
                } else {
                    self.dragging_queue_track = None;
                    self.drag_queue_order_changed = false;
                }
                Task::none()
            }
            Message::QueueDraggedOver { queue_id, index } => {
                // Live-reorder while held, mirroring tab-bar dragging.
                if let Some((drag_id, drag_index)) = self.dragging_queue_track {
                    if drag_id == queue_id
                        && drag_index != index
                        && self.queues.active == Some(queue_id)
                    {
                        let moved = self
                            .queues
                            .active_queue_mut()
                            .is_some_and(|q| index < q.len() && q.move_track_to(drag_index, index));
                        if moved {
                            self.dragging_queue_track = Some((queue_id, index));
                            self.drag_queue_order_changed = true;
                            self.playlists.selected_queue_track = Some((queue_id, index));
                            self.queues.touch_active_summary();
                            self.playlists.recompute_rendered_queue(
                                &self.layout_manager.current_template.view_formats.queue,
                                &self.queues,
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::QueueDragReleased => self.finish_queue_drag_task(),
            Message::QueueRowHovered(target) => {
                self.playlists.hovered_queue_row = target;
                Task::none()
            }
            Message::QueueSaveClicked => {
                self.playlists.saving_queue = true;
                self.playlists.queue_save_input = self
                    .queues
                    .active_queue()
                    .map(|q| q.name.clone())
                    .unwrap_or_else(|| DEFAULT_QUEUE_NAME.to_string());
                Task::none()
            }
            Message::QueueSaveNameChanged(value) => {
                self.playlists.queue_save_input = value;
                Task::none()
            }
            Message::QueueSaveCancelled => {
                self.playlists.saving_queue = false;
                self.playlists.queue_save_input.clear();
                Task::none()
            }
            Message::QueueSaveSubmitted(name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                self.playlists.saving_queue = false;
                self.playlists.queue_save_input.clear();
                let tracks: Vec<TrackRow> = self
                    .queues
                    .active_queue()
                    .map(|q| q.tracks.clone())
                    .unwrap_or_default();
                if tracks.is_empty() {
                    return Task::none();
                }
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let path = playlist::unique_playlist_path(&name);
                        let entries: Vec<PlaylistEntry> = tracks
                            .iter()
                            .map(|t| {
                                let raw_path = t.file_path.to_string();
                                let resolved_path = PathBuf::from(&raw_path);
                                PlaylistEntry {
                                    raw_path,
                                    resolved_path,
                                    title: Some(t.title.clone()),
                                    duration_ms: Some(t.duration_ms),
                                }
                            })
                            .collect();
                        if let Err(e) = write_m3u(&path, &entries) {
                            warn!("Failed to save queue as playlist: {}", e);
                        }
                        let path_str = path.to_string_lossy().to_string();
                        let _ = db
                            .upsert_playlist_file(&path_str, &name, entries.len())
                            .await;
                        db.get_playlists().await.unwrap_or_default()
                    },
                    Message::PlaylistsLoaded,
                )
            }
            Message::QueuesPersisted => Task::none(),

            _ => unreachable!("routed in App::update"),
        }
    }

    pub(super) fn step_active_queue(&mut self, forward: bool) -> Option<i64> {
        let position_seed = self.position_ms;
        let is_shuffle = self.is_shuffle;
        let repeat_all = self.repeat_mode == model::RepeatMode::All;
        let queue = self.queues.active_queue_mut()?;
        if queue.is_empty() {
            return None;
        }

        let target = if is_shuffle {
            let len = queue.len();
            let current = queue.current_index().unwrap_or(0);
            if len <= 1 {
                current
            } else {
                ::queue::shuffle_pick_index(current, position_seed, len)
            }
        } else if forward {
            match queue.next_index() {
                Some(idx) => idx,
                None if repeat_all => 0,
                None => return None,
            }
        } else {
            match queue.previous_index() {
                Some(idx) => idx,
                None if repeat_all => queue.len() - 1,
                None => return None,
            }
        };

        queue.set_current(target);
        Some(queue.tracks[target].id)
    }

    /// Persists the active queue (metadata + membership + active selection).
    pub(super) fn persist_active_queue_task(&self) -> Task<Message> {
        let Some(queue) = self.queues.active_queue() else {
            return Task::none();
        };
        let record = queue.to_record();
        let ids = queue.track_ids();
        let db = self.db.clone();
        Task::perform(
            async move {
                let _ = db.upsert_queue(&record).await;
                let _ = db.replace_queue_tracks(record.id, &ids).await;
                let _ = db.set_active_queue_id(Some(record.id)).await;
            },
            |_| Message::QueuesPersisted,
        )
    }

    /// Persists only the active queue's cursor (cheap; used on every skip).
    pub(super) fn persist_active_current_task(&self) -> Task<Message> {
        let Some(queue) = self.queues.active_queue() else {
            return Task::none();
        };
        let id = queue.id as i64;
        let current = queue.current as i64;
        let db = self.db.clone();
        Task::perform(
            async move {
                let _ = db.set_queue_current(id, current).await;
            },
            |_| Message::QueuesPersisted,
        )
    }

    /// Ends a queue drag-reorder, persisting once iff the order changed.
    pub(super) fn finish_queue_drag_task(&mut self) -> Task<Message> {
        let released = self.dragging_queue_track.take();
        let order_changed = std::mem::take(&mut self.drag_queue_order_changed);
        // The cursor stays over the dropped row but no new hover event will
        // fire, so restore the reveal directly.
        if let Some(pos) = released {
            self.playlists.hovered_queue_row = Some(pos);
        }
        if released.is_some() && order_changed {
            let persist = self.persist_active_queue_task();
            let save_state = self.save_state_debounced();
            Task::batch([persist, save_state])
        } else {
            Task::none()
        }
    }

    /// Snaps the queue detail list so the playing cursor is visible. Skips
    /// when the cursor already sits inside the rendered window.
    pub(super) fn snap_queue_to_current(&self) -> Task<Message> {
        let Some(queue) = self.queues.active_queue() else {
            return Task::none();
        };
        let total = queue.len();
        let Some(current) = queue.current_index() else {
            return Task::none();
        };
        if total == 0 {
            return Task::none();
        }
        let (start, end) = queue_window_for_start(total, self.playlists.queue_window_start);
        if current >= start && current < end {
            return Task::none();
        }
        let y = (current as f32 / total as f32).clamp(0.0, 1.0);
        snap_to(
            iced::widget::Id::new("queue_scroll"),
            RelativeOffset { x: 0.0, y },
            Animation::Instant,
        )
    }

    /// Starts playback of a library track by building a queue from the current
    /// library scope (album/artist/search, or the whole library).
    pub(super) fn start_library_playback(&self, track_id: i64) -> Task<Message> {
        let filter = self.library.filter.clone();
        let name = filter
            .queue_name()
            .unwrap_or_else(|| "All Songs".to_string());
        let source = queue_source_for_filter(&filter);
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;
        let db = self.db.clone();

        Task::perform(
            async move {
                let tracks = db
                    .get_all_track_rows_filtered(Some(&filter), sort_field, sort_dir)
                    .await
                    .unwrap_or_default();
                Message::LibraryQueueBuilt {
                    track_id,
                    name,
                    source_kind: source.as_str().to_string(),
                    tracks,
                }
            },
            |message| message,
        )
    }

    /// Replaces the same-named queue (or creates a new one) from an ordered
    /// track list and makes it active.
    pub(super) fn create_queue_from_tracks(
        &mut self,
        name: String,
        source: QueueSource,
        source_value: Option<String>,
        source_playlist: Option<String>,
        tracks: Vec<TrackRow>,
    ) -> u64 {
        self.playlists.reset_queue_window();
        if let Some(id) = self.queues.find_id_by_name(&name) {
            self.queues.replace_with_tracks(id, tracks);
            self.playlists.recompute_rendered_queue(
                &self.layout_manager.current_template.view_formats.queue,
                &self.queues,
            );
            return id;
        }
        let id = self
            .queues
            .create_with_source(name, source, source_value, source_playlist);
        if let Some(queue) = self.queues.active_queue_mut() {
            queue.tracks = tracks;
            queue.current = 0;
        }
        self.queues.touch_active_summary();
        self.playlists.recompute_rendered_queue(
            &self.layout_manager.current_template.view_formats.queue,
            &self.queues,
        );
        id
    }

    pub(super) fn queue_contains_track(&self, track: &TrackRow) -> bool {
        self.queues
            .active_queue()
            .map(|q| {
                q.tracks
                    .iter()
                    .any(|t| t.id == track.id || t.file_path == track.file_path)
            })
            .unwrap_or(false)
    }

    pub(super) fn execute_add_track_to_queue(&mut self, track: TrackRow) -> Task<Message> {
        if self.queues.active.is_none() {
            self.queues.create(DEFAULT_QUEUE_NAME);
        }
        if let Some(queue) = self.queues.active_queue_mut() {
            queue.append_many([track]);
        }
        self.queues.touch_active_summary();
        self.persist_active_queue_task()
    }
}

pub(super) fn queue_source_for_filter(filter: &TrackFilter) -> QueueSource {
    if filter.album.is_some() {
        QueueSource::Album
    } else if filter.artist.is_some() {
        QueueSource::Artist
    } else if !filter.text.is_empty() || filter.title.is_some() {
        QueueSource::Search
    } else {
        QueueSource::AllSongs
    }
}

/// Snaps the queue detail list back to the top (used on page changes).
pub(super) fn snap_queue_scroll_top() -> Task<Message> {
    snap_to(
        iced::widget::Id::new("queue_scroll"),
        RelativeOffset::START,
        Animation::Instant,
    )
}
