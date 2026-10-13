use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use audio::{AudioController, AudioEngine};
use core_engine::{
    parse_m3u, write_m3u, LibraryDatabase, PlaylistEntry, PlaylistSummary, SortDirection,
    SortField, Track, TrackFilter, TrackRow,
};
use crossbeam_channel::Receiver;
use iced::widget::scrollable::RelativeOffset;
use iced::widget::{button, column, container, image, mouse_area, row, scrollable, Space};
use iced::{Element, Length, Subscription, Task, Theme};
use plugins::scrobbler::{LastFmPlugin, ListenBrainzPlugin, ScrobbleService};
#[cfg(target_os = "linux")]
use plugins::MprisPlugin;
use plugins::{PlayerAction, PluginManager, TrackMetadata};
use tracing::{error, info, warn};

use crate::components::{border_vert, text};
use crate::config::{AppConfig, ScrobbleConfig};
use crate::icons;
use crate::message::{
    Message, PlaylistItem, PlaylistOpenIntent, PlaylistResolution, PlaylistSection, ViewMode,
    ViewTab,
};
use crate::queue::{queue_window_for_start, QueueSource, QueueStore, DEFAULT_QUEUE_NAME};
use crate::theme::{styles, ThemeManager};
use crate::views::{
    self, AlbumsView, ArtistsView, LibraryView, NowPlayingView, PlayerBar, PlaylistsView,
    SettingsView, StatusBar, StatusBarContext,
};

/// Album covers resolved per background chunk. Bounds the `IN (...)` lookup
/// and the `spawn_blocking` downscale batch behind album/artist scrolling.
pub const ALBUM_COVER_LOAD_CHUNK_SIZE: usize = 16;

pub struct App {
    db: LibraryDatabase,
    db_path: PathBuf,
    config_path: PathBuf,
    _audio_engine: AudioEngine,
    audio_controller: AudioController,
    plugin_manager: PluginManager,
    theme_manager: ThemeManager,

    // Navigation & Tabs
    active_tab: ViewTab,
    tab_order: Vec<ViewTab>,
    dragging_tab: Option<ViewTab>,
    hovered_tab: Option<ViewTab>,
    drag_order_changed: bool,

    // Screen-specific state components
    library: LibraryView,
    albums: AlbumsView,
    artists: ArtistsView,
    playlists: PlaylistsView,
    now_playing: NowPlayingView,
    settings: SettingsView,
    player_bar: PlayerBar,

    // Playback queues (in-memory view, persisted to the database)
    queues: QueueStore,

    // Playback State
    current_track: Option<Track>,
    current_track_id: Option<i64>,
    current_cover: Option<image::Handle>,
    current_blurred_cover: Option<image::Handle>,
    current_next_cover: Option<image::Handle>,
    current_tech_info: Option<core_engine::AudioTechnicalInfo>,
    is_playing: bool,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
    is_shuffle: bool,
    is_repeat: bool,

    // Global Scanning Telemetry
    is_scanning: bool,

    // Scrobbling Plugins
    listenbrainz_plugin: Arc<ListenBrainzPlugin>,
    lastfm_plugin: Arc<LastFmPlugin>,
    scrobble_config: ScrobbleConfig,

    config_save_seq: u64,
    search_seq: u64,

    // Storage telemetry (refreshed on demand, never per frame)
    cache_stats: core_engine::CacheStats,
    db_size_bytes: u64,

    mpris_rx: Option<Receiver<PlayerAction>>,
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

        let audio_engine = AudioEngine::new();
        let audio_controller = audio_engine.controller();
        audio_controller.set_volume(config.volume);

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

        let available_fonts = crate::font::list_font_families();
        let config_path = AppConfig::config_path();

        let library = LibraryView::new(config.page_size);
        let albums = AlbumsView::with_tile_width(
            config.albums_view_mode,
            config.album_sort_field,
            config.album_tile_width,
        );
        let artists = ArtistsView::with_tile_width(
            config.artists_view_mode,
            config.artist_sort_field,
            config.artist_tile_width,
        );
        let now_playing = NowPlayingView::with_cover_size(config.now_playing_cover_size);
        let playlists = PlaylistsView::new(config.playlists_view_mode, config.playlist_sort_field);
        let settings = SettingsView::new(config.settings_tab, available_fonts);
        let player_bar = PlayerBar::new();

        let mut queues = QueueStore::default();
        let (queue_records, active_queue_id) = tokio::runtime::Handle::current().block_on(async {
            let records = db.list_queues().await.unwrap_or_default();
            let active = db.get_active_queue_id().await.unwrap_or(None);
            (records, active)
        });
        queues.load_summaries(queue_records);
        queues.set_active_id(active_queue_id.map(|id| id as u64));

        let app = Self {
            db,
            db_path,
            config_path,
            _audio_engine: audio_engine,
            audio_controller,
            plugin_manager,
            theme_manager,

            active_tab: config.active_tab,
            tab_order: config.tab_order.clone(),
            dragging_tab: None,
            hovered_tab: None,
            drag_order_changed: false,

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
            volume: config.volume,
            is_shuffle: config.is_shuffle,
            is_repeat: config.is_repeat,

            is_scanning: false,

            listenbrainz_plugin,
            lastfm_plugin,
            scrobble_config: config.scrobble,
            config_save_seq: 0,
            search_seq: 0,
            cache_stats: core_engine::CacheStats::default(),
            db_size_bytes: 0,

            mpris_rx,
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
        }

        if let Some(active_id) = active_queue_id {
            let db = app.db.clone();
            initial_tasks.push(Task::perform(
                async move {
                    let tracks = db.load_queue_tracks(active_id).await.unwrap_or_default();
                    (active_id, tracks)
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

    pub fn subscription(&self) -> Subscription<Message> {
        // Poll audio playback status at 50ms intervals (~20 FPS smooth seekbar)
        let ticker = iced::time::every(Duration::from_millis(50)).map(|_| Message::AudioTick);
        // Periodic self-healing pass for the album cover window.
        let covers =
            iced::time::every(Duration::from_millis(400)).map(|_| Message::EnsureAlbumCovers);
        let mut subscriptions = vec![ticker, covers];
        if self.dragging_tab.is_some() {
            subscriptions.push(iced::event::listen_with(handle_drag_event));
        }
        Subscription::batch(subscriptions)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AudioTick => {
                let status = self.audio_controller.status();
                let was_playing = self.is_playing;
                self.is_playing = status.is_playing();

                if was_playing != self.is_playing {
                    self.plugin_manager
                        .on_playback_status_change(self.is_playing);
                }

                if !self.player_bar.is_seeking {
                    self.position_ms = status.position_ms();
                    self.plugin_manager.on_position_change(self.position_ms);
                }

                if status.duration_ms() > 0 {
                    self.duration_ms = status.duration_ms();
                }

                // Check if track ended
                if status.take_ended() {
                    info!("Playback ended, triggering next track");
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
                    self.current_track = Some(track.clone());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = track.duration_ms;

                    // Clear stale assets from the previous track right away so we
                    // never render a mismatched cover while the new one loads.
                    self.current_cover = None;
                    self.current_blurred_cover = None;
                    self.current_next_cover = None;
                    self.current_tech_info = None;
                    self.now_playing.set_lyrics(None);

                    let artist = track.artist.clone();
                    let title = track.title.clone();
                    let duration_ms = track.duration_ms;

                    // Notify plugins synchronously (cheap state updates).
                    self.plugin_manager.on_track_change(&TrackMetadata {
                        title: track.title,
                        artist: track.artist,
                        album: track.album,
                        duration_ms: track.duration_ms,
                        file_path: track.file_path,
                    });

                    // Resolve the next track's path from the in-memory window; the
                    // background task then produces its thumbnail off-thread.
                    let next_track_path =
                        self.get_next_track().map(|nt| PathBuf::from(&nt.file_path));

                    let plugin_manager = self.plugin_manager.clone();
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let cover = core_engine::get_or_create_thumbnail(
                                    &path,
                                    core_engine::THUMB_DIM_HIGH,
                                )
                                .or_else(|| core_engine::extract_cover_art(&path));

                                let blurred = cover
                                    .as_deref()
                                    .and_then(core_engine::generate_blurred_cover);

                                let tech_info =
                                    core_engine::extract_technical_info(&path, duration_ms);

                                let lyrics = plugin_manager.fetch_lyrics(&artist, &title);

                                let next_cover = next_track_path.and_then(|nt_path| {
                                    core_engine::get_or_create_thumbnail(
                                        &nt_path,
                                        core_engine::THUMB_DIM_DEFAULT,
                                    )
                                    .or_else(|| core_engine::extract_cover_art(&nt_path))
                                });

                                (cover, blurred, next_cover, tech_info, lyrics)
                            })
                            .await
                            .unwrap_or((None, None, None, None, None))
                        },
                        move |(cover, blurred, next_cover, tech_info, lyrics)| {
                            Message::PlaybackAssetsLoaded {
                                track_id,
                                cover,
                                blurred,
                                next_cover,
                                tech_info,
                                lyrics,
                            }
                        },
                    )
                } else {
                    Task::none()
                }
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
                    return Task::none();
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
                self.save_config_task()
            }

            Message::ToggleRepeat => {
                self.is_repeat = !self.is_repeat;
                info!("Repeat toggled: {}", self.is_repeat);
                self.plugin_manager.on_repeat_change(self.is_repeat);
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
                    return Task::none();
                }

                if let Some(cur_id) = self.current_track_id {
                    if let Some(prev) = self.library.get_previous_row(cur_id) {
                        return self.update(Message::PlayTrack(prev.id));
                    }
                }
                Task::none()
            }

            Message::Seek(ms) => {
                self.player_bar.on_seek(ms);
                Task::none()
            }

            Message::SeekFinished => {
                let seek_pos = self.player_bar.on_seek_finished();
                self.audio_controller.seek(Duration::from_millis(seek_pos));
                self.position_ms = seek_pos;
                self.plugin_manager.on_seeked(self.position_ms);
                Task::none()
            }

            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.audio_controller.set_volume(vol);
                self.plugin_manager.on_volume_change(vol);
                self.save_config_debounced()
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
                    iced::window::get_latest().and_then(iced::window::close)
                }
            },

            Message::SelectTab(tab) => {
                self.active_tab = tab;
                let save_task = self.save_config_task();
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
                    ViewTab::Settings => Task::batch([save_task, self.load_storage_stats_task()]),
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
                let had_drag = self.dragging_tab.is_some();
                let order_changed = self.drag_order_changed;
                self.dragging_tab = None;
                self.drag_order_changed = false;
                if had_drag && order_changed {
                    self.save_config_task()
                } else {
                    Task::none()
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
                Task::batch([self.save_config_task(), covers])
            }

            Message::SetArtistsViewMode(mode) => {
                self.artists.set_view_mode(mode);
                self.save_config_task()
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

            Message::AddLibraryFolderClicked => {
                self.is_scanning = true;
                Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Add Music Folder to Library")
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
                    let db = self.db.clone();
                    Task::perform(
                        async move {
                            let _ = db.add_library_folder(&path_str).await;
                            let summary = db
                                .scan_directory(&path, None::<fn(usize, usize)>)
                                .await
                                .unwrap_or_default();
                            summary
                        },
                        Message::ScanFinished,
                    )
                }
                None => {
                    self.is_scanning = false;
                    Task::none()
                }
            },

            Message::RemoveLibraryFolder(path_str) => {
                info!("Removing library folder: {}", path_str);
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
                self.settings.set_library_folders(folders);
                Task::none()
            }

            Message::RescanLibraryFolder(path) => {
                self.is_scanning = true;
                info!("Rescanning library folder: {:?}", path);
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let summary = db
                            .scan_directory(&path, None::<fn(usize, usize)>)
                            .await
                            .unwrap_or_default();
                        summary
                    },
                    Message::ScanFinished,
                )
            }

            Message::ReindexLibrary => {
                self.is_scanning = true;
                info!(
                    "Reindexing all library folders: {:?}",
                    self.settings.library_folders
                );
                let db = self.db.clone();
                let folders = self.settings.library_folders.clone();
                Task::perform(
                    async move {
                        let mut total_summary = core_engine::ScanSummary::default();
                        for folder_str in folders {
                            let path = PathBuf::from(folder_str);
                            if let Ok(summary) =
                                db.scan_directory(&path, None::<fn(usize, usize)>).await
                            {
                                total_summary.scanned_files += summary.scanned_files;
                                total_summary.inserted_tracks += summary.inserted_tracks;
                                total_summary.updated_tracks += summary.updated_tracks;
                                total_summary.failed_files += summary.failed_files;
                            }
                        }
                        total_summary
                    },
                    Message::ScanFinished,
                )
            }

            Message::ClearLibrary => {
                info!("Clearing all tracks from library database");
                let db = self.db.clone();
                self.library.clear();
                self.albums.clear();
                self.artists.clear();
                self.now_playing.clear();
                self.current_track = None;
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
                let db = self.db.clone();
                Task::batch([
                    Task::perform(
                        async move { db.get_library_folders().await.unwrap_or_default() },
                        Message::LibraryFoldersLoaded,
                    ),
                    self.reload_after_scan_task(),
                    self.load_storage_stats_task(),
                ])
            }

            Message::StorageStatsLoaded(stats, db_size_bytes) => {
                self.cache_stats = stats;
                self.db_size_bytes = db_size_bytes;
                Task::none()
            }

            Message::Settings(settings_msg) => {
                self.settings.update(settings_msg);
                Task::none()
            }

            Message::SelectSettingsTab(tab) => {
                self.settings.select_tab(tab);
                Task::batch([self.save_config_task(), self.load_storage_stats_task()])
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
                Task::batch([self.reload_tracks_task(), self.save_config_task()])
            }

            Message::SelectArtist(artist_name) => {
                self.active_tab = ViewTab::Tracks;
                self.library.set_filter(TrackFilter::artist(artist_name));
                self.reload_tracks_task()
            }

            // ── Playlists ───────────────────────────────────────────────────
            Message::PlaylistSectionChanged(section) => {
                self.playlists.set_section(section);
                Task::none()
            }

            Message::PlaylistsLoaded(playlists) => {
                self.playlists.set_playlists(playlists);
                Task::none()
            }

            Message::SelectPlaylist(id) => {
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
            },

            Message::PlayPlaylist(id) => self.open_playlist(id, 0),
            Message::PlayPlaylistItem(id, index) => self.open_playlist(id, index),
            Message::PlaylistToQueue(id) => {
                self.resolve_playlist_intent(id, PlaylistOpenIntent::AddToQueue)
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
                let db = self.db.clone();
                Task::perform(
                    async move {
                        let path = unique_playlist_path(&name);
                        if let Err(e) = write_m3u(&path, &[]) {
                            warn!("Failed to create playlist file {:?}: {}", path, e);
                        }
                        let path_str = path.to_string_lossy().to_string();
                        let _ = db.upsert_playlist_file(&path_str, &name, 0).await;
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
                self.save_config_task()
            }

            Message::SetPlaylistsViewMode(mode) => {
                self.playlists.set_view_mode(mode);
                self.save_config_task()
            }

            Message::ImportPlaylistsClicked => Task::perform(
                async {
                    let files = rfd::AsyncFileDialog::new()
                        .set_title("Import Playlists")
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
                Task::batch([persist, load, snap_queue_scroll_top()])
            }
            Message::QueueDelete(id) => {
                let was_active = self.queues.active == Some(id);
                let was_playing_deleted = was_active && self.current_track_id.is_some();
                let fallback = self.queues.remove(id);
                self.playlists.reset_queue_window();
                if was_active && was_playing_deleted {
                    self.current_track = None;
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
                if let Some(next) = fallback {
                    let load = Task::perform(
                        async move {
                            let tracks =
                                db.load_queue_tracks(next as i64).await.unwrap_or_default();
                            (next, tracks)
                        },
                        |(id, tracks)| Message::QueueActivated { id, tracks },
                    );
                    Task::batch([persist, load, snap_queue_scroll_top()])
                } else {
                    Task::batch([persist, snap_queue_scroll_top()])
                }
            }
            Message::QueueActivated { id, tracks } => {
                self.queues.activate_with_tracks(id, tracks);
                self.playlists.reset_queue_window();
                snap_queue_scroll_top()
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
            Message::QueueAddTrack(id) => {
                let db = self.db.clone();
                Task::perform(
                    async move {
                        db.get_track_rows_by_ids(&[id])
                            .await
                            .unwrap_or_default()
                            .remove(&id)
                            .into_iter()
                            .collect::<Vec<_>>()
                    },
                    Message::QueueAppendLoaded,
                )
            }
            Message::QueueAppendLoaded(tracks) => {
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
            Message::QueuePlayIndex(index) => {
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
                        Task::batch([play, self.persist_active_current_task()])
                    }
                    None => Task::none(),
                }
            }
            Message::QueueRemoveTrack { queue_id, index } => {
                if self.queues.active == Some(queue_id) {
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.remove(index);
                    }
                }
                self.queues.touch_active_summary();
                self.persist_active_queue_task()
            }
            Message::QueueMoveTrack {
                queue_id,
                index,
                up,
            } => {
                if self.queues.active == Some(queue_id) {
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.move_track(index, up);
                    }
                }
                self.queues.touch_active_summary();
                self.persist_active_queue_task()
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
                let save_task = self.save_config_task();
                let covers_task = self.load_visible_album_covers_task();
                Task::batch([save_task, covers_task])
            }

            Message::ArtistSortChanged(field) => {
                self.artists.toggle_sort(field);
                self.save_config_task()
            }

            Message::AlbumsLoaded(albums) => {
                self.albums.set_albums(albums);
                // The catalog was (re)loaded at offset 0; keep the scrollable in
                // sync so the window and the viewport cannot disagree.
                let snap = iced::widget::scrollable::snap_to(
                    iced::widget::scrollable::Id::new("albums_scroll"),
                    iced::widget::scrollable::RelativeOffset::START,
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
        }
    }

    pub fn get_next_track(&self) -> Option<&TrackRow> {
        if let Some(queue) = self.queues.active_queue() {
            if self.is_shuffle && !queue.is_empty() {
                let idx = (self.position_ms as usize ^ 0x9e3779b9) % queue.len();
                return queue.tracks.get(idx);
            }
            if let Some(idx) = queue.next_index() {
                return queue.tracks.get(idx);
            }
            return None;
        }
        self.library
            .get_next_row(self.current_track_id, self.is_shuffle, self.position_ms)
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
        scrollable::snap_to(
            scrollable::Id::new("queue_scroll"),
            RelativeOffset { x: 0.0, y },
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

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            active_tab: self.active_tab,
            tab_order: self.tab_order.clone(),
            settings_tab: self.settings.active_tab,
            theme_name: self.theme_manager.active_theme_name.clone(),
            primary_font: self.theme_manager.current_config.font_family.clone(),
            library_view_mode: ViewMode::List,
            albums_view_mode: self.albums.view_mode,
            artists_view_mode: self.artists.view_mode,
            playlists_view_mode: self.playlists.view_mode,
            page_size: self.library.page_size,
            tile_width: self.albums.tile_width,
            album_tile_width: self.albums.tile_width,
            artist_tile_width: self.artists.tile_width,
            now_playing_cover_size: self.now_playing.cover_size,
            album_sort_field: self.albums.sort_field,
            artist_sort_field: self.artists.sort_field,
            playlist_sort_field: self.playlists.sort_field,
            volume: self.volume,
            is_shuffle: self.is_shuffle,
            is_repeat: self.is_repeat,
            scrobble: self.scrobble_config.clone(),
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

    pub fn view(&self) -> Element<'_, Message> {
        let theme_cfg = &self.theme_manager.current_config;

        // Top Navigation & Tab Bar
        let is_any_dragging = self.dragging_tab.is_some();

        let tab_buttons = self.tab_order.iter().map(|&tab| -> Element<'_, Message> {
            let is_selected = self.active_tab == tab;
            let is_dragging = self.dragging_tab == Some(tab);
            let is_hovered = self.hovered_tab == Some(tab);

            let tab_icon_color = if is_selected || is_dragging {
                theme_cfg.primary_color()
            } else if is_hovered {
                theme_cfg.text_color()
            } else {
                theme_cfg.text_muted_color()
            };

            let tab_inner = row![
                icons::icon(tab.icon_svg(), 12.0, tab_icon_color),
                text(theme_cfg, tab.label())
                    .size(11)
                    .color(if is_selected || is_dragging {
                        theme_cfg.primary_color()
                    } else if is_hovered {
                        theme_cfg.text_color()
                    } else {
                        theme_cfg.text_muted_color()
                    }),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center);

            let tab_container =
                container(tab_inner)
                    .padding([5, 10])
                    .style(styles::header_tab_container(
                        theme_cfg,
                        is_selected,
                        is_hovered,
                        is_dragging,
                    ));

            mouse_area(tab_container)
                .on_press(Message::TabDragStarted(tab))
                .on_release(Message::TabDragReleased)
                .on_enter(if is_any_dragging {
                    Message::TabDraggedOver(tab)
                } else {
                    Message::TabHovered(Some(tab))
                })
                .on_exit(Message::TabHovered(None))
                .interaction(iced::mouse::Interaction::Pointer)
                .into()
        });

        let settings_btn = button(icons::icon(
            icons::SETTINGS_SVG,
            14.0,
            if self.active_tab == ViewTab::Settings {
                theme_cfg.primary_color()
            } else {
                theme_cfg.text_muted_color()
            },
        ))
        .padding([4, 8])
        .style(styles::icon_control_button(theme_cfg))
        .on_press(Message::SelectTab(ViewTab::Settings));

        let mut nav_row = row![].spacing(4).align_y(iced::Alignment::Center);
        for btn in tab_buttons {
            nav_row = nav_row.push(btn);
        }
        nav_row = nav_row.push(Space::with_width(Length::Fill));
        nav_row = nav_row.push(settings_btn);

        let top_bar = container(column![
            nav_row.padding([4, 12]).height(32),
            border_vert(1.0, theme_cfg.border_color()),
        ])
        .style(styles::top_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(33);

        let main_view: Element<'_, Message> = match self.active_tab {
            ViewTab::Tracks => self
                .library
                .view(theme_cfg, self.current_track_id, self.is_playing),
            ViewTab::Albums => self.albums.view(theme_cfg),
            ViewTab::Artists => self.artists.view(theme_cfg),
            ViewTab::Playlists => self.playlists.view(theme_cfg, &self.queues),
            ViewTab::Lyrics => {
                let next_track = self.get_next_track();

                self.now_playing.view(
                    theme_cfg,
                    self.current_track.as_ref(),
                    self.current_cover.as_ref(),
                    self.current_blurred_cover.as_ref(),
                    next_track,
                    self.current_next_cover.as_ref(),
                    self.is_repeat,
                    self.is_shuffle,
                    self.current_tech_info.as_ref(),
                )
            }
            ViewTab::Settings => {
                let cache_stats = self.cache_stats;
                let db_size_bytes = self.db_size_bytes;

                self.settings.view(
                    theme_cfg,
                    views::settings::SettingsContext {
                        theme_names: self.theme_manager.theme_names(),
                        active_theme_name: &self.theme_manager.active_theme_name,
                        albums_view_mode: self.albums.view_mode,
                        artists_view_mode: self.artists.view_mode,
                        page_size: self.library.page_size,
                        album_tile_width: self.albums.tile_width,
                        artist_tile_width: self.artists.tile_width,
                        now_playing_cover_size: self.now_playing.cover_size,
                        primary_font: self.theme_manager.current_config.font_family.as_deref(),
                        total_tracks: self.library.total_tracks,
                        total_albums: self.albums.len(),
                        total_artists: self.artists.len(),
                        db_size_bytes,
                        cache_file_count: cache_stats.file_count,
                        cache_size_bytes: cache_stats.total_bytes,
                        is_scanning: self.is_scanning,

                        listenbrainz_enabled: self.scrobble_config.listenbrainz_enabled,
                        listenbrainz_token: &self.scrobble_config.listenbrainz_token,
                        listenbrainz_api_url: &self.scrobble_config.listenbrainz_api_url,
                        listenbrainz_status: self.listenbrainz_plugin.status(),
                        listenbrainz_scrobbles: self.listenbrainz_plugin.scrobble_count(),

                        lastfm_enabled: self.scrobble_config.lastfm_enabled,
                        lastfm_api_key: &self.scrobble_config.lastfm_api_key,
                        lastfm_api_secret: &self.scrobble_config.lastfm_api_secret,
                        lastfm_session_key: &self.scrobble_config.lastfm_session_key,
                        lastfm_api_url: &self.scrobble_config.lastfm_api_url,
                        lastfm_status: self.lastfm_plugin.status(),
                        lastfm_scrobbles: self.lastfm_plugin.scrobble_count(),

                        submit_now_playing: self.scrobble_config.submit_now_playing,
                        scrobble_percent: self.scrobble_config.scrobble_percent,
                    },
                )
            }
        };

        let body = container(main_view)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true);

        let player_bar = self.player_bar.view(
            theme_cfg,
            self.current_track.as_ref(),
            self.current_cover.as_ref(),
            self.is_playing,
            self.position_ms,
            self.duration_ms,
            self.volume,
            self.is_shuffle,
            self.is_repeat,
        );

        let status_bar_ctx = StatusBarContext {
            active_tab: self.active_tab,
            total_tracks: self.library.total_tracks,
            search_query: &self.library.search_query,
            albums: &self.albums.albums,
            album_tracks: self.albums.total_tracks,
            album_duration_ms: self.albums.total_duration_ms,
            artists: &self.artists.artists,
            artist_albums: self.artists.total_albums,
            artist_tracks: self.artists.total_tracks,
            current_track: self.current_track.as_ref(),
            is_playing: self.is_playing,
            playlist_count: self.playlists.len(),
            queue_count: self.queues.summaries.len(),
            active_queue: self
                .queues
                .active_queue()
                .map(|q| (q.name.as_str(), q.len())),
            is_scanning: self.is_scanning,
        };

        let status_bar = StatusBar::view(theme_cfg, status_bar_ctx);

        let full_layout = column![top_bar, body, player_bar, status_bar]
            .width(Length::Fill)
            .height(Length::Fill);

        container(full_layout)
            .style(styles::main_background(theme_cfg))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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

    PlaylistResolution { items, tracks }
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
    scrollable::snap_to(scrollable::Id::new("queue_scroll"), RelativeOffset::START)
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
