use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use audio::{AudioController, AudioEngine};
use crossbeam_channel::Receiver;
use db::LibraryDatabase;
use iced::futures::SinkExt;
use iced::widget::{container, image, Space, Widget};
use iced::{Background, Element, Subscription, Task, Theme};
use model::{Track, TrackRow};
#[cfg(target_os = "linux")]
use plugin::media_controls::MprisPlugin;
#[cfg(target_os = "windows")]
use plugin::media_controls::WindowsMediaPlugin;
use plugin::{
    scrobbler::{LastFmPlugin, ListenBrainzPlugin},
    PlayerAction, PluginManager,
};
use tracing::{error, info};

use crate::message::Message;
use crate::views::{
    AlbumsView, ArtistsView, LibraryView, NowPlayingView, PlayerBar, PlaylistsView, SettingsView,
};
use ::queue::QueueStore;
use component::ContextMenuState;
use config::{AppConfig, AppState, ScrobbleConfig, SettingsTab, ViewTab};
use layout::LayoutManager;
use theme::ThemeManager;

mod context_menu;
mod library;
mod persistence;
mod playback;
mod playlists;
mod queue;
mod settings;
mod window;
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
    pub(crate) db_path: &'static Path,
    pub(crate) config_path: &'static Path,
    pub(crate) state_path: &'static Path,
    pub(crate) _audio_engine: AudioEngine,
    pub(crate) audio_controller: AudioController,
    pub(crate) plugin_manager: PluginManager,
    pub(crate) theme_manager: ThemeManager,
    pub(crate) layout_manager: LayoutManager,
    pub(crate) fonts: font::FontConfig,

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
    pub(crate) current_now_playing_cover: Option<image::Handle>,
    pub(crate) current_blurred_cover: Option<image::Handle>,
    pub(crate) current_next_cover: Option<image::Handle>,
    pub(crate) current_tech_info: Option<metadata::AudioTechnicalInfo>,
    pub(crate) is_playing: bool,
    pub(crate) position_ms: u64,
    pub(crate) duration_ms: u64,
    pub(crate) volume: f32,
    pub(crate) is_shuffle: bool,
    pub(crate) repeat_mode: model::RepeatMode,
    pub(crate) gapless: bool,
    pub(crate) output_device: Option<String>,
    pub(crate) available_output_devices: Vec<String>,
    pub(crate) resampling_config: audio::ResamplingConfig,
    pub(crate) pending_gapless_next: Option<TrackRow>,

    // Global Scanning Telemetry
    pub(crate) is_scanning: bool,
    pub(crate) scan_progress: Option<(usize, usize)>,
    pub(crate) is_generating_thumbnails: bool,

    // Pre-rendered Status Bar Labels
    pub(crate) status_now_playing_title: String,
    pub(crate) status_now_playing_duration: String,
    pub(crate) status_queue_name: String,
    pub(crate) status_queue_tracks: String,
    pub(crate) status_playlist_count: String,
    pub(crate) status_queue_count: String,

    // Library Watcher and Directory Monitoring
    pub(crate) watch_library: bool,
    pub(crate) library_watcher: Option<metadata::LibraryWatcher>,
    pub(crate) watcher_rx: Option<crossbeam_channel::Receiver<Vec<PathBuf>>>,
    pub(crate) pending_watcher_paths: Vec<PathBuf>,

    // Scrobbling Plugins
    pub(crate) listenbrainz_plugin: Arc<ListenBrainzPlugin>,
    pub(crate) lastfm_plugin: Arc<LastFmPlugin>,
    pub(crate) scrobble_config: ScrobbleConfig,

    pub(crate) config_save_seq: u64,
    pub(crate) state_save_seq: u64,
    pub(crate) search_seq: u64,

    pub(crate) cache_stats: thumbnail::CacheStats,
    pub(crate) db_size_bytes: u64,

    pub(crate) media_controls_rx: Option<Receiver<PlayerAction>>,

    pub(crate) context_menu: Option<ContextMenuState>,
    pub(crate) duplicate_prompt: Option<DuplicatePromptState>,
    pub(crate) window_size: iced::Size,
    pub(crate) main_window_id: Option<iced::window::Id>,
    pub(crate) is_maximized: bool,
    pub(crate) custom_titlebar: bool,
    pub(crate) minimize_to_tray: bool,
    pub(crate) tray: Option<crate::tray::TrayController>,
    #[cfg(target_os = "windows")]
    pub(crate) windows_media_plugin: Option<std::sync::Arc<WindowsMediaPlugin>>,
    pub(crate) last_titlebar_press: Option<std::time::Instant>,
    pub(crate) pending_playlist_tracks: Vec<TrackRow>,
}

impl App {
    pub fn new_with(config: AppConfig, state: AppState) -> (Self, Task<Message>) {
        let t_new = std::time::Instant::now();
        info!("Initializing Melody Native Player application");

        let db_path = path::database_file();
        info!("Using SQLite database at: {:?}", db_path);

        let db = tokio::runtime::Handle::current().block_on(async {
            LibraryDatabase::new(&db_path).await.unwrap_or_else(|e| {
                error!("Failed to open disk database, falling back to in-memory: {e}");
                tokio::runtime::Handle::current()
                    .block_on(async { LibraryDatabase::in_memory().await })
                    .expect("In-memory database failed")
            })
        });

        let audio_engine = AudioEngine::new();
        let audio_controller = audio_engine.controller();
        audio_controller.set_volume(state.volume);
        audio_controller.set_resampling(config.resampling.clone());
        audio_controller.set_output_device(config.output_device.clone());
        let available_output_devices = audio::list_output_devices();

        let plugin_manager = PluginManager::new();

        let listenbrainz_plugin = Arc::new(ListenBrainzPlugin::new(
            config.scrobble.to_listenbrainz_config(),
        ));
        let lastfm_plugin = Arc::new(LastFmPlugin::new(config.scrobble.to_lastfm_config()));
        plugin_manager.register_plugin(listenbrainz_plugin.clone());
        plugin_manager.register_plugin(lastfm_plugin.clone());

        #[cfg(target_os = "linux")]
        let (media_plugin, media_controls_rx) = {
            let (tx, rx) = crossbeam_channel::unbounded();
            (std::sync::Arc::new(MprisPlugin::new(tx)), Some(rx))
        };
        #[cfg(target_os = "linux")]
        plugin_manager.register_plugin(media_plugin.clone());

        #[cfg(target_os = "windows")]
        let (windows_media_plugin, media_controls_rx) = {
            let (tx, rx) = crossbeam_channel::unbounded();
            (std::sync::Arc::new(WindowsMediaPlugin::new(tx)), Some(rx))
        };
        #[cfg(target_os = "windows")]
        plugin_manager.register_plugin(windows_media_plugin.clone());

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let media_controls_rx = None;

        let mut theme_manager = ThemeManager::new();
        theme_manager.set_theme(&config.theme_name);

        let mut layout_manager = LayoutManager::new();
        layout_manager.select_layout(&config.layout_name);

        let config_path = path::config_file();
        let state_path = path::state_file();

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
        let now_playing = NowPlayingView::new();
        let playlists = PlaylistsView::new(state.playlists_view_mode, state.playlist_sort_field);
        let settings = SettingsView::new(state.settings_tab, Vec::new());
        let mut player_bar = PlayerBar::new();
        player_bar.update_volume(state.volume);
        player_bar.update_position(0, 0);

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

        let (main_window_id, open_window_task) = iced::window::open(crate::build_window_settings(
            &config,
            state.window_width,
            state.window_height,
        ));
        let tray = if config.minimize_to_tray {
            crate::tray::TrayController::new()
        } else {
            None
        };

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
            fonts: config.fonts,

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
            current_now_playing_cover: None,
            current_blurred_cover: None,
            current_next_cover: None,
            current_tech_info: None,
            is_playing: false,
            position_ms: 0,
            duration_ms: 0,
            volume: state.volume,
            is_shuffle: state.is_shuffle,
            repeat_mode: state.repeat_mode,
            gapless: config.gapless,
            output_device: config.output_device,
            available_output_devices,
            resampling_config: config.resampling,
            pending_gapless_next: None,

            is_scanning: false,
            scan_progress: None,
            is_generating_thumbnails: false,

            status_now_playing_title: String::new(),
            status_now_playing_duration: String::new(),
            status_queue_name: String::new(),
            status_queue_tracks: String::new(),
            status_playlist_count: String::new(),
            status_queue_count: String::new(),

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
            cache_stats: thumbnail::CacheStats::default(),
            db_size_bytes: 0,

            media_controls_rx,

            context_menu: None,
            duplicate_prompt: None,
            window_size: iced::Size::new(
                state.window_width.max(config::WINDOW_MIN_WIDTH),
                state.window_height.max(config::WINDOW_MIN_HEIGHT),
            ),
            main_window_id: Some(main_window_id),
            is_maximized: false,
            custom_titlebar: config.custom_titlebar,
            minimize_to_tray: config.minimize_to_tray,
            tray,
            #[cfg(target_os = "windows")]
            windows_media_plugin: Some(windows_media_plugin),
            last_titlebar_press: None,
            pending_playlist_tracks: Vec::new(),
        };

        app.playlists.recompute_rendered_queue(
            &app.layout_manager.current_template.view_formats.queue,
            &app.queues,
        );

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
            Task::perform(updater::check_for_updates(), Message::UpdateCheckFinished),
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

        initial_tasks.push(open_window_task.map(Message::WindowOpened));

        app.sync_status_strings();
        let initial_task = Task::batch(initial_tasks);

        info!(
            "App loaded in {:?} (+{:?} since process start)",
            t_new.elapsed(),
            path::elapsed_since_app_start()
        );

        (app, initial_task)
    }

    pub fn title(&self, _window: iced::window::Id) -> String {
        let base = match &self.current_track {
            Some(track) => format!("{} - {} • Melody", track.title, track.artist),
            None => "Melody".to_string(),
        };

        #[cfg(debug_assertions)]
        let base = format!("{base} [DEBUG]");

        base
    }

    pub fn theme(&self, _window: iced::window::Id) -> Theme {
        self.theme_manager.theme()
    }

    pub(crate) fn cursor_position(&self) -> iced::Point {
        let x = f32::from_bits(LAST_CURSOR_X.load(std::sync::atomic::Ordering::Relaxed));
        let y = f32::from_bits(LAST_CURSOR_Y.load(std::sync::atomic::Ordering::Relaxed));
        iced::Point::new(x, y)
    }

    pub fn subscription(&self) -> Subscription<Message> {
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
            Message::AudioTick
            | Message::PlayTrack(_)
            | Message::TrackResolvedForPlayback(_)
            | Message::TrackResolvedForRestore(_)
            | Message::TrackDetailsLoaded(_)
            | Message::PlaybackAssetsLoaded { .. }
            | Message::NowPlayingCoverLoaded { .. }
            | Message::TogglePlayPause
            | Message::Stop
            | Message::NextTrack
            | Message::ToggleShuffle
            | Message::ToggleRepeat
            | Message::SetRepeatMode(_)
            | Message::ToggleGapless(_)
            | Message::SetOutputDevice(_)
            | Message::RefreshOutputDevices
            | Message::SetResamplingFilter(_)
            | Message::SetResamplingQuality(_)
            | Message::ToggleResamplingLinearInterp(_)
            | Message::PreviousTrack
            | Message::Seek(_)
            | Message::SeekFinished
            | Message::SeekToMs(_)
            | Message::VolumeChanged(_)
            | Message::MediaControlAction(_) => self.handle_playback_message(message),

            Message::SelectTab(_)
            | Message::TabDragStarted(_)
            | Message::TabDraggedOver(_)
            | Message::TabDragReleased
            | Message::TabHovered(_)
            | Message::WindowResized(_)
            | Message::TitlebarPressed
            | Message::WindowDragResize(_)
            | Message::WindowMinimize
            | Message::WindowToggleMaximize
            | Message::WindowMaximizedChanged(_)
            | Message::WindowClose
            | Message::WindowOpened(_)
            | Message::WindowClosed(_)
            | Message::WindowShowSystemMenu
            | Message::ToggleCustomTitlebar(_)
            | Message::ToggleMinimizeToTray(_)
            | Message::TrayAction(_) => self.handle_window_message(message),

            Message::SearchInputChanged(_)
            | Message::SearchDebounced(_)
            | Message::SortChanged(_)
            | Message::SetPageSize(_)
            | Message::SetAlbumsViewMode(_)
            | Message::SetArtistsViewMode(_)
            | Message::SetAlbumTileWidth(_)
            | Message::SetArtistTileWidth(_)
            | Message::TrackClicked(_)
            | Message::AddLibraryFolderClicked
            | Message::LibraryFolderAdded(_)
            | Message::RemoveLibraryFolder(_)
            | Message::LibraryFoldersLoaded(_)
            | Message::RescanLibraryFolder(_)
            | Message::ReindexLibrary
            | Message::ScanLibraryForNewFiles
            | Message::ToggleWatchLibrary(_)
            | Message::ScanProgress { .. }
            | Message::ClearLibrary
            | Message::ClearThumbnailCache
            | Message::ScanFinished(_)
            | Message::ThumbnailsGenerated
            | Message::StorageStatsLoaded(_, _)
            | Message::SelectAlbum(_)
            | Message::SelectArtist(_)
            | Message::TracksLoaded(_, _)
            | Message::LibraryScrolled(_)
            | Message::TrackWindowLoaded { .. }
            | Message::AlbumSortChanged(_)
            | Message::ArtistSortChanged(_)
            | Message::AlbumsLoaded(_)
            | Message::AlbumsScrolled(_)
            | Message::EnsureAlbumCovers
            | Message::ArtistsLoaded(_)
            | Message::ArtistsScrolled(_)
            | Message::AlbumCoverChunkLoaded { .. } => self.handle_library_message(message),

            Message::OpenTrackContextMenu { .. }
            | Message::TrackContextMenuReady { .. }
            | Message::OpenAlbumContextMenu(_)
            | Message::OpenArtistContextMenu(_)
            | Message::OpenPlaylistContextMenu(_)
            | Message::OpenQueueContextMenu { .. }
            | Message::OpenPlayerBarContextMenu
            | Message::ContextMenu(_)
            | Message::QueuePlayTracksNext(_)
            | Message::QueueAddTracks(_)
            | Message::NewPlaylistWithTracks(_)
            | Message::ConfirmDuplicatePrompt
            | Message::CancelDuplicatePrompt
            | Message::EscapePressed => self.handle_context_menu_message(message),

            Message::PlaylistSectionChanged(_)
            | Message::PlaylistsLoaded(_)
            | Message::PlaylistCoversLoaded(_)
            | Message::SelectPlaylist(_)
            | Message::PlaylistResolved { .. }
            | Message::PlayPlaylist(_)
            | Message::PlayPlaylistItem(_, _)
            | Message::PlaylistPlayNext(_)
            | Message::PlaylistToQueue(_)
            | Message::PlaylistDetailClicked { .. }
            | Message::PlaylistDetailDragStarted { .. }
            | Message::PlaylistDetailDraggedOver { .. }
            | Message::PlaylistDetailDragReleased
            | Message::PlaylistDetailHovered(_)
            | Message::PlaylistRemoveTrack { .. }
            | Message::PlaylistDetailPersisted { .. }
            | Message::NewPlaylistClicked
            | Message::NewPlaylistNameChanged(_)
            | Message::NewPlaylistCancelled
            | Message::NewPlaylistNameSubmitted(_)
            | Message::RenamePlaylist(_)
            | Message::RenamePlaylistNameChanged(_)
            | Message::RenamePlaylistCancelled
            | Message::RenamePlaylistSubmitted(_, _)
            | Message::DeletePlaylist(_)
            | Message::PlaylistSortChanged(_)
            | Message::SetPlaylistsViewMode(_)
            | Message::ImportPlaylistsClicked
            | Message::PlaylistsImported(_) => self.handle_playlists_message(message),

            Message::QueueNew
            | Message::QueueSelect(_)
            | Message::QueueDelete(_)
            | Message::QueueActivated { .. }
            | Message::QueueScrolled(_)
            | Message::LibraryQueueBuilt { .. }
            | Message::PlayScope { .. }
            | Message::QueueRemoveTrack { .. }
            | Message::QueueClicked { .. }
            | Message::QueueDragStarted { .. }
            | Message::QueueDraggedOver { .. }
            | Message::QueueDragReleased
            | Message::QueueRowHovered(_)
            | Message::QueueSaveClicked
            | Message::QueueSaveNameChanged(_)
            | Message::QueueSaveCancelled
            | Message::QueueSaveSubmitted(_)
            | Message::QueuesPersisted => self.handle_queue_message(message),

            Message::SelectLyricsFont(_)
            | Message::SetLyricsFontSize(_)
            | Message::SelectSettingsTab(_)
            | Message::SetDebugSubpage(_)
            | Message::DebugNoop
            | Message::DebugToggle(_)
            | Message::DebugSliderChanged(_)
            | Message::DebugInputChanged(_)
            | Message::DebugPickSelected(_)
            | Message::DebugChipSelected(_)
            | Message::DebugViewModeChanged(_)
            | Message::DebugSortChanged(_)
            | Message::SelectTheme(_)
            | Message::SelectLayout(_)
            | Message::ReloadThemesAndLayouts
            | Message::SelectFont(_)
            | Message::FontsLoaded(_)
            | Message::ToggleListenBrainz(_)
            | Message::ListenBrainzTokenChanged(_)
            | Message::ListenBrainzUrlChanged(_)
            | Message::ToggleLastFm(_)
            | Message::LastFmApiKeyChanged(_)
            | Message::LastFmApiSecretChanged(_)
            | Message::LastFmSessionKeyChanged(_)
            | Message::LastFmUrlChanged(_)
            | Message::ToggleSubmitNowPlaying(_)
            | Message::SetScrobblePercent(_)
            | Message::TestScrobble(_)
            | Message::ScrobbleTestFinished(_, _)
            | Message::CheckForUpdates
            | Message::UpdateCheckFinished(_)
            | Message::StartUpdateDownload(_)
            | Message::UpdateDownloadProgress(_)
            | Message::UpdateDownloadFinished(_)
            | Message::ApplyUpdateAndRestart
            | Message::OpenUrl(_) => self.handle_settings_message(message),

            Message::SaveConfig
            | Message::DebouncedConfigSave(_)
            | Message::ConfigSaved
            | Message::SaveState
            | Message::DebouncedStateSave(_)
            | Message::StateSaved => self.handle_persistence_message(message),
        }
    }

    pub fn view(&self, window: iced::window::Id) -> Element<'_, Message> {
        if self.main_window_id != Some(window) {
            return container(Space::new()).boxed();
        }
        let theme_cfg = &self.theme_manager.current_config;
        let layout_view = self.render_layout(theme_cfg);

        let base_view: Element<'_, Message> = if self.use_custom_titlebar()
            && !self
                .layout_manager
                .current_template
                .root
                .has_top_horizontal_navbar()
        {
            let titlebar = crate::views::TitlebarView::view(
                theme_cfg,
                self.fonts.primary_font(),
                self.title(window),
                self.is_maximized,
            );
            iced::widget::column![titlebar, layout_view]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .boxed()
        } else {
            layout_view
        };

        // On Windows, winit's `undecorated_shadow` shifts the unmaximized client rect
        // down by 1px in WM_NCCALCSIZE (`top += 1; bottom += 1`), clipping the bottom 1px
        // of the window. Reserve 1px at the bottom so the status bar stays centered.
        let base_view: Element<'_, Message> =
            if cfg!(target_os = "windows") && self.use_custom_titlebar() && !self.is_maximized {
                iced::widget::container(base_view)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 1.0,
                        left: 0.0,
                    })
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .boxed()
            } else {
                base_view
            };

        let mut layers = vec![base_view];
        if let Some(ref menu) = self.context_menu {
            layers.push(component::render_context_menu(
                menu,
                &self.playlists.playlists,
                theme_cfg,
                self.fonts.primary_font(),
                self.window_size,
                Message::ContextMenu,
            ));
        }
        if let Some(ref prompt) = self.duplicate_prompt {
            layers.push(self.render_duplicate_prompt(prompt, theme_cfg));
        }
        if self.use_custom_titlebar() && !self.is_maximized {
            layers.push(crate::views::resize_handles_overlay());
        }
        iced::widget::stack(layers).boxed()
    }
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

static LAST_CURSOR_X: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(100.0f32.to_bits());
static LAST_CURSOR_Y: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(100.0f32.to_bits());

fn handle_cursor_and_window_event(
    event: iced::Event,
    _status: iced::event::Status,
    id: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            LAST_CURSOR_X.store(position.x.to_bits(), std::sync::atomic::Ordering::Relaxed);
            LAST_CURSOR_Y.store(position.y.to_bits(), std::sync::atomic::Ordering::Relaxed);
            None
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size))
        }
        iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::WindowClose),
        iced::Event::Window(iced::window::Event::Closed) => Some(Message::WindowClosed(id)),
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
