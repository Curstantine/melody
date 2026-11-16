use std::path::PathBuf;

use iced::widget::operation::scrollable::{scroll_to, snap_to, AbsoluteOffset, RelativeOffset};
use iced::widget::operation::Animation;
use iced::Task;
use model::{PlaylistSummary, TrackRow};
use playlist::{parse_m3u, write_m3u};
use tracing::warn;

use crate::message::{Message, PlaylistOpenIntent};
use ::queue::{QueueSource, DEFAULT_QUEUE_NAME};
use config::PlaylistSection;

use super::queue::snap_queue_scroll_top;
use super::*;

impl App {
    pub(super) fn handle_playlists_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PlaylistSectionChanged(section) => {
                self.playlists.section = section;
                let scroll_task = match section {
                    PlaylistSection::Playlists => {
                        let id = if self.playlists.selected_id.is_some() {
                            "playlist_detail_scroll"
                        } else {
                            "playlists_scroll"
                        };
                        let y = if self.playlists.selected_id.is_some() {
                            self.playlists.last_detail_scroll.get()
                        } else {
                            self.playlists.last_playlists_scroll.get()
                        };
                        if y > 0.0 {
                            scroll_to(
                                iced::widget::Id::new(id),
                                AbsoluteOffset { x: 0.0, y },
                                Animation::Instant,
                            )
                        } else {
                            snap_to(
                                iced::widget::Id::new(id),
                                RelativeOffset::START,
                                Animation::Instant,
                            )
                        }
                    }
                    PlaylistSection::Queues => {
                        let y = self.playlists.last_queue_scroll.get();
                        if y > 0.0 {
                            scroll_to(
                                iced::widget::Id::new("queue_scroll"),
                                AbsoluteOffset { x: 0.0, y },
                                Animation::Instant,
                            )
                        } else {
                            snap_to(
                                iced::widget::Id::new("queue_scroll"),
                                RelativeOffset::START,
                                Animation::Instant,
                            )
                        }
                    }
                };
                Task::batch([self.save_state_task(), scroll_task])
            }

            Message::PlaylistsLoaded(playlists) => {
                let load_covers_task = self.load_playlist_covers_task(&playlists);
                self.playlists.set_playlists(playlists);
                self.playlists.recompute_rendered_playlists(
                    &self.layout_manager.current_template.view_formats.playlists,
                );
                self.sync_status_strings();
                load_covers_task
            }

            Message::PlaylistCoversLoaded(covers) => {
                self.playlists.set_playlist_covers(covers);
                Task::none()
            }

            Message::SelectPlaylist(id) => {
                self.dragging_detail = None;
                self.drag_detail_order_changed = false;
                if id < 0 {
                    self.playlists.clear_detail();
                    return if self.playlists.last_playlists_scroll.get() > 0.0 {
                        scroll_to(
                            iced::widget::Id::new("playlists_scroll"),
                            AbsoluteOffset {
                                x: 0.0,
                                y: self.playlists.last_playlists_scroll.get(),
                            },
                            Animation::Instant,
                        )
                    } else {
                        Task::none()
                    };
                }
                let Some(playlist) = self.playlist_by_id(id) else {
                    return Task::none();
                };
                self.playlists.selected_id = Some(id);
                let db = self.db.clone();
                let file_path = playlist.file_path.clone();
                Task::perform(
                    async move { db.resolve_playlist(&file_path).await },
                    move |resolution| Message::PlaylistResolved {
                        playlist_id: id,
                        intent: PlaylistOpenIntent::View,
                        resolution,
                    },
                )
            }

            Message::PlaylistResolved {
                playlist_id,
                intent,
                resolution,
            } => match intent {
                PlaylistOpenIntent::View => {
                    if let Some(playlist) = self.playlist_by_id(playlist_id) {
                        self.playlists.set_detail(
                            playlist_id,
                            playlist.name,
                            playlist.file_path,
                            resolution.items,
                            resolution.tracks,
                            resolution.entries,
                        );
                        self.playlists.recompute_rendered_details(
                            &self.layout_manager.current_template.view_formats.playlists,
                        );
                    }
                    Task::none()
                }
                PlaylistOpenIntent::Play { start_index } => {
                    if resolution.tracks.is_empty() {
                        return Task::none();
                    }
                    let playlist = self.playlist_by_id(playlist_id);
                    let name = playlist
                        .as_ref()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| DEFAULT_QUEUE_NAME.to_string());
                    let source_playlist = playlist.as_ref().map(|p| p.file_path.clone());
                    let index = start_index.min(resolution.tracks.len() - 1);
                    self.create_queue_from_tracks(
                        name,
                        QueueSource::Playlist,
                        None,
                        source_playlist,
                        resolution.tracks,
                    );
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.set_current(index);
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
                            Task::batch([
                                play,
                                self.persist_active_queue_task(),
                                self.snap_queue_to_current(),
                            ])
                        }
                        None => {
                            Task::batch([self.persist_active_queue_task(), snap_queue_scroll_top()])
                        }
                    }
                }
                PlaylistOpenIntent::AddToQueue => {
                    let tracks = resolution.tracks;
                    if tracks.is_empty() {
                        return Task::none();
                    }
                    if self.queues.active.is_none() {
                        self.queues.create(DEFAULT_QUEUE_NAME);
                    }
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.append_many(tracks);
                    }
                    self.queues.touch_active_summary();
                    self.persist_active_queue_task()
                }
                PlaylistOpenIntent::PlayNext => {
                    let tracks = resolution.tracks;
                    if tracks.is_empty() {
                        return Task::none();
                    }
                    if self.queues.active.is_none() {
                        self.queues.create(DEFAULT_QUEUE_NAME);
                    }
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.insert_many_next(tracks);
                    }
                    self.queues.touch_active_summary();
                    self.persist_active_queue_task()
                }
            },

            Message::PlayPlaylist(id) => {
                self.resolve_playlist_intent(id, PlaylistOpenIntent::Play { start_index: 0 })
            }
            Message::PlayPlaylistItem(id, index) => {
                self.resolve_playlist_intent(id, PlaylistOpenIntent::Play { start_index: index })
            }
            Message::PlaylistPlayNext(id) => {
                self.resolve_playlist_intent(id, PlaylistOpenIntent::PlayNext)
            }
            Message::PlaylistToQueue(id) => {
                self.resolve_playlist_intent(id, PlaylistOpenIntent::AddToQueue)
            }
            Message::PlaylistDetailClicked { playlist_id, index } => {
                if self.playlists.selected_id != Some(playlist_id) {
                    return Task::none();
                }
                let available = self
                    .playlists
                    .detail_items
                    .get(index)
                    .is_some_and(|item| item.track_id.is_some());
                if !self.playlists.on_detail_click(playlist_id, index) {
                    return Task::none();
                }
                // Double-click plays; cancel any handle-drag in progress.
                self.dragging_detail = None;
                self.drag_detail_order_changed = false;
                if available {
                    self.update(Message::PlayPlaylistItem(playlist_id, index))
                } else {
                    Task::none()
                }
            }
            Message::PlaylistDetailDragStarted { playlist_id, index } => {
                self.playlists.last_detail_click = None;
                self.playlists.hovered_detail_row = None;
                let in_range = self.playlists.selected_id == Some(playlist_id)
                    && index < self.playlists.detail_items.len();
                if in_range {
                    self.dragging_detail = Some((playlist_id, index));
                    self.drag_detail_order_changed = false;
                } else {
                    self.dragging_detail = None;
                    self.drag_detail_order_changed = false;
                }
                Task::none()
            }
            Message::PlaylistDetailDraggedOver { playlist_id, index } => {
                if let Some((drag_id, drag_index)) = self.dragging_detail {
                    if drag_id == playlist_id
                        && drag_index != index
                        && self.playlists.selected_id == Some(playlist_id)
                        && self.playlists.move_detail_entry(drag_index, index)
                    {
                        self.dragging_detail = Some((playlist_id, index));
                        self.drag_detail_order_changed = true;
                        self.playlists.selected_detail_track = Some((playlist_id, index));
                    }
                }
                Task::none()
            }
            Message::PlaylistDetailDragReleased => self.finish_detail_drag_task(),
            Message::PlaylistDetailHovered(target) => {
                self.playlists.hovered_detail_row = target;
                Task::none()
            }
            Message::PlaylistRemoveTrack { playlist_id, index } => {
                if !self.playlists.remove_detail_entry(playlist_id, index) {
                    return Task::none();
                }
                // A removal invalidates any in-progress handle-drag indices
                // for the same playlist, so cancel the drag outright.
                if self
                    .dragging_detail
                    .is_some_and(|(id, _)| id == playlist_id)
                {
                    self.dragging_detail = None;
                    self.drag_detail_order_changed = false;
                }
                let path = self.playlists.detail_path.clone();
                let entries = self.playlists.detail_entries.clone();
                Task::perform(
                    async move {
                        let wrote = tokio::task::spawn_blocking(move || write_m3u(&path, &entries))
                            .await
                            .is_ok_and(|result| result.is_ok());
                        (playlist_id, wrote)
                    },
                    |(playlist_id, success)| Message::PlaylistDetailPersisted {
                        playlist_id,
                        success,
                    },
                )
            }
            Message::PlaylistDetailPersisted {
                playlist_id,
                success,
            } => {
                if !success && self.playlists.selected_id == Some(playlist_id) {
                    // Disk write failed: reload from the file to undo the
                    // optimistic in-memory reorder.
                    return self.resolve_playlist_intent(playlist_id, PlaylistOpenIntent::View);
                }
                if success && self.playlists.selected_id == Some(playlist_id) {
                    // Reorder leaves the count unchanged but removal shrinks
                    // it, so refresh the cached catalog count in both cases.
                    let path = self.playlists.detail_path.clone();
                    let name = self.playlists.detail_name.clone();
                    let count = self.playlists.detail_items.len();
                    if path.is_empty() {
                        return Task::none();
                    }
                    let db = self.db.clone();
                    return Task::perform(
                        async move {
                            let _ = db.upsert_playlist_file(&path, &name, count).await;
                            db.get_playlists().await.unwrap_or_default()
                        },
                        Message::PlaylistsLoaded,
                    );
                }
                Task::none()
            }

            Message::NewPlaylistClicked => {
                self.playlists.creating = true;
                self.playlists.new_name_input.clear();
                Task::none()
            }
            Message::NewPlaylistNameChanged(value) => {
                self.playlists.new_name_input = value;
                Task::none()
            }
            Message::NewPlaylistCancelled => {
                self.playlists.creating = false;
                self.playlists.new_name_input.clear();
                Task::none()
            }
            Message::NewPlaylistNameSubmitted(name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                self.playlists.creating = false;
                self.playlists.new_name_input.clear();
                let pending_tracks = std::mem::take(&mut self.pending_playlist_tracks);
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let path = playlist::unique_playlist_path(&name);
                        let count = if !pending_tracks.is_empty() {
                            match playlist::append_tracks_to_m3u(&path, &pending_tracks) {
                                Ok(c) => c,
                                Err(e) => {
                                    warn!(
                                        "Failed to create playlist with tracks {:?}: {}",
                                        path, e
                                    );
                                    0
                                }
                            }
                        } else {
                            if let Err(e) = write_m3u(&path, &[]) {
                                warn!("Failed to create playlist file {:?}: {}", path, e);
                            }
                            0
                        };
                        let path_str = path.to_string_lossy().to_string();
                        let _ = db.upsert_playlist_file(&path_str, &name, count).await;
                        db.get_playlists().await.unwrap_or_default()
                    },
                    Message::PlaylistsLoaded,
                )
            }

            Message::RenamePlaylist(id) => {
                if let Some(playlist) = self.playlist_by_id(id) {
                    self.playlists.renaming = true;
                    self.playlists.rename_input = playlist.name;
                }
                Task::none()
            }
            Message::RenamePlaylistNameChanged(value) => {
                self.playlists.rename_input = value;
                Task::none()
            }
            Message::RenamePlaylistCancelled => {
                self.playlists.renaming = false;
                self.playlists.rename_input.clear();
                Task::none()
            }
            Message::RenamePlaylistSubmitted(id, name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Task::none();
                }
                self.playlists.renaming = false;
                self.playlists.rename_input.clear();
                let Some(playlist) = self.playlist_by_id(id) else {
                    return Task::none();
                };
                let db = self.db.clone();
                Task::perform(
                    async move {
                        if playlist.managed {
                            let old_path = PathBuf::from(&playlist.file_path);
                            let old_stem = old_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or_default();
                            let sanitized = playlist::sanitize_playlist_filename(&name);
                            let new_path = if sanitized == old_stem {
                                old_path.clone()
                            } else {
                                playlist::unique_playlist_path(&name)
                            };

                            if new_path != old_path {
                                if let Err(e) = std::fs::rename(&old_path, &new_path) {
                                    warn!("Failed to rename playlist file: {}", e);
                                }
                                let new_str = new_path.to_string_lossy().to_string();
                                let _ = db
                                    .upsert_playlist_file(&new_str, &name, playlist.track_count)
                                    .await;
                                let _ = db.remove_playlist_file_by_path(&playlist.file_path).await;
                            } else {
                                let _ = db.rename_playlist_file(playlist.id, &name).await;
                            }
                        } else {
                            let _ = db.rename_playlist_file(playlist.id, &name).await;
                        }
                        db.get_playlists().await.unwrap_or_default()
                    },
                    Message::PlaylistsLoaded,
                )
            }

            Message::DeletePlaylist(id) => {
                self.dragging_detail = None;
                self.drag_detail_order_changed = false;
                self.playlists.clear_detail();
                let playlist = self.playlist_by_id(id);
                let db = self.db.clone();
                Task::perform(
                    async move {
                        if let Some(playlist) = playlist {
                            if playlist.managed {
                                let _ = std::fs::remove_file(&playlist.file_path);
                            }
                            let _ = db.remove_playlist_file(playlist.id).await;
                        }
                        db.get_playlists().await.unwrap_or_default()
                    },
                    Message::PlaylistsLoaded,
                )
            }

            Message::PlaylistSortChanged(field) => {
                self.playlists.set_sort_field(field);
                self.save_state_task()
            }

            Message::SetPlaylistsViewMode(mode) => {
                self.playlists.view_mode = mode;
                self.save_state_task()
            }

            Message::ImportPlaylistsClicked => Task::perform(
                async {
                    let files = rfd::AsyncFileDialog::new()
                        .set_title("Import playlists")
                        .add_filter("Playlists", &["m3u", "m3u8"])
                        .pick_files()
                        .await;
                    files.map(|handles| {
                        handles
                            .into_iter()
                            .map(|f| f.path().to_path_buf())
                            .collect::<Vec<_>>()
                    })
                },
                Message::PlaylistsImported,
            ),

            Message::PlaylistsImported(paths) => {
                let Some(paths) = paths else {
                    return Task::none();
                };
                if paths.is_empty() {
                    return Task::none();
                }
                let db = self.db.clone();
                Task::perform(
                    async move {
                        for path in paths {
                            if !playlist::is_supported_playlist_file(&path) {
                                continue;
                            }
                            match parse_m3u(&path) {
                                Ok(parsed) => {
                                    let path_str = path.to_string_lossy().to_string();
                                    let _ = db
                                        .upsert_playlist_file(
                                            &path_str,
                                            &parsed.name,
                                            parsed.entries.len(),
                                        )
                                        .await;
                                }
                                Err(e) => warn!("Failed to import playlist {:?}: {}", path, e),
                            }
                        }
                        db.get_playlists().await.unwrap_or_default()
                    },
                    Message::PlaylistsLoaded,
                )
            }

            // ── Queues ──────────────────────────────────────────────────────
            _ => unreachable!("routed in App::update"),
        }
    }

    /// Writes the open playlist's reordered entries back to its M3U file.
    /// Fires exactly once on drag-release (never per hover), off the UI
    /// thread since the write is synchronous filesystem I/O.
    pub(super) fn finish_detail_drag_task(&mut self) -> Task<Message> {
        let released = self.dragging_detail.take();
        let order_changed = std::mem::take(&mut self.drag_detail_order_changed);
        // The cursor stays over the dropped row but no new hover event will
        // fire, so restore the reveal directly.
        if let Some(pos) = released {
            self.playlists.hovered_detail_row = Some(pos);
        }
        let (Some((playlist_id, _)), true) = (released, order_changed) else {
            return Task::none();
        };
        if self.playlists.selected_id != Some(playlist_id) {
            return Task::none();
        }
        let path = self.playlists.detail_path.clone();
        let entries = self.playlists.detail_entries.clone();
        Task::perform(
            async move {
                let wrote = tokio::task::spawn_blocking(move || write_m3u(&path, &entries))
                    .await
                    .is_ok_and(|result| result.is_ok());
                (playlist_id, wrote)
            },
            |(playlist_id, success)| Message::PlaylistDetailPersisted {
                playlist_id,
                success,
            },
        )
    }

    pub(super) fn playlist_by_id(&self, id: i64) -> Option<PlaylistSummary> {
        self.playlists
            .playlists
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    pub(super) fn resolve_playlist_intent(
        &self,
        id: i64,
        intent: PlaylistOpenIntent,
    ) -> Task<Message> {
        let Some(playlist) = self.playlist_by_id(id) else {
            return Task::none();
        };
        let db = self.db.clone();
        let file_path = playlist.file_path.clone();
        Task::perform(
            async move { db.resolve_playlist(&file_path).await },
            move |resolution| Message::PlaylistResolved {
                playlist_id: id,
                intent,
                resolution,
            },
        )
    }

    /// Asynchronously extracts preview cover art thumbnails for the given playlists.
    /// Reads up to 4 track file paths from each playlist's M3U file, downscaling/caching
    /// cover art thumbnails off the main thread.
    pub(super) fn load_playlist_covers_task(&self, playlists: &[PlaylistSummary]) -> Task<Message> {
        if playlists.is_empty() {
            return Task::none();
        }

        let targets: Vec<(i64, PathBuf)> = playlists
            .iter()
            .map(|p| (p.id, PathBuf::from(&p.file_path)))
            .collect();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut results = Vec::with_capacity(targets.len());
                    for (id, m3u_path) in targets {
                        let track_paths =
                            playlist::parse_m3u_preview_paths(&m3u_path, 4).unwrap_or_default();
                        let mut covers = Vec::with_capacity(track_paths.len());
                        for track_path in track_paths {
                            if let Some(bytes) =
                                thumbnail::get_or_create(&track_path, thumbnail::Resolution::Low)
                            {
                                if !bytes.is_empty() {
                                    covers.push(bytes);
                                }
                            }
                        }

                        let final_covers =
                            thumbnail::generate_collage(&covers, thumbnail::Resolution::Low)
                                .or_else(|| covers.into_iter().next())
                                .into_iter()
                                .collect();

                        results.push((id, final_covers));
                    }
                    results
                })
                .await
                .unwrap_or_default()
            },
            Message::PlaylistCoversLoaded,
        )
    }

    pub(super) fn playlist_contains_track(&self, playlist_id: i64, track: &TrackRow) -> bool {
        if self.playlists.selected_id == Some(playlist_id) {
            return self
                .playlists
                .detail_tracks
                .iter()
                .any(|t| t.id == track.id || t.file_path == track.file_path)
                || self.playlists.detail_entries.iter().any(|e| {
                    e.resolved_path.to_string_lossy() == track.file_path
                        || e.raw_path == track.file_path
                });
        }

        if let Some(p) = self.playlist_by_id(playlist_id) {
            if let Ok(parsed) = playlist::parse_m3u(&p.file_path) {
                return parsed.entries.iter().any(|e| {
                    e.resolved_path.to_string_lossy() == track.file_path
                        || e.raw_path == track.file_path
                });
            }
        }

        false
    }

    pub(super) fn execute_add_track_to_playlist(
        &mut self,
        track: TrackRow,
        playlist_id: i64,
    ) -> Task<Message> {
        let playlist = self.playlist_by_id(playlist_id);
        if let Some(p) = playlist {
            let path = PathBuf::from(&p.file_path);
            let file_path = p.file_path.clone();
            let name = p.name.clone();
            let db = self.db.clone();
            let is_open = self.playlists.selected_id == Some(playlist_id);
            let reload_detail = if is_open {
                self.resolve_playlist_intent(playlist_id, PlaylistOpenIntent::View)
            } else {
                Task::none()
            };

            let save_task = Task::perform(
                async move {
                    let wrote = tokio::task::spawn_blocking(move || {
                        playlist::append_tracks_to_m3u(&path, &[track])
                    })
                    .await
                    .ok()
                    .and_then(|r| r.ok());

                    if let Some(count) = wrote {
                        let _ = db.upsert_playlist_file(&file_path, &name, count).await;
                    }
                    db.get_playlists().await.unwrap_or_default()
                },
                Message::PlaylistsLoaded,
            );
            Task::batch([save_task, reload_detail])
        } else {
            Task::none()
        }
    }
}
