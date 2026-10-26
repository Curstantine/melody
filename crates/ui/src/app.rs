use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use audio::{AudioController, AudioEngine};
use core_engine::{
    parse_m3u, write_m3u, LibraryDatabase, PlaylistEntry, PlaylistSummary, SortDirection,
    SortField, Track, TrackFilter, TrackRow,
};
use crossbeam_channel::Receiver;
use iced::futures::SinkExt;
use iced::widget::operation::scrollable::{snap_to, RelativeOffset};
use iced::widget::operation::Animation;
use iced::widget::{button, column, container, image, mouse_area, row, Space, Widget};
use iced::{Alignment, Background, Color, Element, Length, Subscription, Task, Theme};
use plugins::scrobbler::{LastFmPlugin, ListenBrainzPlugin, ScrobbleService};
#[cfg(target_os = "linux")]
use plugins::MprisPlugin;
use plugins::{PlayerAction, PluginManager, TrackMetadata};
use tracing::{error, info, warn};

use crate::components;
use crate::config::{AppConfig, ScrobbleConfig};
use crate::icons;
use crate::layout::LayoutManager;
use crate::message::{
    Message, PlaylistItem, PlaylistOpenIntent, PlaylistResolution, PlaylistSection, SettingsTab,
    ViewTab,
};
use crate::queue::{queue_window_for_start, QueueSource, QueueStore, DEFAULT_QUEUE_NAME};
use crate::state::AppState;
use crate::theme::{styles, ThemeConfig, ThemeManager};
use crate::views::{
    AlbumsView, ArtistsView, LibraryView, NowPlayingView, PlayerBar, PlaylistsView, SettingsView,
};

/// Album covers resolved per background chunk. Bounds the `IN (...)` lookup
/// and the `spawn_blocking` downscale batch behind album/artist scrolling.
pub const ALBUM_COVER_LOAD_CHUNK_SIZE: usize = 16;

#[derive(Clone, Debug)]
pub enum DuplicateTarget {
    Playlist {
        track: TrackRow,
        playlist_id: i64,
        playlist_name: String,
    },
    Queue {
        track: TrackRow,
    },
}

#[derive(Clone, Debug)]
pub struct DuplicatePromptState {
    pub target: DuplicateTarget,
}

pub struct App {
    pub(crate) db: LibraryDatabase,
    pub(crate) db_path: PathBuf,
    pub(crate) config_path: PathBuf,
    pub(crate) state_path: PathBuf,
    pub(crate) _audio_engine: AudioEngine,
    pub(crate) audio_controller: AudioController,
    pub(crate) plugin_manager: PluginManager,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) layout_manager: LayoutManager,

    // Navigation & Tabs
    pub(crate) active_tab: ViewTab,
    pub(crate) tab_order: Vec<ViewTab>,
    pub(crate) dragging_tab: Option<ViewTab>,
    pub(crate) hovered_tab: Option<ViewTab>,
    pub(crate) drag_order_changed: bool,
    /// Queue track being drag-reordered: `(queue_id, current_index)`.
    pub(crate) dragging_queue_track: Option<(u64, usize)>,
    pub(crate) drag_queue_order_changed: bool,
    /// Open-playlist row being drag-reordered: `(playlist_id, index)`.
    pub(crate) dragging_detail: Option<(i64, usize)>,
    pub(crate) drag_detail_order_changed: bool,

    // Screen-specific state components
    pub(crate) library: LibraryView,
    pub(crate) albums: AlbumsView,
    pub(crate) artists: ArtistsView,
    pub(crate) playlists: PlaylistsView,
    pub(crate) now_playing: NowPlayingView,
    pub(crate) settings: SettingsView,
    pub(crate) player_bar: PlayerBar,

    // Playback queues (in-memory view, persisted to the database)
    pub(crate) queues: QueueStore,

    // Playback State
    pub(crate) current_track: Option<Track>,
    pub(crate) current_track_id: Option<i64>,
    pub(crate) current_cover: Option<image::Handle>,
    pub(crate) current_blurred_cover: Option<image::Handle>,
    pub(crate) current_next_cover: Option<image::Handle>,
    pub(crate) current_tech_info: Option<core_engine::AudioTechnicalInfo>,
    pub(crate) is_playing: bool,
    pub(crate) position_ms: u64,
    pub(crate) duration_ms: u64,
    pub(crate) volume: f32,
    pub(crate) is_shuffle: bool,
    pub(crate) is_repeat: bool,
    pub(crate) gapless: bool,
    pub(crate) pending_gapless_next: Option<TrackRow>,
    pub(crate) background_mode: bool,
    pub(crate) background_mode_timeout_secs: u64,
    pub(crate) is_background_mode: bool,
    pub(crate) is_minimized: bool,
    pub(crate) minimized_at: Option<std::time::Instant>,
    pub(crate) window_id: Option<iced::window::Id>,

    // Global Scanning Telemetry
    pub(crate) is_scanning: bool,
    pub(crate) scan_progress: Option<(usize, usize)>,
    pub(crate) is_generating_thumbnails: bool,

    // Library Watcher and Directory Monitoring
    pub(crate) watch_library: bool,
    pub(crate) library_watcher: Option<core_engine::LibraryWatcher>,
    pub(crate) watcher_rx: Option<crossbeam_channel::Receiver<Vec<PathBuf>>>,
    pub(crate) pending_watcher_paths: Vec<PathBuf>,

    // Scrobbling Plugins
    pub(crate) listenbrainz_plugin: Arc<ListenBrainzPlugin>,
    pub(crate) lastfm_plugin: Arc<LastFmPlugin>,
    pub(crate) scrobble_config: ScrobbleConfig,

    pub(crate) config_save_seq: u64,
    pub(crate) state_save_seq: u64,
    pub(crate) search_seq: u64,

    pub(crate) cache_stats: core_engine::CacheStats,
    pub(crate) db_size_bytes: u64,

    pub(crate) mpris_rx: Option<Receiver<PlayerAction>>,

    pub(crate) context_menu: Option<components::ContextMenuState>,
    pub(crate) duplicate_prompt: Option<DuplicatePromptState>,
    pub(crate) window_size: iced::Size,
    pub(crate) pending_playlist_tracks: Vec<TrackRow>,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        info!("Initializing Melody Native Player application");

        let db_path = get_database_path();
        info!("Using SQLite database at: {:?}", db_path);

        let db = tokio::runtime::Handle::current().block_on(async {
            LibraryDatabase::new(&db_path).await.unwrap_or_else(|e| {
                error!("Failed to open disk database, falling back to in-memory: {e}");
                tokio::runtime::Handle::current()
                    .block_on(async { LibraryDatabase::in_memory().await })
                    .expect("In-memory database failed")
            })
        });

        let config = AppConfig::load();
        let state = AppState::load();

        let audio_engine = AudioEngine::new();
        let audio_controller = audio_engine.controller();
        audio_controller.set_volume(state.volume);

        let plugin_manager = PluginManager::new();

        let listenbrainz_plugin = Arc::new(ListenBrainzPlugin::new(
            config.scrobble.to_listenbrainz_config(),
        ));
        let lastfm_plugin = Arc::new(LastFmPlugin::new(config.scrobble.to_lastfm_config()));
        plugin_manager.register_plugin(listenbrainz_plugin.clone());
        plugin_manager.register_plugin(lastfm_plugin.clone());

        #[cfg(target_os = "linux")]
        let (mpris_plugin, mpris_rx) = {
            let (tx, rx) = crossbeam_channel::unbounded();
            (std::sync::Arc::new(MprisPlugin::new(tx)), Some(rx))
        };
        #[cfg(target_os = "linux")]
        plugin_manager.register_plugin(mpris_plugin);

        #[cfg(not(target_os = "linux"))]
        let mpris_rx = None;

        let mut theme_manager = ThemeManager::new();
        theme_manager.set_theme(&config.theme_name);
        if let Some(ref font) = config.primary_font {
            theme_manager.set_font(Some(font.clone()));
        }

        let mut layout_manager = LayoutManager::new();
        layout_manager.select_layout(&config.layout_name);

        let config_path = AppConfig::config_path();
        let state_path = AppState::state_path();

        let library = LibraryView::new(config.page_size);
        let albums = AlbumsView::with_tile_width(
            state.albums_view_mode,
            state.album_sort_field,
            config.album_tile_width,
        );
        let artists = ArtistsView::with_tile_width(
            state.artists_view_mode,
            state.artist_sort_field,
            config.artist_tile_width,
        );
        let now_playing = NowPlayingView::with_cover_size(config.now_playing_cover_size);
        let playlists = PlaylistsView::new(state.playlists_view_mode, state.playlist_sort_field);
        let settings = SettingsView::new(state.settings_tab, Vec::new());
        let player_bar = PlayerBar::new();

        let mut queues = QueueStore::default();
        let (queue_records, active_queue_id) = tokio::runtime::Handle::current().block_on(async {
            let records = db.list_queues().await.unwrap_or_default();
            let active = db.get_active_queue_id().await.unwrap_or(None);
            (records, active)
        });
        queues.load_summaries(queue_records);

        let active_id = state
            .queue_id
            .filter(|&id| queues.summaries.iter().any(|s| s.id == id))
            .or_else(|| active_queue_id.map(|id| id as u64))
            .filter(|&id| queues.summaries.iter().any(|s| s.id == id));
        queues.set_active_id(active_id);
        if let (Some(active_id), Some(seq)) = (active_id, state.playing_seq) {
            if let Some(summary) = queues.summaries.iter_mut().find(|s| s.id == active_id) {
                summary.current = seq;
            }
        }

        let mut app = Self {
            db,
            db_path,
            config_path,
            state_path,
            _audio_engine: audio_engine,
            audio_controller,
            plugin_manager,
            theme_manager,
            layout_manager,

            active_tab: state.active_tab,
            tab_order: config.tab_order.clone(),
            dragging_tab: None,
            hovered_tab: None,
            drag_order_changed: false,
            dragging_queue_track: None,
            drag_queue_order_changed: false,
            dragging_detail: None,
            drag_detail_order_changed: false,

            library,
            albums,
            artists,
            playlists,
            now_playing,
            settings,
            player_bar,

            queues,

            current_track: None,
            current_track_id: None,
            current_cover: None,
            current_blurred_cover: None,
            current_next_cover: None,
            current_tech_info: None,
            is_playing: false,
            position_ms: 0,
            duration_ms: 0,
            volume: state.volume,
            is_shuffle: state.is_shuffle,
            is_repeat: state.is_repeat,
            gapless: config.gapless,
            pending_gapless_next: None,
            background_mode: config.background_mode,
            background_mode_timeout_secs: config.background_mode_timeout_secs,
            is_background_mode: false,
            is_minimized: false,
            minimized_at: None,
            window_id: None,

            is_scanning: false,
            scan_progress: None,
            is_generating_thumbnails: false,

            watch_library: config.watch_library,
            library_watcher: None,
            watcher_rx: None,
            pending_watcher_paths: Vec::new(),

            listenbrainz_plugin,
            lastfm_plugin,
            scrobble_config: config.scrobble,
            config_save_seq: 0,
            state_save_seq: 0,
            search_seq: 0,
            cache_stats: core_engine::CacheStats::default(),
            db_size_bytes: 0,

            mpris_rx,

            context_menu: None,
            duplicate_prompt: None,
            window_size: {
                let (w, h) = state.window_size().unwrap_or((1130.0, 740.0));
                iced::Size::new(w.max(1000.0), h.max(520.0))
            },
            pending_playlist_tracks: Vec::new(),
        };

        let mut initial_tasks = vec![
            app.reload_tracks_task(),
            {
                let db = app.db.clone();
                Task::perform(
                    async move { db.get_library_folders().await.unwrap_or_default() },
                    Message::LibraryFoldersLoaded,
                )
            },
            {
                let db = app.db.clone();
                Task::perform(
                    async move { db.get_playlists().await.unwrap_or_default() },
                    Message::PlaylistsLoaded,
                )
            },
        ];

        if app.active_tab == ViewTab::Albums {
            let db = app.db.clone();
            initial_tasks.push(Task::perform(
                async move { db.get_albums().await.unwrap_or_default() },
                Message::AlbumsLoaded,
            ));
        } else if app.active_tab == ViewTab::Artists {
            let db = app.db.clone();
            initial_tasks.push(Task::perform(
                async move { db.get_artists().await.unwrap_or_default() },
                Message::ArtistsLoaded,
            ));
        } else if app.active_tab == ViewTab::Settings
            && app.settings.active_tab == SettingsTab::Appearance
        {
            if let Some(task) = app.ensure_fonts_task() {
                initial_tasks.push(task);
            }
        }

        if let Some(active_id) = active_id {
            let db = app.db.clone();
            initial_tasks.push(Task::perform(
                async move {
                    let tracks = db
                        .load_queue_tracks(active_id as i64)
                        .await
                        .unwrap_or_default();
                    (active_id as i64, tracks)
                },
                |(id, tracks)| Message::QueueActivated {
                    id: id as u64,
                    tracks,
                },
            ));
        }

        let initial_task = Task::batch(initial_tasks);

        (app, initial_task)
    }

    pub fn title(&self) -> String {
        match &self.current_track {
            Some(track) => format!("{} - {} | Melody", track.title, track.artist),
            None => "Melody".to_string(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme_manager.theme()
    }

    pub(crate) fn cursor_position(&self) -> iced::Point {
        LAST_CURSOR_POS
            .lock()
            .map(|p| *p)
            .unwrap_or(iced::Point::new(100.0, 100.0))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.is_background_mode {
            // Slower ticker in background mode: 250ms is plenty for gapless transition and track ended checks,
            // while cutting timer wakes by 80% (from 20 wakes/sec to 4 wakes/sec).
            let ticker = iced::time::every(Duration::from_millis(250)).map(|_| Message::AudioTick);
            let window_events = iced::event::listen_with(handle_cursor_and_window_event);
            return Subscription::batch([ticker, window_events]);
        }

        // Poll audio playback status at 50ms intervals (~20 FPS smooth seekbar)
        let ticker = iced::time::every(Duration::from_millis(50)).map(|_| Message::AudioTick);
        let mut subscriptions = vec![ticker];

        // Periodic self-healing pass for the album cover window.
        if self.active_tab == ViewTab::Albums && !self.albums.is_empty() {
            subscriptions.push(
                iced::time::every(Duration::from_millis(400)).map(|_| Message::EnsureAlbumCovers),
            );
        }
        if self.dragging_tab.is_some()
            || self.dragging_queue_track.is_some()
            || self.dragging_detail.is_some()
        {
            subscriptions.push(iced::event::listen_with(handle_drag_event));
        }

        subscriptions.push(iced::event::listen_with(handle_cursor_and_window_event));

        if self.context_menu.is_some() || self.duplicate_prompt.is_some() {
            subscriptions.push(iced::event::listen_with(handle_escape_key));
        }

        Subscription::batch(subscriptions)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AudioTick => {
                if self.background_mode && self.is_minimized && !self.is_background_mode {
                    if let Some(min_at) = self.minimized_at {
                        if min_at.elapsed()
                            >= Duration::from_secs(self.background_mode_timeout_secs)
                        {
                            return self.enter_background_mode();
                        }
                    }
                }

                let status = self.audio_controller.status();
                let was_playing = self.is_playing;
                self.is_playing = status.is_playing();

                if was_playing != self.is_playing {
                    self.plugin_manager
                        .on_playback_status_change(self.is_playing);
                }

                if status.duration_ms() > 0 {
                    self.duration_ms = status.duration_ms();
                }

                if !self.player_bar.is_seeking {
                    let old_pos = self.position_ms;
                    let raw_pos = status.position_ms();
                    self.position_ms = if self.duration_ms > 0 {
                        raw_pos.min(self.duration_ms)
                    } else {
                        raw_pos
                    };
                    if self.is_playing && self.position_ms != old_pos {
                        self.plugin_manager.on_position_change(self.position_ms);
                    }
                }

                // Check for gapless track transition
                if status.take_gapless_transition() {
                    info!("Gapless transition occurred");
                    self.position_ms = 0;
                    if status.duration_ms() > 0 {
                        self.duration_ms = status.duration_ms();
                    }
                    self.plugin_manager.on_playback_end();

                    if self.is_repeat {
                        if let Some(track) = &self.current_track {
                            self.plugin_manager.on_track_change(&TrackMetadata {
                                title: track.title.clone(),
                                artist: track.artist.clone(),
                                album: track.album.clone(),
                                duration_ms: track.duration_ms,
                                file_path: track.file_path.clone(),
                            });
                        }
                        self.queue_next_gapless_track();
                        return Task::none();
                    }

                    if let Some(next_track) = self.pending_gapless_next.take() {
                        if let Some(queue) = self.queues.active_queue_mut() {
                            if let Some(pos) =
                                queue.tracks.iter().position(|t| t.id == next_track.id)
                            {
                                queue.set_current(pos);
                            }
                            self.queues.touch_active_summary();
                        }

                        let track_id = next_track.id;
                        let duration_ms = next_track.duration_ms;
                        let file_path = next_track.file_path;
                        let title = next_track.title;
                        let artist = next_track.artist.to_string();
                        let album = next_track.album.to_string();

                        self.current_cover = None;
                        self.current_blurred_cover = None;
                        self.current_next_cover = None;
                        self.current_tech_info = None;
                        self.now_playing.set_lyrics(None);

                        self.plugin_manager.on_track_change(&TrackMetadata {
                            title: title.clone(),
                            artist: artist.clone(),
                            album: album.clone(),
                            duration_ms,
                            file_path: file_path.clone(),
                        });

                        let path = PathBuf::from(&file_path);
                        let load_assets = self.load_track_assets_task(
                            track_id,
                            path,
                            artist.clone(),
                            title.clone(),
                            duration_ms,
                        );

                        let track = Track {
                            id: track_id,
                            file_path,
                            title,
                            artist,
                            album_artist: None,
                            album,
                            genre: None,
                            duration_ms,
                            track_num: None,
                            disc_num: None,
                            year: None,
                            composer: None,
                            label: None,
                            musicbrainz_track_id: None,
                            musicbrainz_artist_id: None,
                            musicbrainz_album_id: None,
                            added_timestamp: 0,
                        };

                        self.current_track = Some(track);
                        self.player_bar.update_track(self.current_track.as_ref());
                        self.current_track_id = Some(track_id);
                        self.duration_ms = duration_ms;

                        let persist = self.persist_active_current_task();
                        let snap = self.snap_queue_to_current();
                        let db = self.db.clone();
                        let fetch_details = Task::perform(
                            async move { db.get_track_by_id(track_id).await.unwrap_or(None) },
                            |res| Message::TrackDetailsLoaded(Box::new(res)),
                        );

                        self.queue_next_gapless_track();
                        let save_state = self.save_state_debounced();

                        return Task::batch([
                            load_assets,
                            persist,
                            snap,
                            fetch_details,
                            save_state,
                        ]);
                    } else {
                        return self.update(Message::NextTrack);
                    }
                }

                // Check if track ended
                if status.take_ended() {
                    info!("Playback ended, triggering next track");
                    if self.duration_ms > 0 {
                        self.position_ms = self.duration_ms;
                    }
                    self.plugin_manager.on_playback_end();
                    return self.update(Message::NextTrack);
                }

                // Drain any incoming MPRIS actions
                let mut mpris_action = None;
                if let Some(ref rx) = self.mpris_rx {
                    while let Ok(action) = rx.try_recv() {
                        mpris_action = Some(action);
                    }
                }
                if let Some(action) = mpris_action {
                    return self.update(Message::MprisAction(action));
                }

                // Drain any incoming library watcher events
                let mut watcher_paths = Vec::new();
                if let Some(ref rx) = self.watcher_rx {
                    while let Ok(paths) = rx.try_recv() {
                        watcher_paths.extend(paths);
                    }
                }
                if !watcher_paths.is_empty() {
                    if self.is_scanning {
                        self.pending_watcher_paths.extend(watcher_paths);
                    } else {
                        self.is_scanning = true;
                        self.scan_progress = None;
                        let db = self.db.clone();
                        let paths = watcher_paths;
                        return Task::stream(iced::stream::channel(
                            100,
                            async move |mut sender| {
                                let mut cb_sender = sender.clone();
                                let summary = db
                                    .scan_new_files(
                                        &paths,
                                        Some(move |current, total| {
                                            let _ = cb_sender
                                                .try_send(Message::ScanProgress { current, total });
                                        }),
                                    )
                                    .await
                                    .unwrap_or_default();
                                let _ = sender.send(Message::ScanFinished(summary)).await;
                            },
                        ));
                    }
                }

                Task::none()
            }

            Message::PlayTrack(track_id) => {
                let db = self.db.clone();
                Task::perform(
                    async move { db.get_track_by_id(track_id).await.unwrap_or(None) },
                    |res| Message::TrackResolvedForPlayback(Box::new(res)),
                )
            }

            Message::TrackResolvedForPlayback(maybe_track) => {
                if let Some(track) = *maybe_track {
                    info!("Playing track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    // Start playback immediately; the expensive visual/metadata
                    // work is dispatched to a blocking worker below so the UI
                    // thread never stalls on disk I/O, image decoding or FFmpeg.
                    self.audio_controller.load(path.clone(), true);
                    let track_id = track.id;
                    let duration_ms = track.duration_ms;
                    let title = track.title.clone();
                    let artist = track.artist.clone();
                    let album = track.album.clone();
                    let file_path = track.file_path.clone();

                    self.current_track = Some(track);
                    self.player_bar.update_track(self.current_track.as_ref());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = duration_ms;

                    // Clear stale assets from the previous track right away so we
                    // never render a mismatched cover while the new one loads.
                    self.current_cover = None;
                    self.current_blurred_cover = None;
                    self.current_next_cover = None;
                    self.current_tech_info = None;
                    self.now_playing.set_lyrics(None);

                    // Notify plugins synchronously (cheap state updates).
                    self.plugin_manager.on_track_change(&TrackMetadata {
                        title: title.clone(),
                        artist: artist.clone(),
                        album,
                        duration_ms,
                        file_path,
                    });

                    self.queue_next_gapless_track();
                    let load_assets =
                        self.load_track_assets_task(track_id, path, artist, title, duration_ms);
                    let save_state = self.save_state_debounced();
                    Task::batch([load_assets, save_state])
                } else {
                    Task::none()
                }
            }

            Message::TrackResolvedForRestore(maybe_track) => {
                if let Some(track) = *maybe_track {
                    info!("Restoring player track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    self.audio_controller.load(path.clone(), false);
                    let track_id = track.id;
                    let duration_ms = track.duration_ms;
                    let title = track.title.clone();
                    let artist = track.artist.clone();

                    self.current_track = Some(track);
                    self.player_bar.update_track(self.current_track.as_ref());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = duration_ms;

                    self.current_cover = None;
                    self.current_blurred_cover = None;
                    self.current_next_cover = None;
                    self.current_tech_info = None;
                    self.now_playing.set_lyrics(None);

                    self.queue_next_gapless_track();
                    self.load_track_assets_task(track_id, path, artist, title, duration_ms)
                } else {
                    Task::none()
                }
            }

            Message::TrackDetailsLoaded(maybe_track) => {
                if let Some(track) = *maybe_track {
                    if self.current_track_id == Some(track.id) {
                        self.current_track = Some(track);
                        self.player_bar.update_track(self.current_track.as_ref());
                    }
                }
                Task::none()
            }

            Message::PlaybackAssetsLoaded {
                track_id,
                cover,
                blurred,
                next_cover,
                tech_info,
                lyrics,
            } => {
                // Drop assets that belong to a track the user has since skipped.
                if self.current_track_id != Some(track_id) {
                    return Task::none();
                }

                self.current_cover = cover.map(image::Handle::from_bytes);
                self.current_blurred_cover = blurred.map(image::Handle::from_bytes);
                self.current_next_cover = next_cover.map(image::Handle::from_bytes);
                self.current_tech_info = tech_info;
                self.now_playing.set_lyrics(lyrics);
                Task::none()
            }

            Message::TogglePlayPause => {
                self.audio_controller.toggle_play_pause();
                Task::none()
            }

            Message::Stop => {
                self.pending_gapless_next = None;
                self.audio_controller.stop();
                self.position_ms = 0;
                Task::none()
            }

            Message::NextTrack => {
                if self.is_repeat {
                    self.audio_controller.seek(Duration::from_millis(0));
                    self.position_ms = 0;
                    self.audio_controller.play();
                    return Task::none();
                }

                if self.queues.active.is_some() {
                    if let Some(id) = self.step_active_queue(true) {
                        self.queues.touch_active_summary();
                        let play = self.update(Message::PlayTrack(id));
                        return Task::batch([
                            play,
                            self.persist_active_current_task(),
                            self.snap_queue_to_current(),
                        ]);
                    }
                }

                if let Some(next) = self.get_next_track() {
                    return self.update(Message::PlayTrack(next.id));
                }
                Task::none()
            }

            Message::ToggleShuffle => {
                self.is_shuffle = !self.is_shuffle;
                info!("Shuffle toggled: {}", self.is_shuffle);
                self.plugin_manager.on_shuffle_change(self.is_shuffle);
                self.queue_next_gapless_track();
                self.save_state_task()
            }

            Message::ToggleRepeat => {
                self.is_repeat = !self.is_repeat;
                info!("Repeat toggled: {}", self.is_repeat);
                self.plugin_manager.on_repeat_change(self.is_repeat);
                self.queue_next_gapless_track();
                self.save_state_task()
            }

            Message::ToggleGapless(enabled) => {
                self.gapless = enabled;
                info!("Gapless playback toggled: {}", self.gapless);
                self.queue_next_gapless_track();
                self.save_config_task()
            }

            Message::PreviousTrack => {
                // If more than 3 seconds in, restart track
                if self.position_ms > 3000 {
                    self.audio_controller.seek(Duration::from_millis(0));
                    self.position_ms = 0;
                    return Task::none();
                }

                if self.queues.active.is_some() {
                    if let Some(id) = self.step_active_queue(false) {
                        self.queues.touch_active_summary();
                        let play = self.update(Message::PlayTrack(id));
                        return Task::batch([
                            play,
                            self.persist_active_current_task(),
                            self.snap_queue_to_current(),
                        ]);
                    }
                }

                if let Some(cur_id) = self.current_track_id {
                    if let Some(prev) = self.library.get_previous_row(cur_id) {
                        return self.update(Message::PlayTrack(prev.id));
                    }
                }
                Task::none()
            }

            Message::Seek(ms) => {
                let clamped = if self.duration_ms > 0 {
                    ms.min(self.duration_ms as f32)
                } else {
                    ms
                };
                self.player_bar.on_seek(clamped);
                Task::none()
            }

            Message::SeekFinished => {
                let seek_pos = self.player_bar.on_seek_finished();
                let clamped_pos = if self.duration_ms > 0 {
                    seek_pos.min(self.duration_ms)
                } else {
                    seek_pos
                };
                self.audio_controller
                    .seek(Duration::from_millis(clamped_pos));
                self.position_ms = clamped_pos;
                self.plugin_manager.on_seeked(self.position_ms);
                Task::none()
            }

            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.audio_controller.set_volume(vol);
                self.plugin_manager.on_volume_change(vol);
                self.save_state_debounced()
            }

            Message::MprisAction(action) => match action {
                PlayerAction::Play => {
                    self.audio_controller.play();
                    Task::none()
                }
                PlayerAction::Pause => {
                    self.audio_controller.pause();
                    Task::none()
                }
                PlayerAction::TogglePlayPause => self.update(Message::TogglePlayPause),
                PlayerAction::Stop => self.update(Message::Stop),
                PlayerAction::Next => self.update(Message::NextTrack),
                PlayerAction::Previous => self.update(Message::PreviousTrack),
                PlayerAction::Seek(offset) => {
                    let current = Duration::from_millis(self.position_ms);
                    let target = current.saturating_add(offset);
                    let target_ms = target.as_millis().min(self.duration_ms as u128) as u64;
                    self.audio_controller.seek(Duration::from_millis(target_ms));
                    self.position_ms = target_ms;
                    self.plugin_manager.on_seeked(target_ms);
                    Task::none()
                }
                PlayerAction::SetPosition(pos) => {
                    let target_ms = pos.as_millis().min(self.duration_ms as u128) as u64;
                    self.audio_controller.seek(Duration::from_millis(target_ms));
                    self.position_ms = target_ms;
                    self.plugin_manager.on_seeked(target_ms);
                    Task::none()
                }
                PlayerAction::SetVolume(vol) => self.update(Message::VolumeChanged(vol)),
                PlayerAction::SetShuffle(shuffle) => {
                    if self.is_shuffle != shuffle {
                        self.update(Message::ToggleShuffle)
                    } else {
                        Task::none()
                    }
                }
                PlayerAction::SetRepeat(repeat) => {
                    if self.is_repeat != repeat {
                        self.update(Message::ToggleRepeat)
                    } else {
                        Task::none()
                    }
                }
                PlayerAction::OpenUri(uri) => {
                    if let Ok(url) = url::Url::parse(&uri) {
                        if url.scheme() == "file" {
                            if let Ok(path) = url.to_file_path() {
                                let path_str = path.to_string_lossy().to_string();
                                let db = self.db.clone();
                                return Task::perform(
                                    async move { db.get_track_by_path(&path_str).await.unwrap_or(None) },
                                    |res| Message::TrackResolvedForPlayback(Box::new(res)),
                                );
                            }
                        }
                    }
                    Task::none()
                }
                PlayerAction::Raise => {
                    info!("MPRIS Raise requested");
                    Task::none()
                }
                PlayerAction::Quit => {
                    info!("MPRIS Quit requested");
                    iced::window::latest().and_then(iced::window::close)
                }
            },

            Message::SelectTab(tab) => {
                self.active_tab = tab;
                let save_task = self.save_state_task();
                match tab {
                    ViewTab::Albums => {
                        let load_task = if self.albums.is_empty() {
                            let db = self.db.clone();
                            Task::perform(
                                async move { db.get_albums().await.unwrap_or_default() },
                                Message::AlbumsLoaded,
                            )
                        } else {
                            self.load_visible_album_covers_task()
                        };
                        Task::batch([save_task, load_task])
                    }
                    ViewTab::Artists => {
                        self.albums.clear_covers();
                        if self.artists.is_empty() {
                            let db = self.db.clone();
                            let load_task = Task::perform(
                                async move { db.get_artists().await.unwrap_or_default() },
                                Message::ArtistsLoaded,
                            );
                            Task::batch([save_task, load_task])
                        } else {
                            save_task
                        }
                    }
                    ViewTab::Settings => {
                        let mut tasks = vec![save_task, self.load_storage_stats_task()];
                        if self.settings.active_tab == SettingsTab::Appearance {
                            if let Some(font_task) = self.ensure_fonts_task() {
                                tasks.push(font_task);
                            }
                        }
                        Task::batch(tasks)
                    }
                    ViewTab::Playlists => {
                        self.albums.clear_covers();
                        let db = self.db.clone();
                        let refresh = Task::perform(
                            async move { db.get_playlists().await.unwrap_or_default() },
                            Message::PlaylistsLoaded,
                        );
                        Task::batch([save_task, refresh])
                    }
                    _ => {
                        self.albums.clear_covers();
                        save_task
                    }
                }
            }

            Message::TabDragStarted(tab) => {
                self.dragging_tab = Some(tab);
                self.drag_order_changed = false;
                if self.active_tab != tab {
                    self.update(Message::SelectTab(tab))
                } else {
                    Task::none()
                }
            }

            Message::TabDraggedOver(target_tab) => {
                if let Some(dragged) = self.dragging_tab {
                    if dragged != target_tab {
                        if let (Some(drag_idx), Some(target_idx)) = (
                            self.tab_order.iter().position(|&t| t == dragged),
                            self.tab_order.iter().position(|&t| t == target_tab),
                        ) {
                            self.tab_order.remove(drag_idx);
                            self.tab_order.insert(target_idx, dragged);
                            self.drag_order_changed = true;
                        }
                    }
                }
                Task::none()
            }

            Message::TabDragReleased => {
                let queue_task = self.finish_queue_drag_task();
                let detail_task = self.finish_detail_drag_task();
                let had_drag = self.dragging_tab.is_some();
                let order_changed = self.drag_order_changed;
                self.dragging_tab = None;
                self.drag_order_changed = false;
                if had_drag && order_changed {
                    Task::batch([self.save_config_task(), queue_task, detail_task])
                } else {
                    Task::batch([queue_task, detail_task])
                }
            }

            Message::TabHovered(hovered) => {
                self.hovered_tab = hovered;
                Task::none()
            }

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
                    self.reload_tracks_task()
                } else {
                    Task::none()
                }
            }

            Message::SortChanged(field) => {
                self.library.toggle_sort(field);
                self.reload_tracks_task()
            }

            Message::SetPageSize(size) => {
                self.library.set_page_size(size);
                Task::batch([self.save_config_task(), self.reload_tracks_task()])
            }

            Message::SetAlbumsViewMode(mode) => {
                let changed = self.albums.set_view_mode(mode);
                let covers = if changed {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                };
                Task::batch([self.save_state_task(), covers])
            }

            Message::SetArtistsViewMode(mode) => {
                self.artists.set_view_mode(mode);
                self.save_state_task()
            }

            Message::SetAlbumTileWidth(width) => {
                let changed = self.albums.set_tile_width(width);
                let covers = if changed {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                };
                Task::batch([self.save_config_debounced(), covers])
            }

            Message::SetArtistTileWidth(width) => {
                self.artists.set_tile_width(width);
                self.save_config_debounced()
            }

            Message::SetNowPlayingCoverSize(size) => {
                self.now_playing.set_cover_size(size);
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

            Message::WindowResized(size) => {
                if size.width <= 0.0 || size.height <= 0.0 {
                    self.is_minimized = true;
                    if self.minimized_at.is_none() {
                        self.minimized_at = Some(std::time::Instant::now());
                    }
                    Task::none()
                } else {
                    self.window_size = size;
                    self.is_minimized = false;
                    self.minimized_at = None;
                    let restore_task = if self.is_background_mode {
                        self.exit_background_mode()
                    } else {
                        Task::none()
                    };
                    Task::batch([restore_task, self.save_state_debounced()])
                }
            }

            Message::WindowUnfocused(id) => {
                self.window_id = Some(id);
                Task::perform(iced::window::is_minimized(id), move |res| {
                    Message::WindowMinimizedStatus(id, res)
                })
            }

            Message::WindowFocused(id) => {
                self.window_id = Some(id);
                self.is_minimized = false;
                self.minimized_at = None;
                if self.is_background_mode {
                    self.exit_background_mode()
                } else {
                    Task::none()
                }
            }

            Message::WindowMinimizedStatus(id, maybe_minimized) => {
                self.window_id = Some(id);
                if maybe_minimized == Some(true) {
                    self.is_minimized = true;
                    if self.minimized_at.is_none() {
                        self.minimized_at = Some(std::time::Instant::now());
                    }
                }
                Task::none()
            }

            Message::ToggleBackgroundMode(enabled) => {
                self.background_mode = enabled;
                if !enabled && self.is_background_mode {
                    Task::batch([self.exit_background_mode(), self.save_config_task()])
                } else {
                    self.save_config_task()
                }
            }

            Message::SetBackgroundModeTimeout(secs) => {
                self.background_mode_timeout_secs = secs;
                self.save_config_task()
            }

            Message::OpenTrackContextMenu { track_id, context } => {
                let position = self.cursor_position();
                if let Some(track) = self.find_track_row(track_id) {
                    self.context_menu = Some(components::ContextMenuState {
                        position,
                        target: components::ContextMenuTarget::Track { track, context },
                        submenu: None,
                    });
                    Task::none()
                } else {
                    let db = self.db.clone();
                    Task::perform(
                        async move {
                            db.get_track_rows_by_ids(&[track_id])
                                .await
                                .ok()
                                .and_then(|mut r| r.remove(&track_id))
                        },
                        move |maybe_track| {
                            if let Some(track) = maybe_track {
                                Message::TrackContextMenuReady {
                                    track,
                                    context,
                                    position,
                                }
                            } else {
                                Message::CloseContextMenu
                            }
                        },
                    )
                }
            }

            Message::TrackContextMenuReady {
                track,
                context,
                position,
            } => {
                self.context_menu = Some(components::ContextMenuState {
                    position,
                    target: components::ContextMenuTarget::Track { track, context },
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenAlbumContextMenu(album) => {
                let position = self.cursor_position();
                self.context_menu = Some(components::ContextMenuState {
                    position,
                    target: components::ContextMenuTarget::Album(album),
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenArtistContextMenu(artist) => {
                let position = self.cursor_position();
                self.context_menu = Some(components::ContextMenuState {
                    position,
                    target: components::ContextMenuTarget::Artist(artist),
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenPlaylistContextMenu(playlist) => {
                let position = self.cursor_position();
                self.context_menu = Some(components::ContextMenuState {
                    position,
                    target: components::ContextMenuTarget::Playlist(playlist),
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenQueueContextMenu { id, name } => {
                let position = self.cursor_position();
                self.context_menu = Some(components::ContextMenuState {
                    position,
                    target: components::ContextMenuTarget::Queue { id, name },
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenPlayerBarContextMenu => {
                if let Some(ref track) = self.current_track {
                    let track_row = TrackRow::from(track);
                    let position = self.cursor_position();
                    self.context_menu = Some(components::ContextMenuState {
                        position,
                        target: components::ContextMenuTarget::Track {
                            track: track_row,
                            context: components::TrackMenuContext::PlayerBar,
                        },
                        submenu: None,
                    });
                }
                Task::none()
            }

            Message::CloseContextMenu => {
                self.context_menu = None;
                Task::none()
            }

            Message::ToggleContextMenuSubmenu(sub) => {
                if let Some(ref mut menu) = self.context_menu {
                    menu.submenu = sub;
                }
                Task::none()
            }

            Message::ContextPlayTrack(id) => {
                self.context_menu = None;
                self.start_library_playback(id)
            }

            Message::ContextPlayNext(track) => {
                self.context_menu = None;
                if self.queues.active.is_none() {
                    self.queues.create(DEFAULT_QUEUE_NAME);
                }
                if let Some(queue) = self.queues.active_queue_mut() {
                    queue.insert_next(track);
                }
                self.queues.touch_active_summary();
                self.persist_active_queue_task()
            }

            Message::ContextAddToQueue(track) => {
                self.context_menu = None;
                if self.queue_contains_track(&track) {
                    self.duplicate_prompt = Some(DuplicatePromptState {
                        target: DuplicateTarget::Queue { track },
                    });
                    Task::none()
                } else {
                    self.execute_add_track_to_queue(track)
                }
            }

            Message::ContextAddTracksToQueue(tracks) => {
                self.context_menu = None;
                if !tracks.is_empty() {
                    if self.queues.active.is_none() {
                        self.queues.create(DEFAULT_QUEUE_NAME);
                    }
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.append_many(tracks);
                    }
                    self.queues.touch_active_summary();
                    return self.persist_active_queue_task();
                }
                Task::none()
            }

            Message::ContextPlayTracksNext(tracks) => {
                self.context_menu = None;
                if !tracks.is_empty() {
                    if self.queues.active.is_none() {
                        self.queues.create(DEFAULT_QUEUE_NAME);
                    }
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.insert_many_next(tracks);
                    }
                    self.queues.touch_active_summary();
                    return self.persist_active_queue_task();
                }
                Task::none()
            }

            Message::ContextPlayAlbumNext(album) => {
                self.context_menu = None;
                let db = self.db.clone();
                let filter = TrackFilter::album(album.album.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::ContextPlayTracksNext,
                )
            }

            Message::ContextAddAlbumToQueue(album) => {
                self.context_menu = None;
                let db = self.db.clone();
                let filter = TrackFilter::album(album.album.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::ContextAddTracksToQueue,
                )
            }

            Message::ContextPlayArtistNext(artist) => {
                self.context_menu = None;
                let db = self.db.clone();
                let filter = TrackFilter::artist(artist.artist.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::ContextPlayTracksNext,
                )
            }

            Message::ContextAddArtistToQueue(artist) => {
                self.context_menu = None;
                let db = self.db.clone();
                let filter = TrackFilter::artist(artist.artist.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::ContextAddTracksToQueue,
                )
            }

            Message::ContextAddTrackToPlaylist { track, playlist_id } => {
                self.context_menu = None;
                if self.playlist_contains_track(playlist_id, &track) {
                    let playlist_name = self
                        .playlist_by_id(playlist_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Playlist".to_string());
                    self.duplicate_prompt = Some(DuplicatePromptState {
                        target: DuplicateTarget::Playlist {
                            track,
                            playlist_id,
                            playlist_name,
                        },
                    });
                    Task::none()
                } else {
                    self.execute_add_track_to_playlist(track, playlist_id)
                }
            }

            Message::ContextAddAlbumToPlaylist { album, playlist_id } => {
                self.context_menu = None;
                let playlist = self.playlist_by_id(playlist_id);
                if let Some(p) = playlist {
                    let path = PathBuf::from(&p.file_path);
                    let file_path = p.file_path.clone();
                    let name = p.name.clone();
                    let db = self.db.clone();
                    let filter = TrackFilter::album(album.album.as_ref());
                    let is_open = self.playlists.selected_id == Some(playlist_id);
                    let reload_detail = if is_open {
                        self.resolve_playlist_intent(playlist_id, PlaylistOpenIntent::View)
                    } else {
                        Task::none()
                    };

                    let save_task = Task::perform(
                        async move {
                            let tracks = db
                                .get_all_track_rows_filtered(
                                    Some(&filter),
                                    SortField::TrackNum,
                                    SortDirection::Ascending,
                                )
                                .await
                                .unwrap_or_default();

                            let wrote = tokio::task::spawn_blocking(move || {
                                core_engine::append_tracks_to_m3u(&path, &tracks)
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

            Message::ContextAddArtistToPlaylist {
                artist,
                playlist_id,
            } => {
                self.context_menu = None;
                let playlist = self.playlist_by_id(playlist_id);
                if let Some(p) = playlist {
                    let path = PathBuf::from(&p.file_path);
                    let file_path = p.file_path.clone();
                    let name = p.name.clone();
                    let db = self.db.clone();
                    let filter = TrackFilter::artist(artist.artist.as_ref());
                    let is_open = self.playlists.selected_id == Some(playlist_id);
                    let reload_detail = if is_open {
                        self.resolve_playlist_intent(playlist_id, PlaylistOpenIntent::View)
                    } else {
                        Task::none()
                    };

                    let save_task = Task::perform(
                        async move {
                            let tracks = db
                                .get_all_track_rows_filtered(
                                    Some(&filter),
                                    SortField::TrackNum,
                                    SortDirection::Ascending,
                                )
                                .await
                                .unwrap_or_default();

                            let wrote = tokio::task::spawn_blocking(move || {
                                core_engine::append_tracks_to_m3u(&path, &tracks)
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

            Message::ContextNewPlaylistWithTracks(tracks) => {
                self.context_menu = None;
                self.active_tab = ViewTab::Playlists;
                self.playlists.set_section(PlaylistSection::Playlists);
                self.playlists.creating = true;
                self.playlists.new_name_input.clear();
                self.pending_playlist_tracks = tracks;
                Task::none()
            }

            Message::ContextNewPlaylistWithAlbum(album) => {
                self.context_menu = None;
                let db = self.db.clone();
                let filter = TrackFilter::album(album.album.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::ContextNewPlaylistWithTracks,
                )
            }

            Message::ContextNewPlaylistWithArtist(artist) => {
                self.context_menu = None;
                let db = self.db.clone();
                let filter = TrackFilter::artist(artist.artist.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::ContextNewPlaylistWithTracks,
                )
            }

            Message::ConfirmDuplicatePrompt => {
                if let Some(prompt) = self.duplicate_prompt.take() {
                    match prompt.target {
                        DuplicateTarget::Playlist {
                            track, playlist_id, ..
                        } => self.execute_add_track_to_playlist(track, playlist_id),
                        DuplicateTarget::Queue { track } => self.execute_add_track_to_queue(track),
                    }
                } else {
                    Task::none()
                }
            }

            Message::CancelDuplicatePrompt => {
                self.duplicate_prompt = None;
                Task::none()
            }

            Message::EscapePressed => {
                if self.duplicate_prompt.is_some() {
                    self.duplicate_prompt = None;
                } else if self.context_menu.is_some() {
                    self.context_menu = None;
                }
                Task::none()
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
                self.settings.set_library_folders(folders.clone());
                if self.watch_library {
                    if let Some(ref mut watcher) = self.library_watcher {
                        let _ = watcher.update_folders(&folders);
                    } else {
                        let (tx, rx) = crossbeam_channel::unbounded();
                        match core_engine::LibraryWatcher::new(tx, Duration::from_millis(1000)) {
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
                        match core_engine::LibraryWatcher::new(tx, Duration::from_millis(1000)) {
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
                self.now_playing.clear();
                self.current_track = None;
                self.player_bar.update_track(None);
                self.current_track_id = None;
                self.current_cover = None;
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
                info!("Purging thumbnail disk and GPU cache");
                self.albums.clear_covers();
                let db_path = self.db_path.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let _ = core_engine::clear_thumbnail_cache();
                            let stats = core_engine::get_thumbnail_cache_stats();
                            let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
                            (stats, db_size)
                        })
                        .await
                        .unwrap_or_default()
                    },
                    |(stats, db_size)| Message::StorageStatsLoaded(stats, db_size),
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

            Message::StorageStatsLoaded(stats, db_size_bytes) => {
                self.cache_stats = stats;
                self.db_size_bytes = db_size_bytes;
                Task::none()
            }

            Message::SelectSettingsTab(tab) => {
                self.settings.select_tab(tab);
                let mut tasks = vec![self.save_state_task(), self.load_storage_stats_task()];
                if tab == SettingsTab::Appearance {
                    if let Some(font_task) = self.ensure_fonts_task() {
                        tasks.push(font_task);
                    }
                }
                Task::batch(tasks)
            }

            Message::SelectAlbum(album_name) => {
                self.albums.clear_covers();
                self.active_tab = ViewTab::Tracks;
                // Exact album scope so the library shows only this album,
                // instead of every track whose title/artist contains the name.
                self.library.set_filter(TrackFilter::album(album_name));
                // Present the album in disc/track order rather than title order.
                self.library.sort_field = SortField::TrackNum;
                self.library.sort_dir = SortDirection::Ascending;
                Task::batch([self.reload_tracks_task(), self.save_state_task()])
            }

            Message::SelectArtist(artist_name) => {
                self.active_tab = ViewTab::Tracks;
                self.library.set_filter(TrackFilter::artist(artist_name));
                Task::batch([self.reload_tracks_task(), self.save_state_task()])
            }

            // ── Playlists ───────────────────────────────────────────────────
            Message::PlaylistSectionChanged(section) => {
                self.playlists.set_section(section);
                Task::none()
            }

            Message::PlaylistsLoaded(playlists) => {
                let load_covers_task = self.load_playlist_covers_task(&playlists);
                self.playlists.set_playlists(playlists);
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
                    return Task::none();
                }
                let Some(playlist) = self.playlist_by_id(id) else {
                    return Task::none();
                };
                self.playlists.selected_id = Some(id);
                let db = self.db.clone();
                Task::perform(
                    resolve_playlist(db, playlist.file_path),
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

            Message::PlayPlaylist(id) => self.open_playlist(id, 0),
            Message::PlayPlaylistItem(id, index) => self.open_playlist(id, index),
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
                    .is_some_and(|item| item.is_available());
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
                        let path = unique_playlist_path(&name);
                        let count = if !pending_tracks.is_empty() {
                            match core_engine::append_tracks_to_m3u(&path, &pending_tracks) {
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
                            let sanitized = core_engine::sanitize_playlist_filename(&name);
                            let new_path = if sanitized == old_stem {
                                old_path.clone()
                            } else {
                                unique_playlist_path(&name)
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
                self.playlists.set_view_mode(mode);
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
                            if !core_engine::is_supported_playlist_file(&path) {
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
            Message::QueueNew => {
                self.queues.create(DEFAULT_QUEUE_NAME);
                self.playlists.reset_queue_window();
                self.playlists.set_section(PlaylistSection::Queues);
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
                    self.current_blurred_cover = None;
                    self.current_next_cover = None;
                    self.current_tech_info = None;
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
                self.playlists.reset_queue_window();
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
                    self.playlists.on_queue_scroll_offset(scroll_y, queue.len());
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
                            shuffle_tracks(&mut tracks);
                        }
                        let track_id = tracks.first().map(|t| t.id).unwrap_or(0);
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
            Message::QueueRemoveTrack { queue_id, index } => {
                if self.queues.active == Some(queue_id) {
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.remove(index);
                    }
                }
                self.queues.touch_active_summary();
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
                        let path = unique_playlist_path(&name);
                        let entries: Vec<PlaylistEntry> = tracks
                            .iter()
                            .map(|t| PlaylistEntry {
                                raw_path: t.file_path.clone(),
                                resolved_path: PathBuf::from(&t.file_path),
                                title: Some(t.title.clone()),
                                duration_ms: Some(t.duration_ms),
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

            Message::SelectTheme(name) => {
                self.theme_manager.set_theme(&name);
                self.save_config_task()
            }

            Message::ReloadThemes => {
                self.theme_manager.load_all_themes();
                Task::none()
            }

            Message::SelectLayout(name) => {
                if self.layout_manager.select_layout(&name) {
                    self.save_config_task()
                } else {
                    Task::none()
                }
            }

            Message::ReloadLayouts => {
                self.layout_manager.load_all_layouts();
                Task::none()
            }

            Message::SelectFont(font_name) => {
                info!("Selected font: {}", font_name);
                let font_opt = if font_name == "Default" || font_name.trim().is_empty() {
                    None
                } else {
                    Some(font_name.trim().to_string())
                };
                self.theme_manager.set_font(font_opt);
                self.save_config_task()
            }

            Message::FontsLoaded(fonts) => {
                self.settings.set_available_fonts(fonts);
                Task::none()
            }

            Message::ToggleListenBrainz(enabled) => {
                self.scrobble_config.listenbrainz_enabled = enabled;
                self.listenbrainz_plugin.set_enabled(enabled);
                self.save_config_task()
            }

            Message::ListenBrainzTokenChanged(token) => {
                self.scrobble_config.listenbrainz_token = token.clone();
                self.listenbrainz_plugin.set_user_token(token);
                self.save_config_task()
            }

            Message::ListenBrainzUrlChanged(url) => {
                self.scrobble_config.listenbrainz_api_url = url.clone();
                self.listenbrainz_plugin.set_api_url(url);
                self.save_config_task()
            }

            Message::ToggleLastFm(enabled) => {
                self.scrobble_config.lastfm_enabled = enabled;
                self.lastfm_plugin.set_enabled(enabled);
                self.save_config_task()
            }

            Message::LastFmApiKeyChanged(key) => {
                self.scrobble_config.lastfm_api_key = key.clone();
                self.lastfm_plugin.set_api_key(key);
                self.save_config_task()
            }

            Message::LastFmApiSecretChanged(secret) => {
                self.scrobble_config.lastfm_api_secret = secret.clone();
                self.lastfm_plugin.set_api_secret(secret);
                self.save_config_task()
            }

            Message::LastFmSessionKeyChanged(sk) => {
                self.scrobble_config.lastfm_session_key = sk.clone();
                self.lastfm_plugin.set_session_key(sk);
                self.save_config_task()
            }

            Message::LastFmUrlChanged(url) => {
                self.scrobble_config.lastfm_api_url = url.clone();
                self.lastfm_plugin.set_api_url(url);
                self.save_config_task()
            }

            Message::ToggleSubmitNowPlaying(submit) => {
                self.scrobble_config.submit_now_playing = submit;
                self.listenbrainz_plugin.set_submit_now_playing(submit);
                self.lastfm_plugin.set_submit_now_playing(submit);
                self.save_config_task()
            }

            Message::SetScrobblePercent(pct) => {
                self.scrobble_config.scrobble_percent = pct;
                self.listenbrainz_plugin.set_scrobble_percent(pct);
                self.lastfm_plugin.set_scrobble_percent(pct);
                self.save_config_task()
            }

            Message::TestScrobble(service) => {
                let plugin_lb = self.listenbrainz_plugin.clone();
                let plugin_lfm = self.lastfm_plugin.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || match service {
                            ScrobbleService::ListenBrainz => (service, plugin_lb.test_connection()),
                            ScrobbleService::LastFm => (service, plugin_lfm.test_connection()),
                        })
                        .await
                        .unwrap_or_else(|e| (service, Err(format!("Task execution error: {}", e))))
                    },
                    |(service, res)| Message::ScrobbleTestFinished(service, res),
                )
            }

            Message::ScrobbleTestFinished(service, res) => {
                self.settings.set_scrobble_test_status(Some((service, res)));
                Task::none()
            }

            Message::TracksLoaded(rows, total) => {
                self.library.set_window(rows, total);
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
                Task::none()
            }

            Message::AlbumSortChanged(field) => {
                self.albums.toggle_sort(field);
                let save_task = self.save_state_task();
                let covers_task = self.load_visible_album_covers_task();
                Task::batch([save_task, covers_task])
            }

            Message::ArtistSortChanged(field) => {
                self.artists.toggle_sort(field);
                self.save_state_task()
            }

            Message::AlbumsLoaded(albums) => {
                self.albums.set_albums(albums);
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
                Task::none()
            }

            Message::ArtistsScrolled(viewport) => {
                self.artists.on_scroll(&viewport);
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

            Message::SaveConfig => {
                self.config_save_seq = self.config_save_seq.wrapping_add(1);
                self.save_config_task()
            }

            Message::DebouncedConfigSave(seq) => {
                if seq == self.config_save_seq {
                    self.save_config_task()
                } else {
                    Task::none()
                }
            }

            Message::ConfigSaved => Task::none(),

            Message::SaveState => {
                self.state_save_seq = self.state_save_seq.wrapping_add(1);
                self.save_state_task()
            }

            Message::DebouncedStateSave(seq) => {
                if seq == self.state_save_seq {
                    self.save_state_task()
                } else {
                    Task::none()
                }
            }

            Message::StateSaved => Task::none(),
        }
    }

    pub fn get_next_track(&self) -> Option<&TrackRow> {
        if let Some(queue) = self.queues.active_queue() {
            if self.is_shuffle && !queue.is_empty() {
                let len = queue.len();
                if len <= 1 {
                    return queue.tracks.get(queue.current_index().unwrap_or(0));
                }
                let current = queue.current_index().unwrap_or(0);
                let mut pick = (self.position_ms as usize ^ 0x9e3779b9) % len;
                if pick == current {
                    pick = (pick + 1) % len;
                }
                return queue.tracks.get(pick);
            }
            if let Some(idx) = queue.next_index() {
                return queue.tracks.get(idx);
            }
            return None;
        }
        self.library
            .get_next_row(self.current_track_id, self.is_shuffle, self.position_ms)
    }

    fn load_track_assets_task(
        &self,
        track_id: i64,
        path: PathBuf,
        artist: String,
        title: String,
        duration_ms: u64,
    ) -> Task<Message> {
        if self.is_background_mode {
            return Task::none();
        }

        let next_track_path = self.get_next_track().map(|nt| PathBuf::from(&nt.file_path));
        let plugin_manager = self.plugin_manager.clone();

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let cover =
                        core_engine::get_or_create_thumbnail(&path, core_engine::THUMB_DIM_HIGH);

                    let blurred = cover
                        .as_deref()
                        .and_then(core_engine::generate_blurred_cover);

                    let tech_info = core_engine::extract_technical_info(&path, duration_ms);

                    let lyrics = plugin_manager.fetch_lyrics(&artist, &title);

                    let next_cover = next_track_path.and_then(|nt_path| {
                        core_engine::get_or_create_thumbnail(
                            &nt_path,
                            core_engine::THUMB_DIM_DEFAULT,
                        )
                    });

                    (cover, blurred, next_cover, tech_info, lyrics)
                })
                .await
                .unwrap_or((None, None, None, None, None))
            },
            move |(cover, blurred, next_cover, tech_info, lyrics)| Message::PlaybackAssetsLoaded {
                track_id,
                cover,
                blurred,
                next_cover,
                tech_info,
                lyrics,
            },
        )
    }

    pub fn queue_next_gapless_track(&mut self) {
        if !self.gapless {
            self.pending_gapless_next = None;
            self.audio_controller.set_next_track(None);
            return;
        }

        if self.is_repeat {
            if let Some(track) = &self.current_track {
                let path = PathBuf::from(&track.file_path);
                self.audio_controller.set_next_track(Some(path));
                let track_row = TrackRow::from(track);
                self.pending_gapless_next = Some(track_row);
            } else {
                self.audio_controller.set_next_track(None);
                self.pending_gapless_next = None;
            }
            return;
        }

        if let Some(next_track) = self.get_next_track() {
            let path = PathBuf::from(&next_track.file_path);
            self.audio_controller.set_next_track(Some(path));
            self.pending_gapless_next = Some(next_track.clone());
        } else {
            self.audio_controller.set_next_track(None);
            self.pending_gapless_next = None;
        }
    }

    /// Advances the active queue cursor and returns the id of the track to play.
    ///
    /// `forward` selects next/previous; shuffle ignores direction and jumps to a
    /// pseudo-random slot. Returns `None` at a queue boundary.
    fn step_active_queue(&mut self, forward: bool) -> Option<i64> {
        let position_seed = self.position_ms;
        let is_shuffle = self.is_shuffle;
        let queue = self.queues.active_queue_mut()?;
        if queue.is_empty() {
            return None;
        }

        let target = if is_shuffle {
            let len = queue.len();
            if len <= 1 {
                queue.current_index().unwrap_or(0)
            } else {
                let current = queue.current_index().unwrap_or(0);
                let mut pick = (position_seed as usize ^ 0x9e3779b9) % len;
                if pick == current {
                    pick = (pick + 1) % len;
                }
                pick
            }
        } else if forward {
            queue.next_index()?
        } else {
            queue.previous_index()?
        };

        queue.set_current(target);
        Some(queue.tracks[target].id)
    }

    /// Persists the active queue (metadata + membership + active selection).
    fn persist_active_queue_task(&self) -> Task<Message> {
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
    fn persist_active_current_task(&self) -> Task<Message> {
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
    fn finish_queue_drag_task(&mut self) -> Task<Message> {
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

    /// Writes the open playlist's reordered entries back to its M3U file.
    /// Fires exactly once on drag-release (never per hover), off the UI
    /// thread since the write is synchronous filesystem I/O.
    fn finish_detail_drag_task(&mut self) -> Task<Message> {
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

    /// Snaps the queue detail list so the playing cursor is visible. Skips
    /// when the cursor already sits inside the rendered window.
    fn snap_queue_to_current(&self) -> Task<Message> {
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
    fn start_library_playback(&self, track_id: i64) -> Task<Message> {
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
    fn create_queue_from_tracks(
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
        id
    }

    fn playlist_by_id(&self, id: i64) -> Option<PlaylistSummary> {
        self.playlists
            .playlists
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// Resolves a playlist and plays it, optionally starting at `start_index`.
    fn open_playlist(&self, id: i64, start_index: usize) -> Task<Message> {
        self.resolve_playlist_intent(id, PlaylistOpenIntent::Play { start_index })
    }

    fn resolve_playlist_intent(&self, id: i64, intent: PlaylistOpenIntent) -> Task<Message> {
        let Some(playlist) = self.playlist_by_id(id) else {
            return Task::none();
        };
        let db = self.db.clone();
        Task::perform(
            resolve_playlist(db, playlist.file_path),
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
    fn load_playlist_covers_task(&self, playlists: &[PlaylistSummary]) -> Task<Message> {
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
                            core_engine::parse_m3u_preview_paths(&m3u_path, 4).unwrap_or_default();
                        let mut covers = Vec::with_capacity(track_paths.len());
                        for track_path in track_paths {
                            if let Ok(bytes) = core_engine::get_or_create_thumbnail(
                                &track_path,
                                core_engine::THUMB_DIM_DEFAULT,
                            ) {
                                if !bytes.is_empty() {
                                    covers.push(bytes);
                                }
                            }
                        }
                        results.push((id, covers));
                    }
                    results
                })
                .await
                .unwrap_or_default()
            },
            Message::PlaylistCoversLoaded,
        )
    }

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            tab_order: self.tab_order.clone(),
            theme_name: self.theme_manager.active_theme_name.clone(),
            layout_name: self.layout_manager.active_layout_name.clone(),
            primary_font: self.theme_manager.current_config.font_family.clone(),
            page_size: self.library.page_size,
            tile_width: self.albums.tile_width,
            album_tile_width: self.albums.tile_width,
            artist_tile_width: self.artists.tile_width,
            now_playing_cover_size: self.now_playing.cover_size,
            gapless: self.gapless,
            background_mode: self.background_mode,
            background_mode_timeout_secs: self.background_mode_timeout_secs,
            watch_library: self.watch_library,
            scrobble: self.scrobble_config.clone(),
        }
    }

    pub fn to_state(&self) -> AppState {
        let (queue_id, playing_seq) = if let Some(queue) = self.queues.active_queue() {
            (Some(queue.id), queue.current_index())
        } else if let Some(active_id) = self.queues.active {
            if let Some(summary) = self.queues.summary(active_id) {
                (
                    Some(summary.id),
                    if summary.track_count > 0 {
                        Some(summary.current)
                    } else {
                        None
                    },
                )
            } else {
                (Some(active_id), None)
            }
        } else {
            (None, None)
        };

        AppState {
            active_tab: self.active_tab,
            settings_tab: self.settings.active_tab,
            volume: self.volume,
            is_shuffle: self.is_shuffle,
            is_repeat: self.is_repeat,
            albums_view_mode: self.albums.view_mode,
            artists_view_mode: self.artists.view_mode,
            playlists_view_mode: self.playlists.view_mode,
            album_sort_field: self.albums.sort_field,
            artist_sort_field: self.artists.sort_field,
            playlist_sort_field: self.playlists.sort_field,
            window_size: Some((self.window_size.width, self.window_size.height)),
            window_width: None,
            window_height: None,
            queue_id,
            playing_seq,
        }
    }

    pub fn save_config_task(&self) -> Task<Message> {
        let config = self.to_config();
        let path = self.config_path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    config.save_to_path(&path);
                })
                .await
                .ok();
            },
            |_| Message::ConfigSaved,
        )
    }

    pub fn save_config_debounced(&mut self) -> Task<Message> {
        self.config_save_seq = self.config_save_seq.wrapping_add(1);
        let seq = self.config_save_seq;
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                seq
            },
            Message::DebouncedConfigSave,
        )
    }

    pub fn save_state_task(&self) -> Task<Message> {
        let state = self.to_state();
        let path = self.state_path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    state.save_to_path(&path);
                })
                .await
                .ok();
            },
            |_| Message::StateSaved,
        )
    }

    pub fn save_state_debounced(&mut self) -> Task<Message> {
        self.state_save_seq = self.state_save_seq.wrapping_add(1);
        let seq = self.state_save_seq;
        Task::perform(
            async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                seq
            },
            Message::DebouncedStateSave,
        )
    }

    /// Refreshes thumbnail-cache and database-size telemetry off the UI thread.
    /// Called only when the relevant screens/actions occur instead of every frame.
    fn load_storage_stats_task(&self) -> Task<Message> {
        let db_path = self.db_path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let stats = core_engine::get_thumbnail_cache_stats();
                    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
                    (stats, db_size)
                })
                .await
                .unwrap_or_default()
            },
            |(stats, db_size)| Message::StorageStatsLoaded(stats, db_size),
        )
    }

    fn ensure_fonts_task(&mut self) -> Option<Task<Message>> {
        if !self.settings.fonts_loaded {
            self.settings.fonts_loaded = true;
            Some(Task::perform(
                async move {
                    tokio::task::spawn_blocking(crate::font::list_font_families)
                        .await
                        .unwrap_or_default()
                },
                Message::FontsLoaded,
            ))
        } else {
            None
        }
    }

    fn find_track_row(&self, track_id: i64) -> Option<TrackRow> {
        if let Some(row) = self.library.rows.iter().find(|t| t.id == track_id) {
            return Some(row.clone());
        }
        if let Some(row) = self
            .queues
            .active_queue()
            .and_then(|q| q.tracks.iter().find(|t| t.id == track_id))
        {
            return Some(row.clone());
        }
        if let Some(row) = self
            .playlists
            .detail_tracks
            .iter()
            .find(|t| t.id == track_id)
        {
            return Some(row.clone());
        }
        if let Some(track) = &self.current_track {
            if track.id == track_id {
                return Some(TrackRow::from(track));
            }
        }
        if let Some(item) = self
            .playlists
            .detail_items
            .iter()
            .find(|i| i.track_id == Some(track_id))
        {
            return Some(TrackRow {
                id: track_id,
                title: item.title.clone(),
                artist: item.artist.as_str().into(),
                album: "".into(),
                duration_ms: 0,
                file_path: String::new(),
                duration_label: item.duration_label.clone(),
                year: None,
                track_num: None,
                disc_num: None,
                album_artist: None,
                genre: None,
            });
        }
        None
    }

    fn load_cover_chunk_task(
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
        Task::perform(
            async move {
                // Resolve representative track paths for the whole chunk in a
                // single `IN (...)` query, then downscale/cache covers off the
                // async runtime.
                let ids: Vec<i64> = chunk.iter().map(|(_, id)| *id).collect();
                let paths = db.get_track_paths_by_ids(&ids).await.unwrap_or_default();

                let resolved: Vec<(String, Option<PathBuf>)> = chunk
                    .into_iter()
                    .map(|(key, id)| {
                        let path = paths.get(&id).map(PathBuf::from);
                        (key, path)
                    })
                    .collect();

                tokio::task::spawn_blocking(move || {
                    let mut results = Vec::with_capacity(resolved.len());
                    for (key, path) in resolved {
                        // `None` bytes means "no cover available"; `Some` carries the
                        // downscaled JPEG. A missing path is treated as no cover.
                        let bytes = match path {
                            Some(path) => core_engine::get_or_create_thumbnail(
                                &path,
                                core_engine::THUMB_DIM_DEFAULT,
                            )
                            .unwrap_or_default(),
                            None => Vec::new(),
                        };
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

    fn generate_thumbnails_task(&self) -> Task<Message> {
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
                                    let _ = core_engine::get_or_create_thumbnail(
                                        path,
                                        core_engine::THUMB_DIM_DEFAULT,
                                    );
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

    fn load_visible_album_covers_task(&mut self) -> Task<Message> {
        let to_load = self.albums.get_missing_covers_to_load();
        if to_load.is_empty() {
            return Task::none();
        }
        let request_id = self
            .albums
            .mark_loading(to_load.iter().map(|(k, _)| k.clone()));
        self.load_cover_chunk_task(request_id, to_load)
    }

    fn reload_tracks_task(&self) -> Task<Message> {
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

    fn reload_after_scan_task(&mut self) -> Task<Message> {
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

    fn load_track_window_task(&self, offset: usize) -> Task<Message> {
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

    fn playlist_contains_track(&self, playlist_id: i64, track: &TrackRow) -> bool {
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
            if let Ok(parsed) = core_engine::parse_m3u(&p.file_path) {
                return parsed.entries.iter().any(|e| {
                    e.resolved_path.to_string_lossy() == track.file_path
                        || e.raw_path == track.file_path
                });
            }
        }

        false
    }

    fn queue_contains_track(&self, track: &TrackRow) -> bool {
        self.queues
            .active_queue()
            .map(|q| {
                q.tracks
                    .iter()
                    .any(|t| t.id == track.id || t.file_path == track.file_path)
            })
            .unwrap_or(false)
    }

    fn execute_add_track_to_playlist(
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
                        core_engine::append_tracks_to_m3u(&path, &[track])
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

    fn execute_add_track_to_queue(&mut self, track: TrackRow) -> Task<Message> {
        if self.queues.active.is_none() {
            self.queues.create(DEFAULT_QUEUE_NAME);
        }
        if let Some(queue) = self.queues.active_queue_mut() {
            queue.append_many([track]);
        }
        self.queues.touch_active_summary();
        self.persist_active_queue_task()
    }

    fn render_duplicate_prompt<'a>(
        &'a self,
        prompt: &'a DuplicatePromptState,
        theme_cfg: &'a ThemeConfig,
    ) -> Element<'a, Message> {
        let (header, message_text) = match &prompt.target {
            DuplicateTarget::Playlist {
                track,
                playlist_name,
                ..
            } => (
                "Track Already in Playlist",
                format!(
                    "\"{}\" by {} is already in playlist \"{}\". Do you want to add it again?",
                    track.title,
                    track.display_artist(),
                    playlist_name
                ),
            ),
            DuplicateTarget::Queue { track } => (
                "Track Already in Queue",
                format!(
                    "\"{}\" by {} is already in the playback queue. Do you want to add it again?",
                    track.title,
                    track.display_artist()
                ),
            ),
        };

        let backdrop = mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                    ..Default::default()
                }),
        )
        .on_press(Message::CancelDuplicatePrompt);

        let dialog_content = column![
            row![
                icons::icon(icons::DISC_SVG, 16.0, theme_cfg.primary_color()),
                Space::new().width(8),
                components::text(theme_cfg, header)
                    .size(14)
                    .color(theme_cfg.text_color()),
            ]
            .align_y(Alignment::Center),
            Space::new().height(12),
            components::text(theme_cfg, message_text)
                .size(12)
                .color(theme_cfg.text_muted_color()),
            Space::new().height(20),
            row![
                Space::new().width(Length::Fill),
                button(
                    components::text(theme_cfg, "Cancel")
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                )
                .padding([6, 14])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::CancelDuplicatePrompt),
                Space::new().width(8),
                button(
                    components::text(theme_cfg, "Add anyway")
                        .size(12)
                        .color(Color::WHITE)
                )
                .padding([6, 14])
                .style(styles::primary_button(theme_cfg))
                .on_press(Message::ConfirmDuplicatePrompt),
            ]
            .align_y(Alignment::Center),
        ]
        .width(Length::Fixed(380.0));

        let dialog_box = container(dialog_content)
            .padding(20)
            .style(styles::dialog_container(theme_cfg));

        let centered_dialog = iced::widget::center(dialog_box)
            .width(Length::Fill)
            .height(Length::Fill);

        iced::widget::stack([backdrop.boxed(), centered_dialog.boxed()]).boxed()
    }

    pub fn enter_background_mode(&mut self) -> Task<Message> {
        if self.is_background_mode {
            return Task::none();
        }
        info!("Entering background playback mode: evicting UI catalogs, covers, and caches from memory");
        self.is_background_mode = true;

        // 1. Evict all decoded textures and handles
        self.current_cover = None;
        self.current_blurred_cover = None;
        self.current_next_cover = None;
        self.albums.clear_covers();
        self.playlists.playlist_covers = std::collections::HashMap::new();

        // 2. Drop catalog summaries from RAM
        self.albums.clear();
        self.artists.clear();
        self.library.clear();
        self.playlists.clear_detail();
        self.playlists.playlists = Vec::new();
        self.now_playing.clear();

        // 3. Clear transient popups
        self.context_menu = None;
        self.duplicate_prompt = None;

        // 4. Release intern pool capacity
        core_engine::intern::clear();

        Task::none()
    }

    pub fn exit_background_mode(&mut self) -> Task<Message> {
        if !self.is_background_mode {
            return Task::none();
        }
        info!("Exiting background playback mode: restoring active view and UI assets");
        self.is_background_mode = false;
        self.minimized_at = None;

        let mut tasks = Vec::new();

        // Restore current track artwork and technical info for player bar & now playing
        if let Some(track) = &self.current_track {
            let path = PathBuf::from(&track.file_path);
            tasks.push(self.load_track_assets_task(
                track.id,
                path,
                track.artist.to_string(),
                track.title.clone(),
                track.duration_ms,
            ));
        }

        // Restore active tab content
        match self.active_tab {
            ViewTab::Tracks => tasks.push(self.reload_tracks_task()),
            ViewTab::Albums => {
                let db = self.db.clone();
                tasks.push(Task::perform(
                    async move { db.get_albums().await.unwrap_or_default() },
                    Message::AlbumsLoaded,
                ));
            }
            ViewTab::Artists => {
                let db = self.db.clone();
                tasks.push(Task::perform(
                    async move { db.get_artists().await.unwrap_or_default() },
                    Message::ArtistsLoaded,
                ));
            }
            ViewTab::Playlists => {
                let db = self.db.clone();
                tasks.push(Task::perform(
                    async move { db.get_playlists().await.unwrap_or_default() },
                    Message::PlaylistsLoaded,
                ));
            }
            ViewTab::Settings | ViewTab::Lyrics => {}
        }

        Task::batch(tasks)
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.is_background_mode {
            return iced::widget::Space::new(0, 0).into();
        }
        let theme_cfg = &self.theme_manager.current_config;
        let base_view = self.render_layout(theme_cfg);
        let mut layers = vec![base_view];
        if let Some(ref menu) = self.context_menu {
            layers.push(components::render_context_menu(
                menu,
                &self.playlists.playlists,
                theme_cfg,
                self.window_size,
            ));
        }
        if let Some(ref prompt) = self.duplicate_prompt {
            layers.push(self.render_duplicate_prompt(prompt, theme_cfg));
        }
        iced::widget::stack(layers).boxed()
    }
}

fn get_database_path() -> PathBuf {
    core_engine::database_file()
}

/// Parses an M3U file and matches its entries against indexed library tracks.
async fn resolve_playlist(db: LibraryDatabase, file_path: String) -> PlaylistResolution {
    let parsed = match parse_m3u(&file_path) {
        Ok(parsed) => parsed,
        Err(e) => {
            warn!("Failed to parse playlist {}: {}", file_path, e);
            return PlaylistResolution {
                items: Vec::new(),
                entries: Vec::new(),
                tracks: Vec::new(),
            };
        }
    };

    let path_strings: Vec<String> = parsed
        .entries
        .iter()
        .map(|e| e.resolved_path.to_string_lossy().to_string())
        .collect();
    let id_map = db
        .get_track_ids_by_paths(&path_strings)
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = path_strings
        .iter()
        .filter_map(|p| id_map.get(p).copied())
        .collect();
    let rows = db.get_track_rows_by_ids(&ids).await.unwrap_or_default();

    let mut items = Vec::with_capacity(parsed.entries.len());
    let mut tracks = Vec::new();
    for entry in &parsed.entries {
        let resolved = entry.resolved_path.to_string_lossy().to_string();
        if let Some(id) = id_map.get(&resolved).copied() {
            if let Some(row) = rows.get(&id) {
                items.push(PlaylistItem {
                    title: row.title.clone(),
                    artist: row.display_artist().to_string(),
                    duration_label: row.formatted_duration().to_string(),
                    track_id: Some(id),
                });
                tracks.push(row.clone());
                continue;
            }
        }

        let title = entry
            .title
            .clone()
            .or_else(|| {
                entry
                    .resolved_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| entry.raw_path.clone());
        items.push(PlaylistItem {
            title,
            artist: "Not in library".to_string(),
            duration_label: entry
                .duration_ms
                .map(core_engine::format_duration)
                .unwrap_or_default(),
            track_id: None,
        });
    }

    PlaylistResolution {
        items,
        entries: parsed.entries,
        tracks,
    }
}

/// Maps the dominant scope of a filter to a queue source kind.
fn queue_source_for_filter(filter: &TrackFilter) -> QueueSource {
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
fn snap_queue_scroll_top() -> Task<Message> {
    snap_to(
        iced::widget::Id::new("queue_scroll"),
        RelativeOffset::START,
        Animation::Instant,
    )
}

/// Fisher–Yates shuffle seeded from the system clock (no `rand` dependency).
fn shuffle_tracks(tracks: &mut [TrackRow]) {
    if tracks.len() < 2 {
        return;
    }
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        | 1;
    for i in (1..tracks.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let j = (seed % (i as u64 + 1)) as usize;
        tracks.swap(i, j);
    }
}

/// Returns a non-colliding `.m3u8` path inside the managed playlists directory.
fn unique_playlist_path(name: &str) -> PathBuf {
    let dir = core_engine::playlists_dir();
    let base = core_engine::sanitize_playlist_filename(name);
    let mut candidate = dir.join(format!("{}.m3u8", base));
    let mut counter = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{} ({}).m3u8", base, counter));
        counter += 1;
    }
    candidate
}

fn handle_drag_event(
    event: iced::Event,
    _status: iced::event::Status,
    _id: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::TabDragReleased)
        }
        iced::Event::Touch(
            iced::touch::Event::FingerLifted { .. } | iced::touch::Event::FingerLost { .. },
        ) => Some(Message::TabDragReleased),
        _ => None,
    }
}

static LAST_CURSOR_POS: std::sync::Mutex<iced::Point> =
    std::sync::Mutex::new(iced::Point { x: 100.0, y: 100.0 });

fn handle_cursor_and_window_event(
    event: iced::Event,
    _status: iced::event::Status,
    id: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            if let Ok(mut guard) = LAST_CURSOR_POS.lock() {
                *guard = position;
            }
            None
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        }
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Message::WindowUnfocused(id)),
        iced::Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused(id)),
        _ => None,
    }
}

fn handle_escape_key(
    event: iced::Event,
    _status: iced::event::Status,
    _id: iced::window::Id,
) -> Option<Message> {
    if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        ..
    }) = event
    {
        Some(Message::EscapePressed)
    } else {
        None
    }
}
