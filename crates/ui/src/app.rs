use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use audio::{AudioController, AudioEngine};
use core_engine::{LibraryDatabase, Track};
use crossbeam_channel::Receiver;
use iced::widget::{button, column, container, image, mouse_area, row, Space};
use iced::{Element, Length, Subscription, Task, Theme};
use plugins::scrobbler::{LastFmPlugin, ListenBrainzPlugin, ScrobbleService};
#[cfg(target_os = "linux")]
use plugins::MprisPlugin;
use plugins::{PlayerAction, PluginManager, TrackMetadata};
use tracing::{error, info};

use crate::components::{border_vert, text};
use crate::config::{AppConfig, ScrobbleConfig};
use crate::icons;
use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeManager};
use crate::views::{
    self, AlbumsView, ArtistsView, LibraryView, NowPlayingView, PlayerBar, SettingsView, StatusBar,
    StatusBarContext,
};

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
    now_playing: NowPlayingView,
    settings: SettingsView,
    player_bar: PlayerBar,

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
    loved_tracks: HashSet<i64>,

    // Global Scanning Telemetry
    is_scanning: bool,

    // Scrobbling Plugins
    listenbrainz_plugin: Arc<ListenBrainzPlugin>,
    lastfm_plugin: Arc<LastFmPlugin>,
    scrobble_config: ScrobbleConfig,

    mpris_rx: Option<Receiver<PlayerAction>>,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        info!("Initializing Vibecore Native Player application");

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

        let library = LibraryView::new(config.library_view_mode, config.page_size);
        let albums = AlbumsView::new(config.albums_view_mode, config.album_sort_field);
        let artists = ArtistsView::new(config.artists_view_mode, config.artist_sort_field);
        let now_playing = NowPlayingView::new();
        let settings = SettingsView::new(config.settings_tab, available_fonts);
        let player_bar = PlayerBar::new();

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
            now_playing,
            settings,
            player_bar,

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
            loved_tracks: HashSet::new(),

            is_scanning: false,

            listenbrainz_plugin,
            lastfm_plugin,
            scrobble_config: config.scrobble,

            mpris_rx,
        };

        let mut initial_tasks = vec![app.reload_tracks_task(), {
            let db = app.db.clone();
            Task::perform(
                async move { db.get_library_folders().await.unwrap_or_default() },
                Message::LibraryFoldersLoaded,
            )
        }];

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

        let initial_task = Task::batch(initial_tasks);

        (app, initial_task)
    }

    pub fn title(&self) -> String {
        match &self.current_track {
            Some(track) => format!("{} - {} | Vibecore", track.title, track.artist),
            None => "Vibecore".to_string(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme_manager.current_config.to_iced_theme()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Poll audio playback status at 50ms intervals (~20 FPS smooth seekbar)
        let ticker = iced::time::every(Duration::from_millis(50)).map(|_| Message::AudioTick);
        if self.dragging_tab.is_some() {
            let drag_sub = iced::event::listen_with(handle_drag_event);
            Subscription::batch([ticker, drag_sub])
        } else {
            ticker
        }
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
                // If the track is already in the currently visible window, play immediately without querying DB
                if let Some(track) = self.library.find_track(track_id).cloned() {
                    return self.update(Message::TrackResolvedForPlayback(Some(track)));
                }

                let db = self.db.clone();
                Task::perform(
                    async move { db.get_track_by_id(track_id).await.unwrap_or(None) },
                    Message::TrackResolvedForPlayback,
                )
            }

            Message::TrackResolvedForPlayback(maybe_track) => {
                if let Some(track) = maybe_track {
                    info!("Playing track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    let cover_bytes =
                        core_engine::get_or_create_thumbnail(&path, core_engine::THUMB_DIM_HIGH)
                            .or_else(|| core_engine::extract_cover_art(&path));

                    if let Some(ref bytes) = cover_bytes {
                        self.current_blurred_cover = core_engine::generate_blurred_cover(bytes)
                            .map(image::Handle::from_bytes);
                    } else {
                        self.current_blurred_cover = None;
                    }

                    self.current_cover = cover_bytes.map(image::Handle::from_bytes);
                    self.current_tech_info =
                        core_engine::extract_technical_info(&path, track.duration_ms);

                    self.audio_controller.load(path, true);
                    let track_id = track.id;
                    self.current_track = Some(track.clone());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = track.duration_ms;

                    // Pre-extract cover for next track for the Now Playing view
                    let next_track_opt = self.get_next_track();
                    self.current_next_cover = next_track_opt.and_then(|nt| {
                        let nt_path = PathBuf::from(&nt.file_path);
                        core_engine::get_or_create_thumbnail(
                            &nt_path,
                            core_engine::THUMB_DIM_DEFAULT,
                        )
                        .or_else(|| core_engine::extract_cover_art(&nt_path))
                        .map(image::Handle::from_bytes)
                    });

                    let lyrics = self
                        .plugin_manager
                        .fetch_lyrics(&track.artist, &track.title);
                    self.now_playing.set_lyrics(lyrics);

                    // Notify plugins
                    self.plugin_manager.on_track_change(&TrackMetadata {
                        title: track.title,
                        artist: track.artist,
                        album: track.album,
                        duration_ms: track.duration_ms,
                        file_path: track.file_path,
                    });
                }
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

            Message::ToggleLoveCurrentTrack => {
                if let Some(id) = self.current_track_id {
                    if self.loved_tracks.contains(&id) {
                        self.loved_tracks.remove(&id);
                        info!("Removed from loved tracks: {}", id);
                    } else {
                        self.loved_tracks.insert(id);
                        info!("Added to loved tracks: {}", id);
                    }
                }
                Task::none()
            }

            Message::PreviousTrack => {
                // If more than 3 seconds in, restart track
                if self.position_ms > 3000 {
                    self.audio_controller.seek(Duration::from_millis(0));
                    self.position_ms = 0;
                    return Task::none();
                }

                if let Some(cur_id) = self.current_track_id {
                    if let Some(prev) = self.library.get_previous_track(cur_id) {
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
                self.save_config_task()
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
                                    Message::TrackResolvedForPlayback,
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

            Message::MoveTab(tab, move_left) => {
                if let Some(idx) = self.tab_order.iter().position(|&t| t == tab) {
                    let new_idx = if move_left {
                        if idx > 0 {
                            idx - 1
                        } else {
                            return Task::none();
                        }
                    } else if idx + 1 < self.tab_order.len() {
                        idx + 1
                    } else {
                        return Task::none();
                    };
                    self.tab_order.remove(idx);
                    self.tab_order.insert(new_idx, tab);
                    return self.save_config_task();
                }
                Task::none()
            }

            Message::ResetTabOrder => {
                self.tab_order = crate::config::default_tab_order();
                self.save_config_task()
            }

            Message::SearchInputChanged(query) => {
                self.library.set_search_query(query);
                self.reload_tracks_task()
            }

            Message::SortChanged(field) => {
                self.library.toggle_sort(field);
                self.reload_tracks_task()
            }

            Message::SetPageSize(size) => {
                self.library.set_page_size(size);
                Task::batch([self.save_config_task(), self.reload_tracks_task()])
            }

            Message::SetLibraryViewMode(mode) => {
                self.library.set_view_mode(mode);
                self.save_config_task()
            }

            Message::SetAlbumsViewMode(mode) => {
                self.albums.set_view_mode(mode);
                self.save_config_task()
            }

            Message::SetArtistsViewMode(mode) => {
                self.artists.set_view_mode(mode);
                self.save_config_task()
            }

            Message::TrackClicked(id) => {
                if self.library.on_track_click(id) {
                    self.update(Message::PlayTrack(id))
                } else {
                    Task::none()
                }
            }

            Message::ScanFolderClicked => {
                self.is_scanning = true;
                Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Select Music Folder to Scan")
                            .pick_folder()
                            .await;
                        folder.map(|f| f.path().to_path_buf())
                    },
                    Message::FolderSelected,
                )
            }

            Message::FolderSelected(maybe_path) => match maybe_path {
                Some(path) => {
                    info!("Scanning folder: {:?}", path);
                    let path_str = path.to_string_lossy().to_string();
                    let db = self.db.clone();
                    Task::perform(
                        async move {
                            let _ = db.add_library_folder(&path_str).await;
                            let folders = db.get_library_folders().await.unwrap_or_default();
                            let summary = db
                                .scan_directory(&path, None::<fn(usize, usize)>)
                                .await
                                .unwrap_or_default();
                            (folders, summary)
                        },
                        |(_folders, summary)| {
                            // First update folders then process scan results
                            Message::ScanFinished(summary)
                        },
                    )
                }
                None => {
                    self.is_scanning = false;
                    Task::none()
                }
            },

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
                let _ = core_engine::clear_thumbnail_cache();
                self.albums.clear_covers();
                Task::none()
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
                ])
            }

            Message::Settings(settings_msg) => {
                self.settings.update(settings_msg);
                Task::none()
            }

            Message::SelectSettingsTab(tab) => {
                self.settings.select_tab(tab);
                self.save_config_task()
            }

            Message::SelectAlbum(album_name) => {
                self.albums.clear_covers();
                self.active_tab = ViewTab::Tracks;
                self.library.set_search_query(album_name);
                Task::batch([self.reload_tracks_task(), self.save_config_task()])
            }

            Message::SelectArtist(artist_name) => {
                self.active_tab = ViewTab::Tracks;
                self.library.set_search_query(artist_name);
                self.reload_tracks_task()
            }

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

            Message::TracksLoaded(tracks, total) => {
                self.library.set_tracks(tracks, total);
                Task::none()
            }

            Message::LibraryScrolled(viewport) => {
                if self.library.on_scroll(&viewport) {
                    return self.load_more_tracks_task();
                }
                Task::none()
            }

            Message::MoreTracksLoaded(tracks) => {
                self.library.append_tracks(tracks);
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
                if self.active_tab == ViewTab::Albums {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                }
            }

            Message::AlbumsScrolled(viewport) => {
                if self.albums.on_scroll(&viewport) {
                    return self.load_visible_album_covers_task();
                }
                Task::none()
            }

            Message::ArtistsScrolled(viewport) => {
                self.artists.on_scroll(&viewport);
                Task::none()
            }

            Message::AlbumCoverChunkLoaded(chunk_results, remaining) => {
                self.albums.insert_covers(chunk_results);
                let remaining: Vec<_> = remaining
                    .into_iter()
                    .filter(|(k, _)| self.albums.is_in_flight(k))
                    .collect();
                if remaining.is_empty() {
                    Task::none()
                } else {
                    self.load_cover_chunk_task(remaining)
                }
            }

            Message::ArtistsLoaded(artists) => {
                self.artists.set_artists(artists);
                Task::none()
            }

            Message::ConfigSaved => Task::none(),
        }
    }

    pub fn get_next_track(&self) -> Option<Track> {
        self.library.get_next_track(
            self.current_track_id,
            self.is_repeat,
            self.is_shuffle,
            self.position_ms,
            self.current_track.as_ref(),
        )
    }

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            active_tab: self.active_tab,
            tab_order: self.tab_order.clone(),
            settings_tab: self.settings.active_tab,
            theme_name: self.theme_manager.active_theme_name.clone(),
            primary_font: self.theme_manager.current_config.font_family.clone(),
            library_view_mode: self.library.view_mode,
            albums_view_mode: self.albums.view_mode,
            artists_view_mode: self.artists.view_mode,
            page_size: self.library.page_size,
            album_sort_field: self.albums.sort_field,
            artist_sort_field: self.artists.sort_field,
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

    fn load_cover_chunk_task(&self, mut pending: Vec<(String, PathBuf)>) -> Task<Message> {
        const CHUNK_SIZE: usize = 16;
        let (chunk, remaining) = if pending.len() <= CHUNK_SIZE {
            (pending, Vec::new())
        } else {
            let rest = pending.split_off(CHUNK_SIZE);
            (pending, rest)
        };

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut results = Vec::with_capacity(chunk.len());
                    for (key, path) in chunk {
                        let bytes = core_engine::get_or_create_thumbnail(
                            &path,
                            core_engine::THUMB_DIM_DEFAULT,
                        )
                        .unwrap_or_default();
                        results.push((key, bytes));
                    }

                    (results, remaining)
                })
                .await
                .unwrap_or_else(|_| (Vec::new(), Vec::new()))
            },
            |(results, remaining)| Message::AlbumCoverChunkLoaded(results, remaining),
        )
    }

    fn load_visible_album_covers_task(&mut self) -> Task<Message> {
        let to_load = self.albums.get_missing_covers_to_load();
        if to_load.is_empty() {
            return Task::none();
        }
        self.albums
            .mark_loading(to_load.iter().map(|(k, _)| k.clone()));
        self.load_cover_chunk_task(to_load)
    }

    fn reload_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let page_size = self.library.page_size;
        let query = if self.library.search_query.trim().is_empty() {
            None
        } else {
            Some(self.library.search_query.clone())
        };
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;

        Task::perform(
            async move {
                let total = db
                    .get_total_track_count(query.as_deref())
                    .await
                    .unwrap_or(0);
                let tracks = db
                    .get_tracks_window(0, page_size, query.as_deref(), sort_field, sort_dir)
                    .await
                    .unwrap_or_default();
                (tracks, total)
            },
            |(tracks, total)| Message::TracksLoaded(tracks, total),
        )
    }

    fn reload_after_scan_task(&mut self) -> Task<Message> {
        let db = self.db.clone();
        let page_size = self.library.page_size;
        let query = if self.library.search_query.trim().is_empty() {
            None
        } else {
            Some(self.library.search_query.clone())
        };
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;

        let mut tasks = vec![Task::perform(
            async move {
                let total = db
                    .get_total_track_count(query.as_deref())
                    .await
                    .unwrap_or(0);
                let tracks = db
                    .get_tracks_window(0, page_size, query.as_deref(), sort_field, sort_dir)
                    .await
                    .unwrap_or_default();
                (tracks, total)
            },
            |(tracks, total)| Message::TracksLoaded(tracks, total),
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

    fn load_more_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let offset = self.library.visible_tracks.len();
        let limit = self.library.page_size;
        let query = if self.library.search_query.trim().is_empty() {
            None
        } else {
            Some(self.library.search_query.clone())
        };
        let sort_field = self.library.sort_field;
        let sort_dir = self.library.sort_dir;

        Task::perform(
            async move {
                db.get_tracks_window(offset, limit, query.as_deref(), sort_field, sort_dir)
                    .await
                    .unwrap_or_default()
            },
            Message::MoreTracksLoaded,
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

            let tab_inner = row![
                icons::icon(
                    icons::DRAG_HANDLE_SVG,
                    10.0,
                    if is_dragging {
                        theme_cfg.primary_color()
                    } else if is_selected {
                        theme_cfg.primary_color().scale_alpha(0.7)
                    } else if is_hovered {
                        theme_cfg.text_color().scale_alpha(0.6)
                    } else {
                        theme_cfg.text_muted_color().scale_alpha(0.35)
                    },
                ),
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
            .spacing(5)
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
                .interaction(if is_dragging {
                    iced::mouse::Interaction::Grabbing
                } else {
                    iced::mouse::Interaction::Grab
                })
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
            ViewTab::Lyrics => {
                let next_track = self.get_next_track();
                let is_loved = self
                    .current_track_id
                    .map(|id| self.loved_tracks.contains(&id))
                    .unwrap_or(false);

                self.now_playing.view(
                    theme_cfg,
                    self.current_track.as_ref(),
                    self.current_cover.as_ref(),
                    self.current_blurred_cover.as_ref(),
                    next_track,
                    self.current_next_cover.as_ref(),
                    self.is_repeat,
                    self.is_shuffle,
                    is_loved,
                    self.current_tech_info.as_ref(),
                )
            }
            ViewTab::Settings => {
                let cache_stats = core_engine::get_thumbnail_cache_stats();
                let db_size_bytes = std::fs::metadata(&self.db_path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                self.settings.view(
                    theme_cfg,
                    views::settings::SettingsContext {
                        tab_order: &self.tab_order,
                        theme_names: self.theme_manager.theme_names(),
                        active_theme_name: &self.theme_manager.active_theme_name,
                        library_view_mode: self.library.view_mode,
                        albums_view_mode: self.albums.view_mode,
                        artists_view_mode: self.artists.view_mode,
                        page_size: self.library.page_size,
                        primary_font: self.theme_manager.current_config.font_family.as_deref(),
                        total_tracks: self.library.total_tracks,
                        total_albums: self.albums.len(),
                        total_artists: self.artists.len(),
                        db_path: &self.db_path,
                        config_path: &self.config_path,
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

        let is_loved = self
            .current_track_id
            .map(|id| self.loved_tracks.contains(&id))
            .unwrap_or(false);

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
            is_loved,
        );

        let status_bar_ctx = StatusBarContext {
            active_tab: self.active_tab,
            page: self.library.page,
            page_size: self.library.page_size,
            total_tracks: self.library.total_tracks,
            visible_count: self.library.visible_tracks.len(),
            search_query: &self.library.search_query,
            albums: &self.albums.albums,
            artists: &self.artists.artists,
            current_track: self.current_track.as_ref(),
            is_playing: self.is_playing,
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
