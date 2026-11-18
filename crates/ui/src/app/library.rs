use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use iced::widget::operation::scrollable::{scroll_to, snap_to, AbsoluteOffset, RelativeOffset};
use iced::widget::operation::Animation;
use iced::Task;
use model::{SortDirection, SortField, TrackFilter};
use tracing::{info, warn};

use crate::message::Message;
use config::ViewTab;

use super::*;

impl App {
    pub(super) fn handle_library_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchInputChanged(query) => {
                self.library.set_search_query(query);
                // Debounce: typing fires a query per keystroke, and a leading
                // wildcard LIKE cannot use an index, so we wait out the burst.
                self.search_seq = self.search_seq.wrapping_add(1);
                let seq = self.search_seq;
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        seq
                    },
                    Message::SearchDebounced,
                )
            }

            Message::SearchDebounced(seq) => {
                if seq == self.search_seq {
                    let snap = snap_to(
                        iced::widget::Id::new("library_tracks"),
                        RelativeOffset::START,
                        Animation::Instant,
                    );
                    Task::batch([self.reload_tracks_task(), snap])
                } else {
                    Task::none()
                }
            }

            Message::SortChanged(field) => {
                self.library.toggle_sort(field);
                let snap = snap_to(
                    iced::widget::Id::new("library_tracks"),
                    RelativeOffset::START,
                    Animation::Instant,
                );
                Task::batch([self.reload_tracks_task(), snap])
            }

            Message::SetPageSize(size) => {
                self.library.page_size = size;
                Task::batch([self.save_config_task(), self.reload_tracks_task()])
            }

            Message::SetAlbumsViewMode(mode) => {
                let changed = self.albums.set_view_mode(mode);
                if changed {
                    self.albums.recompute_rendered_window(
                        &self.layout_manager.current_template.view_formats.albums,
                    );
                }
                let covers = if changed {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                };
                let scroll = if self.albums.scroll_offset.get() > 0.0 {
                    scroll_to(
                        iced::widget::Id::new("albums_scroll"),
                        AbsoluteOffset {
                            x: 0.0,
                            y: self.albums.scroll_offset.get(),
                        },
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                };
                Task::batch([self.save_state_task(), covers, scroll])
            }

            Message::SetArtistsViewMode(mode) => {
                if self.artists.set_view_mode(mode) {
                    self.artists.recompute_rendered_window(
                        &self.layout_manager.current_template.view_formats.artists,
                    );
                }
                let scroll = if self.artists.scroll_offset.get() > 0.0 {
                    scroll_to(
                        iced::widget::Id::new("artists_scroll"),
                        AbsoluteOffset {
                            x: 0.0,
                            y: self.artists.scroll_offset.get(),
                        },
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                };
                Task::batch([self.save_state_task(), scroll])
            }

            Message::SetAlbumTileWidth(width) => {
                let changed = self.albums.set_tile_width(width);
                if changed {
                    self.albums.recompute_rendered_window(
                        &self.layout_manager.current_template.view_formats.albums,
                    );
                }
                let covers = if changed {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                };
                Task::batch([self.save_config_debounced(), covers])
            }

            Message::SetArtistTileWidth(width) => {
                if self.artists.set_tile_width(width) {
                    self.artists.recompute_rendered_window(
                        &self.layout_manager.current_template.view_formats.artists,
                    );
                }
                self.save_config_debounced()
            }

            Message::TrackClicked(id) => {
                if self.library.on_track_click(id) {
                    // Playing from the library (re)builds a queue from the
                    // current scope (album/artist/search/all songs) so
                    // next/previous follow exactly what the user is viewing.
                    self.start_library_playback(id)
                } else {
                    Task::none()
                }
            }

            Message::AddLibraryFolderClicked => {
                self.is_scanning = true;
                Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Add music folder to library")
                            .pick_folder()
                            .await;
                        folder.map(|f| f.path().to_path_buf())
                    },
                    Message::LibraryFolderAdded,
                )
            }

            Message::LibraryFolderAdded(maybe_path) => match maybe_path {
                Some(path) => {
                    let path_str = path.to_string_lossy().to_string();
                    info!("Adding library folder: {}", path_str);
                    if let Some(ref mut watcher) = self.library_watcher {
                        let _ = watcher.watch_folder(&path);
                    }
                    self.is_scanning = true;
                    self.scan_progress = None;
                    let db = self.db.clone();
                    Task::stream(iced::stream::channel(100, async move |mut sender| {
                        let _ = db.add_library_folder(&path_str).await;
                        let mut cb_sender = sender.clone();
                        let summary = db
                            .scan_directory(
                                &path,
                                Some(move |current, total| {
                                    let _ = cb_sender
                                        .try_send(Message::ScanProgress { current, total });
                                }),
                            )
                            .await
                            .unwrap_or_default();
                        let _ = sender.send(Message::ScanFinished(summary)).await;
                    }))
                }
                None => {
                    self.is_scanning = false;
                    self.scan_progress = None;
                    Task::none()
                }
            },

            Message::RemoveLibraryFolder(path_str) => {
                info!("Removing library folder: {}", path_str);
                if let Some(ref mut watcher) = self.library_watcher {
                    let _ = watcher.unwatch_folder(Path::new(&path_str));
                }
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let _ = db.remove_library_folder(&path_str).await;
                        db.get_library_folders().await.unwrap_or_default()
                    },
                    Message::LibraryFoldersLoaded,
                )
            }

            Message::LibraryFoldersLoaded(folders) => {
                self.settings.library_folders = folders.clone();
                if self.watch_library {
                    if let Some(ref mut watcher) = self.library_watcher {
                        let _ = watcher.update_folders(&folders);
                    } else {
                        let (tx, rx) = crossbeam_channel::unbounded();
                        match metadata::LibraryWatcher::new(tx, Duration::from_millis(1000)) {
                            Ok(mut watcher) => {
                                let _ = watcher.update_folders(&folders);
                                self.library_watcher = Some(watcher);
                                self.watcher_rx = Some(rx);
                            }
                            Err(e) => {
                                warn!("Failed to initialize library directory watcher: {}", e);
                            }
                        }
                    }
                }
                Task::none()
            }

            Message::RescanLibraryFolder(path) => {
                self.is_scanning = true;
                self.scan_progress = None;
                info!("Rescanning library folder: {:?}", path);
                let db = self.db.clone();
                Task::stream(iced::stream::channel(100, async move |mut sender| {
                    let mut cb_sender = sender.clone();
                    let summary = db
                        .scan_directory(
                            &path,
                            Some(move |current, total| {
                                let _ =
                                    cb_sender.try_send(Message::ScanProgress { current, total });
                            }),
                        )
                        .await
                        .unwrap_or_default();
                    let _ = sender.send(Message::ScanFinished(summary)).await;
                }))
            }

            Message::ReindexLibrary => {
                self.is_scanning = true;
                self.scan_progress = None;
                info!(
                    "Reindexing all library folders: {:?}",
                    self.settings.library_folders
                );
                let db = self.db.clone();
                let folders = self.settings.library_folders.clone();
                Task::stream(iced::stream::channel(100, async move |mut sender| {
                    let mut cb_sender = sender.clone();
                    let paths: Vec<PathBuf> = folders.into_iter().map(PathBuf::from).collect();
                    let summary = db
                        .scan_directories(
                            &paths,
                            Some(move |current, total| {
                                let _ =
                                    cb_sender.try_send(Message::ScanProgress { current, total });
                            }),
                        )
                        .await
                        .unwrap_or_default();
                    let _ = sender.send(Message::ScanFinished(summary)).await;
                }))
            }

            Message::ScanLibraryForNewFiles => {
                self.is_scanning = true;
                self.scan_progress = None;
                info!(
                    "Scanning for new files in library folders: {:?}",
                    self.settings.library_folders
                );
                let db = self.db.clone();
                let folders = self.settings.library_folders.clone();
                Task::stream(iced::stream::channel(100, async move |mut sender| {
                    let mut cb_sender = sender.clone();
                    let paths: Vec<PathBuf> = folders.into_iter().map(PathBuf::from).collect();
                    let summary = db
                        .scan_directories_new(
                            &paths,
                            Some(move |current, total| {
                                let _ =
                                    cb_sender.try_send(Message::ScanProgress { current, total });
                            }),
                        )
                        .await
                        .unwrap_or_default();
                    let _ = sender.send(Message::ScanFinished(summary)).await;
                }))
            }

            Message::ToggleWatchLibrary(enabled) => {
                self.watch_library = enabled;
                if enabled {
                    if self.library_watcher.is_none() {
                        let (tx, rx) = crossbeam_channel::unbounded();
                        match metadata::LibraryWatcher::new(tx, Duration::from_millis(1000)) {
                            Ok(mut watcher) => {
                                let _ = watcher.update_folders(&self.settings.library_folders);
                                self.library_watcher = Some(watcher);
                                self.watcher_rx = Some(rx);
                            }
                            Err(e) => {
                                warn!("Failed to initialize library directory watcher: {}", e);
                            }
                        }
                    } else if let Some(ref mut watcher) = self.library_watcher {
                        let _ = watcher.update_folders(&self.settings.library_folders);
                    }
                } else {
                    if let Some(mut watcher) = self.library_watcher.take() {
                        watcher.stop();
                    }
                    self.watcher_rx = None;
                    self.pending_watcher_paths.clear();
                }
                self.save_config_task()
            }

            Message::ScanProgress { current, total } => {
                self.scan_progress = Some((current, total));
                Task::none()
            }

            Message::ClearLibrary => {
                info!("Clearing all tracks from library database");
                let db = self.db.clone();
                self.library.clear();
                self.albums.clear();
                self.artists.clear();
                self.library_counts = db::LibraryCounts::default();
                self.status_settings_str = "0 tracks indexed".to_string();
                self.now_playing.clear();
                self.current_track = None;
                self.player_bar.update_track(None);
                self.sync_now_playing();
                self.current_track_id = None;
                self.current_cover = None;
                self.current_now_playing_cover = None;
                self.is_playing = false;
                self.audio_controller.stop();
                Task::perform(
                    async move {
                        let _ = db.clear_all_tracks().await;
                    },
                    |_| Message::TracksLoaded(Vec::new(), 0),
                )
            }

            Message::ClearThumbnailCache => {
                info!("Purging thumbnail cache");
                self.albums.clear_covers();
                let db_path = self.db_path;
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let counts = db.get_library_counts().await.unwrap_or_default();
                        let (stats, db_size) = tokio::task::spawn_blocking(move || {
                            let _ = thumbnail::clear_thumbnail_cache();
                            let stats = thumbnail::get_thumbnail_cache_stats();
                            let db_size = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
                            (stats, db_size)
                        })
                        .await
                        .unwrap_or_default();
                        (stats, db_size, counts)
                    },
                    |(stats, db_size, counts)| Message::StorageStatsLoaded(stats, db_size, counts),
                )
            }

            Message::ScanFinished(_summary) => {
                self.is_scanning = false;
                self.scan_progress = None;
                self.is_generating_thumbnails = true;
                let db = self.db.clone();
                let mut tasks = vec![
                    Task::perform(
                        async move { db.get_library_folders().await.unwrap_or_default() },
                        Message::LibraryFoldersLoaded,
                    ),
                    self.reload_after_scan_task(),
                    self.load_storage_stats_task(),
                    self.generate_thumbnails_task(),
                ];

                if !self.pending_watcher_paths.is_empty() {
                    let pending = std::mem::take(&mut self.pending_watcher_paths);
                    self.is_scanning = true;
                    let db_scan = self.db.clone();
                    tasks.push(Task::stream(iced::stream::channel(
                        100,
                        async move |mut sender| {
                            let mut cb_sender = sender.clone();
                            let summary = db_scan
                                .scan_new_files(
                                    &pending,
                                    Some(move |current, total| {
                                        let _ = cb_sender
                                            .try_send(Message::ScanProgress { current, total });
                                    }),
                                )
                                .await
                                .unwrap_or_default();
                            let _ = sender.send(Message::ScanFinished(summary)).await;
                        },
                    )));
                }

                Task::batch(tasks)
            }

            Message::ThumbnailsGenerated => {
                self.is_generating_thumbnails = false;
                let covers_task = if self.active_tab == ViewTab::Albums {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                };
                Task::batch([self.load_storage_stats_task(), covers_task])
            }

            Message::StorageStatsLoaded(stats, db_size_bytes, counts) => {
                self.cache_stats = stats;
                self.db_size_bytes = db_size_bytes;
                self.library_counts = counts;
                self.status_settings_str = format!("{} tracks indexed", counts.tracks);
                if !self.artists.is_empty() {
                    self.artists.set_total_albums(counts.albums);
                }
                Task::none()
            }

            Message::SelectAlbum(album_name) => {
                self.active_tab = ViewTab::Tracks;
                // Exact album scope so the library shows only this album,
                // instead of every track whose title/artist contains the name.
                self.library
                    .set_filter(TrackFilter::album(album_name.as_ref()));
                // Present the album in disc/track order rather than title order.
                self.library.sort_field = SortField::TrackNum;
                self.library.sort_dir = SortDirection::Ascending;
                let snap = snap_to(
                    iced::widget::Id::new("library_tracks"),
                    RelativeOffset::START,
                    Animation::Instant,
                );
                Task::batch([self.reload_tracks_task(), self.save_state_task(), snap])
            }

            Message::SelectArtist(artist_name) => {
                self.active_tab = ViewTab::Tracks;
                self.library
                    .set_filter(TrackFilter::artist(artist_name.as_ref()));
                let snap = snap_to(
                    iced::widget::Id::new("library_tracks"),
                    RelativeOffset::START,
                    Animation::Instant,
                );
                Task::batch([self.reload_tracks_task(), self.save_state_task(), snap])
            }

            // ── Playlists ───────────────────────────────────────────────────
            Message::TracksLoaded(rows, total) => {
                self.library.set_window(rows, total);
                if self.library.active_filter().is_none() {
                    self.library_counts.tracks = total;
                    self.status_settings_str = format!("{} tracks indexed", total);
                }
                self.library.recompute_rendered_rows(
                    &self.layout_manager.current_template.view_formats.library,
                );
                Task::none()
            }

            Message::LibraryScrolled(viewport) => {
                if let Some(offset) = self.library.on_scroll(&viewport) {
                    return self.load_track_window_task(offset);
                }
                Task::none()
            }

            Message::TrackWindowLoaded { offset, rows } => {
                self.library.replace_window(offset, rows);
                self.library.recompute_rendered_rows(
                    &self.layout_manager.current_template.view_formats.library,
                );
                Task::none()
            }

            Message::AlbumSortChanged(field) => {
                self.albums.toggle_sort(field);
                self.albums.recompute_rendered_window(
                    &self.layout_manager.current_template.view_formats.albums,
                );
                let save_task = self.save_state_task();
                let covers_task = self.load_visible_album_covers_task();
                Task::batch([save_task, covers_task])
            }

            Message::ArtistSortChanged(field) => {
                self.artists.toggle_sort(field);
                self.artists.recompute_rendered_window(
                    &self.layout_manager.current_template.view_formats.artists,
                );
                self.save_state_task()
            }

            Message::AlbumsLoaded(albums) => {
                self.albums.set_albums(albums);
                self.library_counts.albums = self.albums.len();
                self.albums.recompute_rendered_window(
                    &self.layout_manager.current_template.view_formats.albums,
                );
                // The catalog was (re)loaded at offset 0; keep the scrollable in
                // sync so the window and the viewport cannot disagree.
                let snap = snap_to(
                    iced::widget::Id::new("albums_scroll"),
                    RelativeOffset::START,
                    Animation::Instant,
                );
                if self.active_tab == ViewTab::Albums {
                    Task::batch([self.load_visible_album_covers_task(), snap])
                } else {
                    snap
                }
            }

            Message::AlbumsScrolled(viewport) => {
                if self.albums.on_scroll(&viewport) {
                    self.albums.recompute_rendered_window(
                        &self.layout_manager.current_template.view_formats.albums,
                    );
                    return self.load_visible_album_covers_task();
                }
                Task::none()
            }

            Message::EnsureAlbumCovers => {
                if self.active_tab != ViewTab::Albums || self.albums.is_empty() {
                    return Task::none();
                }
                // Release keys whose request never returned, then request any
                // visible cover that is still missing.
                self.albums.expire_stale_in_flight(Duration::from_secs(10));
                self.load_visible_album_covers_task()
            }

            Message::ArtistsLoaded(artists) => {
                self.artists.set_artists(artists);
                self.library_counts.artists = self.artists.len();
                if self.library_counts.albums > 0 || self.library_counts.tracks == 0 {
                    self.artists.set_total_albums(self.library_counts.albums);
                }
                self.artists.recompute_rendered_window(
                    &self.layout_manager.current_template.view_formats.artists,
                );
                snap_to(
                    iced::widget::Id::new("artists_scroll"),
                    RelativeOffset::START,
                    Animation::Instant,
                )
            }

            Message::ArtistsScrolled(viewport) => {
                if self.artists.on_scroll(&viewport) {
                    self.artists.recompute_rendered_window(
                        &self.layout_manager.current_template.view_formats.artists,
                    );
                }
                Task::none()
            }

            Message::AlbumCoverChunkLoaded {
                request_id,
                results,
                remaining,
            } => {
                self.albums.insert_covers(request_id, results);
                let remaining: Vec<_> = remaining
                    .into_iter()
                    .filter(|(k, _)| self.albums.is_in_flight(k))
                    .collect();

                // After every batch, re-scan the current window for anything
                // still missing. This guarantees the landing window converges
                // even when fast scrolling abandoned earlier cover requests.
                let visible_follow_up = self.load_visible_album_covers_task();

                if remaining.is_empty() {
                    visible_follow_up
                } else {
                    Task::batch([
                        self.load_cover_chunk_task(request_id, remaining),
                        visible_follow_up,
                    ])
                }
            }

            _ => unreachable!("routed in App::update"),
        }
    }

    pub(super) fn load_cover_chunk_task(
        &self,
        request_id: u64,
        mut pending: Vec<(String, i64)>,
    ) -> Task<Message> {
        let (chunk, remaining) = if pending.len() <= ALBUM_COVER_LOAD_CHUNK_SIZE {
            (pending, Vec::new())
        } else {
            let rest = pending.split_off(ALBUM_COVER_LOAD_CHUNK_SIZE);
            (pending, rest)
        };

        let db = self.db.clone();
        let active_in_flight = Arc::clone(&self.albums.active_in_flight);
        Task::perform(
            async move {
                // Filter out keys already cancelled before hitting SQLite.
                let active_chunk: Vec<(String, i64)> = {
                    let guard = active_in_flight.read().ok();
                    chunk
                        .into_iter()
                        .filter(|(k, _)| guard.as_ref().is_none_or(|s| s.contains(k)))
                        .collect()
                };
                if active_chunk.is_empty() {
                    return (Vec::new(), remaining);
                }

                // Resolve representative track paths for the whole chunk in a
                // single `IN (...)` query, then downscale/cache covers off the
                // async runtime.
                let ids: Vec<i64> = active_chunk.iter().map(|(_, id)| *id).collect();
                let paths = db.get_track_paths_by_ids(&ids).await.unwrap_or_default();

                let resolved: Vec<(String, Option<PathBuf>)> = active_chunk
                    .into_iter()
                    .map(|(key, id)| {
                        let path = paths.get(&id).map(PathBuf::from);
                        (key, path)
                    })
                    .collect();

                tokio::task::spawn_blocking(move || {
                    let mut results = Vec::with_capacity(resolved.len());
                    for (key, path) in resolved {
                        // Skip expensive FFmpeg/JPEG work if the user scrolled past this album.
                        let still_needed = active_in_flight
                            .read()
                            .map(|s| s.contains(&key))
                            .unwrap_or(true);
                        if !still_needed {
                            continue;
                        }
                        let bytes = path
                            .and_then(|p| thumbnail::get_or_create(&p, thumbnail::Resolution::Low))
                            .unwrap_or_default();
                        results.push((key, bytes));
                    }

                    (results, remaining)
                })
                .await
                .unwrap_or_else(|_| (Vec::new(), Vec::new()))
            },
            move |(results, remaining)| Message::AlbumCoverChunkLoaded {
                request_id,
                results,
                remaining,
            },
        )
    }

    pub(super) fn generate_thumbnails_task(&self) -> Task<Message> {
        let db = self.db.clone();
        Task::perform(
            async move {
                let albums = db.get_albums().await.unwrap_or_default();
                let cover_ids: Vec<i64> = albums.iter().filter_map(|a| a.cover_track_id).collect();
                let paths_map = db
                    .get_track_paths_by_ids(&cover_ids)
                    .await
                    .unwrap_or_default();

                tokio::task::spawn_blocking(move || {
                    let paths: Vec<PathBuf> = paths_map.values().map(PathBuf::from).collect();
                    if paths.is_empty() {
                        return;
                    }
                    let num_workers = std::thread::available_parallelism()
                        .map(|n| n.get())
                        .unwrap_or(4)
                        .clamp(1, 8)
                        .min(paths.len());
                    let chunk_size = paths.len().div_ceil(num_workers);
                    std::thread::scope(|s| {
                        for chunk in paths.chunks(chunk_size.max(1)) {
                            s.spawn(move || {
                                for path in chunk {
                                    let _ =
                                        thumbnail::get_or_create(path, thumbnail::Resolution::Low);
                                }
                            });
                        }
                    });
                })
                .await
                .ok();
            },
            |_| Message::ThumbnailsGenerated,
        )
    }

    pub(super) fn load_visible_album_covers_task(&mut self) -> Task<Message> {
        let to_load = self.albums.get_missing_covers_to_load();
        if to_load.is_empty() {
            return Task::none();
        }
        let request_id = self
            .albums
            .mark_loading(to_load.iter().map(|(k, _)| k.clone()));
        self.load_cover_chunk_task(request_id, to_load)
    }

    pub(super) fn reload_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let window_size = self.library.window_capacity();
        let filter = self.library.active_filter();
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;

        Task::perform(
            async move {
                let total = db.get_total_track_count(filter.as_ref()).await.unwrap_or(0);
                let rows = db
                    .get_track_rows_window(0, window_size, filter.as_ref(), sort_field, sort_dir)
                    .await
                    .unwrap_or_default();
                (rows, total)
            },
            |(rows, total)| Message::TracksLoaded(rows, total),
        )
    }

    pub(super) fn reload_after_scan_task(&mut self) -> Task<Message> {
        let db = self.db.clone();
        let window_size = self.library.window_capacity();
        let filter = self.library.active_filter();
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;

        let mut tasks = vec![Task::perform(
            async move {
                let total = db.get_total_track_count(filter.as_ref()).await.unwrap_or(0);
                let rows = db
                    .get_track_rows_window(0, window_size, filter.as_ref(), sort_field, sort_dir)
                    .await
                    .unwrap_or_default();
                (rows, total)
            },
            |(rows, total)| Message::TracksLoaded(rows, total),
        )];

        if self.active_tab == ViewTab::Albums {
            let db = self.db.clone();
            tasks.push(Task::perform(
                async move { db.get_albums().await.unwrap_or_default() },
                Message::AlbumsLoaded,
            ));
        } else {
            self.albums.clear();
        }

        if self.active_tab == ViewTab::Artists {
            let db = self.db.clone();
            tasks.push(Task::perform(
                async move { db.get_artists().await.unwrap_or_default() },
                Message::ArtistsLoaded,
            ));
        } else {
            self.artists.clear();
        }

        Task::batch(tasks)
    }

    pub(super) fn load_track_window_task(&self, offset: usize) -> Task<Message> {
        let db = self.db.clone();
        let limit = self.library.window_capacity();
        let filter = self.library.active_filter();
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;

        Task::perform(
            async move {
                let rows = db
                    .get_track_rows_window(offset, limit, filter.as_ref(), sort_field, sort_dir)
                    .await
                    .unwrap_or_default();
                (offset, rows)
            },
            |(offset, rows)| Message::TrackWindowLoaded { offset, rows },
        )
    }
}
