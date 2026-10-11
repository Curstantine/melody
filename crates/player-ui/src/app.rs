use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use directories::ProjectDirs;
use iced::widget::{button, column, container, image, row, Space};
use iced::{Element, Length, Subscription, Task, Theme};
use player_audio::{AudioController, AudioEngine};
use player_core::{AlbumSummary, ArtistSummary, LibraryDatabase, SortDirection, SortField, Track};
#[cfg(target_os = "linux")]
use player_plugins::MprisPlugin;
use player_plugins::scrobbler::{LastFmPlugin, ListenBrainzPlugin, ScrobbleService};
use player_plugins::{PlayerAction, PluginManager, TrackMetadata};
use tracing::{error, info};

use crate::components::{border_vert, text};
use crate::config::{AppConfig, ScrobbleConfig};
use crate::icons;
use crate::message::{AlbumSortField, ArtistSortField, Message, SettingsTab, ViewMode, ViewTab};
use crate::theme::{styles, ThemeManager};
use crate::views;

pub const DEFAULT_CATALOG_CHUNK_SIZE: usize = 60;

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
    settings_tab: SettingsTab,

    // View Modes & Selection
    library_view_mode: ViewMode,
    albums_view_mode: ViewMode,
    artists_view_mode: ViewMode,
    selected_track_id: Option<i64>,
    hovered_track_id: Option<i64>,
    last_track_click: Option<(i64, std::time::Instant)>,

    // Virtualized Library View State
    visible_tracks: Vec<Track>,
    total_tracks: usize,
    page: usize,
    page_size: usize,
    is_loading_more: bool,
    search_query: String,
    sort_field: SortField,
    sort_dir: SortDirection,

    // Catalogs & Folders
    albums: Vec<AlbumSummary>,
    album_covers: HashMap<String, image::Handle>,
    album_sort_field: AlbumSortField,
    album_sort_dir: SortDirection,
    visible_albums_count: usize,
    artists: Vec<ArtistSummary>,
    artist_sort_field: ArtistSortField,
    artist_sort_dir: SortDirection,
    visible_artists_count: usize,
    library_folders: Vec<String>,

    // Playback State
    current_track: Option<Track>,
    current_track_id: Option<i64>,
    current_cover: Option<image::Handle>,
    current_blurred_cover: Option<image::Handle>,
    current_next_cover: Option<image::Handle>,
    current_tech_info: Option<player_core::AudioTechnicalInfo>,
    is_playing: bool,
    position_ms: u64,
    duration_ms: u64,
    volume: f32,
    is_seeking: bool,
    seek_pos_ms: u64,
    current_lyrics: Option<String>,
    is_shuffle: bool,
    is_repeat: bool,
    loved_tracks: HashSet<i64>,

    // Library Scanning & Fonts
    is_scanning: bool,
    available_fonts: Vec<String>,
    primary_font: Option<String>,
    custom_font_input: String,

    // Scrobbling Plugins
    listenbrainz_plugin: Arc<ListenBrainzPlugin>,
    lastfm_plugin: Arc<LastFmPlugin>,
    scrobble_config: ScrobbleConfig,
    scrobble_test_status: Option<(ScrobbleService, Result<String, String>)>,

    mpris_rx: Option<Receiver<PlayerAction>>,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        info!("Initializing Vibecore Native Player application");

        let db_path = get_database_path();
        info!("Using SQLite database at: {:?}", db_path);

        let db = LibraryDatabase::new(&db_path).unwrap_or_else(|e| {
            error!(
                "Failed to open disk database, falling back to in-memory: {}",
                e
            );
            LibraryDatabase::in_memory().expect("In-memory SQLite failed")
        });

        let config = AppConfig::load();

        let audio_engine = AudioEngine::new();
        let audio_controller = audio_engine.controller();
        audio_controller.set_volume(config.volume);

        let plugin_manager = PluginManager::new();

        let listenbrainz_plugin = Arc::new(ListenBrainzPlugin::new(
            config.scrobble.to_listenbrainz_config(),
        ));
        let lastfm_plugin = Arc::new(LastFmPlugin::new(
            config.scrobble.to_lastfm_config(),
        ));
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

        let page_size = config.page_size;
        let total_tracks = db.get_total_track_count(None).unwrap_or(0);
        let visible_tracks = db
            .get_tracks_window(
                0,
                page_size,
                None,
                SortField::Title,
                SortDirection::Ascending,
            )
            .unwrap_or_default();
        let mut albums = db.get_albums().unwrap_or_default();
        Self::sort_albums(
            &mut albums,
            config.album_sort_field,
            SortDirection::Ascending,
        );
        let artists = db.get_artists().unwrap_or_default();
        let library_folders = db.get_library_folders().unwrap_or_default();

        let available_fonts = crate::font::list_font_families();
        let primary_font = config.primary_font;
        let config_path = AppConfig::config_path();

        let app = Self {
            db,
            db_path,
            config_path,
            _audio_engine: audio_engine,
            audio_controller,
            plugin_manager,
            theme_manager,

            active_tab: config.active_tab,
            settings_tab: config.settings_tab,

            library_view_mode: config.library_view_mode,
            albums_view_mode: config.albums_view_mode,
            artists_view_mode: config.artists_view_mode,
            selected_track_id: None,
            hovered_track_id: None,
            last_track_click: None,

            visible_tracks,
            total_tracks,
            page: 0,
            page_size,
            is_loading_more: false,
            search_query: String::new(),
            sort_field: SortField::Title,
            sort_dir: SortDirection::Ascending,

            albums,
            album_covers: HashMap::new(),
            album_sort_field: config.album_sort_field,
            album_sort_dir: SortDirection::Ascending,
            visible_albums_count: DEFAULT_CATALOG_CHUNK_SIZE,
            artists,
            artist_sort_field: config.artist_sort_field,
            artist_sort_dir: SortDirection::Ascending,
            visible_artists_count: DEFAULT_CATALOG_CHUNK_SIZE,
            library_folders,

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
            is_seeking: false,
            seek_pos_ms: 0,
            current_lyrics: None,
            is_shuffle: config.is_shuffle,
            is_repeat: config.is_repeat,
            loved_tracks: HashSet::new(),

            is_scanning: false,
            available_fonts,
            primary_font,
            custom_font_input: String::new(),

            listenbrainz_plugin,
            lastfm_plugin,
            scrobble_config: config.scrobble,
            scrobble_test_status: None,

            mpris_rx,
        };

        let initial_task = if app.active_tab == ViewTab::Albums {
            app.load_visible_album_covers_task()
        } else {
            Task::none()
        };

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
        iced::time::every(Duration::from_millis(50)).map(|_| Message::AudioTick)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AudioTick => {
                let status = self.audio_controller.status();
                let was_playing = self.is_playing;
                self.is_playing = status.is_playing();

                if was_playing != self.is_playing {
                    self.plugin_manager.on_playback_status_change(self.is_playing);
                }

                if !self.is_seeking {
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
                if let Ok(Some(track)) = self.db.get_track_by_id(track_id) {
                    info!("Playing track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    let cover_bytes = player_core::get_or_create_thumbnail(&path, 500)
                        .or_else(|| player_core::extract_cover_art(&path));

                    if let Some(ref bytes) = cover_bytes {
                        self.current_blurred_cover = player_core::generate_blurred_cover(bytes)
                            .map(image::Handle::from_bytes);
                    } else {
                        self.current_blurred_cover = None;
                    }

                    self.current_cover = cover_bytes.map(image::Handle::from_bytes);
                    self.current_tech_info =
                        player_core::extract_technical_info(&path, track.duration_ms);

                    self.audio_controller.load(path, true);
                    self.current_track = Some(track.clone());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = track.duration_ms;

                    // Pre-extract cover for next track for the Now Playing view
                    let next_track_opt = self.get_next_track();
                    self.current_next_cover = next_track_opt.and_then(|nt| {
                        let nt_path = PathBuf::from(&nt.file_path);
                        player_core::get_or_create_thumbnail(&nt_path, 200)
                            .or_else(|| player_core::extract_cover_art(&nt_path))
                            .map(image::Handle::from_bytes)
                    });

                    let lyrics = self
                        .plugin_manager
                        .fetch_lyrics(&track.artist, &track.title);
                    self.current_lyrics = lyrics;

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

                if self.is_shuffle && !self.visible_tracks.is_empty() {
                    let rand_idx =
                        (self.position_ms as usize ^ 0x9e3779b9) % self.visible_tracks.len();
                    let next_id = self.visible_tracks[rand_idx].id;
                    return self.update(Message::PlayTrack(next_id));
                }

                if let Some(cur_id) = self.current_track_id {
                    // Try to play next track from currently loaded window
                    if let Some(pos) = self.visible_tracks.iter().position(|t| t.id == cur_id) {
                        if pos + 1 < self.visible_tracks.len() {
                            let next_id = self.visible_tracks[pos + 1].id;
                            return self.update(Message::PlayTrack(next_id));
                        }
                    }
                }
                // Fallback: play first track if available
                if let Some(first) = self.visible_tracks.first() {
                    return self.update(Message::PlayTrack(first.id));
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
                    if let Some(pos) = self.visible_tracks.iter().position(|t| t.id == cur_id) {
                        if pos > 0 {
                            let prev_id = self.visible_tracks[pos - 1].id;
                            return self.update(Message::PlayTrack(prev_id));
                        }
                    }
                }
                Task::none()
            }

            Message::Seek(ms) => {
                self.is_seeking = true;
                self.seek_pos_ms = ms as u64;
                Task::none()
            }

            Message::SeekFinished => {
                self.is_seeking = false;
                self.audio_controller
                    .seek(Duration::from_millis(self.seek_pos_ms));
                self.position_ms = self.seek_pos_ms;
                self.plugin_manager.on_seeked(self.position_ms);
                Task::none()
            }

            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.audio_controller.set_volume(vol);
                self.plugin_manager.on_volume_change(vol);
                self.save_config_task()
            }

            Message::MprisAction(action) => {
                match action {
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
                                    if let Ok(Some(track)) = self.db.get_track_by_path(path.to_str().unwrap_or_default()) {
                                        return self.update(Message::PlayTrack(track.id));
                                    }
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
                }
            }

            Message::SelectTab(tab) => {
                self.active_tab = tab;
                let save_task = self.save_config_task();
                match tab {
                    ViewTab::Albums => {
                        self.visible_albums_count = DEFAULT_CATALOG_CHUNK_SIZE;
                        let load_task = if self.albums.is_empty() {
                            let db = self.db.clone();
                            Task::perform(
                                async move { db.get_albums().unwrap_or_default() },
                                Message::AlbumsLoaded,
                            )
                        } else {
                            self.load_visible_album_covers_task()
                        };
                        Task::batch([save_task, load_task])
                    }
                    ViewTab::Artists => {
                        self.visible_artists_count = DEFAULT_CATALOG_CHUNK_SIZE;
                        if self.artists.is_empty() {
                            let db = self.db.clone();
                            let load_task = Task::perform(
                                async move { db.get_artists().unwrap_or_default() },
                                Message::ArtistsLoaded,
                            );
                            Task::batch([save_task, load_task])
                        } else {
                            save_task
                        }
                    }
                    _ => save_task,
                }
            }

            Message::SearchInputChanged(query) => {
                self.search_query = query;
                self.page = 0;
                self.reload_tracks_task()
            }

            Message::SortChanged(field) => {
                if self.sort_field == field {
                    self.sort_dir = match self.sort_dir {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                } else {
                    self.sort_field = field;
                    self.sort_dir = SortDirection::Ascending;
                }
                self.page = 0;
                self.reload_tracks_task()
            }

            Message::NextPage => {
                let max_pages = (self.total_tracks + self.page_size - 1) / self.page_size;
                if self.page + 1 < max_pages {
                    self.page += 1;
                    return self.reload_tracks_task();
                }
                Task::none()
            }

            Message::PrevPage => {
                if self.page > 0 {
                    self.page -= 1;
                    return self.reload_tracks_task();
                }
                Task::none()
            }

            Message::GoToPage(p) => {
                self.page = p;
                self.reload_tracks_task()
            }

            Message::SetPageSize(size) => {
                self.page_size = size;
                self.page = 0;
                Task::batch([self.save_config_task(), self.reload_tracks_task()])
            }

            Message::SetLibraryViewMode(mode) => {
                self.library_view_mode = mode;
                self.save_config_task()
            }

            Message::SetAlbumsViewMode(mode) => {
                self.albums_view_mode = mode;
                self.save_config_task()
            }

            Message::SetArtistsViewMode(mode) => {
                self.artists_view_mode = mode;
                self.save_config_task()
            }

            Message::TrackHovered(maybe_id) => {
                self.hovered_track_id = maybe_id;
                Task::none()
            }

            Message::TrackClicked(id) => {
                let now = std::time::Instant::now();
                let is_double_click = if let Some((last_id, last_time)) = self.last_track_click {
                    last_id == id && now.duration_since(last_time).as_millis() <= 400
                } else {
                    false
                };

                if is_double_click {
                    self.last_track_click = None;
                    self.selected_track_id = Some(id);
                    self.update(Message::PlayTrack(id))
                } else {
                    self.last_track_click = Some((id, now));
                    self.selected_track_id = Some(id);
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
                    let _ = self.db.add_library_folder(&path_str);
                    self.library_folders = self.db.get_library_folders().unwrap_or_default();
                    let db = self.db.clone();
                    Task::perform(
                        async move {
                            let summary = db
                                .scan_directory(&path, None::<fn(usize, usize)>)
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
                    let _ = self.db.add_library_folder(&path_str);
                    self.library_folders = self.db.get_library_folders().unwrap_or_default();
                    let db = self.db.clone();
                    Task::perform(
                        async move {
                            let summary = db
                                .scan_directory(&path, None::<fn(usize, usize)>)
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
                let _ = self.db.remove_library_folder(&path_str);
                self.library_folders = self.db.get_library_folders().unwrap_or_default();
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
                            .unwrap_or_default();
                        summary
                    },
                    Message::ScanFinished,
                )
            }

            Message::ReindexLibrary => {
                self.is_scanning = true;
                info!("Reindexing all library folders: {:?}", self.library_folders);
                let db = self.db.clone();
                let folders = self.library_folders.clone();
                Task::perform(
                    async move {
                        let mut total_summary = player_core::ScanSummary::default();
                        for folder_str in folders {
                            let path = PathBuf::from(folder_str);
                            if let Ok(summary) = db.scan_directory(&path, None::<fn(usize, usize)>)
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
                let _ = self.db.clear_all_tracks();
                self.visible_tracks.clear();
                self.total_tracks = 0;
                self.albums.clear();
                self.album_covers.clear();
                self.artists.clear();
                self.current_track = None;
                self.current_track_id = None;
                self.current_cover = None;
                self.is_playing = false;
                self.audio_controller.stop();
                Task::none()
            }

            Message::ClearThumbnailCache => {
                info!("Purging thumbnail disk and GPU cache");
                let _ = player_core::clear_thumbnail_cache();
                self.album_covers.clear();
                Task::none()
            }

            Message::ScanFinished(summary) => {
                info!("Scan completed: {} tracks indexed", summary.inserted_tracks);
                self.is_scanning = false;
                self.library_folders = self.db.get_library_folders().unwrap_or_default();
                self.reload_after_scan_task()
            }

            Message::SelectSettingsTab(tab) => {
                self.settings_tab = tab;
                self.save_config_task()
            }

            Message::SelectAlbum(album_name) => {
                self.active_tab = ViewTab::Tracks;
                self.search_query = album_name;
                self.page = 0;
                self.reload_tracks_task()
            }

            Message::SelectArtist(artist_name) => {
                self.active_tab = ViewTab::Tracks;
                self.search_query = artist_name;
                self.page = 0;
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
                self.primary_font = font_opt.clone();
                self.theme_manager.set_font(font_opt);
                self.save_config_task()
            }

            Message::CustomFontInputChanged(input) => {
                self.custom_font_input = input;
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
                            ScrobbleService::ListenBrainz => {
                                (service, plugin_lb.test_connection())
                            }
                            ScrobbleService::LastFm => {
                                (service, plugin_lfm.test_connection())
                            }
                        })
                        .await
                        .unwrap_or_else(|e| (service, Err(format!("Task execution error: {}", e))))
                    },
                    |(service, res)| Message::ScrobbleTestFinished(service, res),
                )
            }

            Message::ScrobbleTestFinished(service, res) => {
                self.scrobble_test_status = Some((service, res));
                Task::none()
            }

            Message::TracksLoaded(tracks, total) => {
                self.visible_tracks = tracks;
                self.total_tracks = total;
                self.is_loading_more = false;
                Task::none()
            }

            Message::LibraryScrolled(viewport) => {
                let rel_offset = viewport.relative_offset();
                if rel_offset.y >= 0.8
                    && !self.is_loading_more
                    && self.visible_tracks.len() < self.total_tracks
                {
                    self.is_loading_more = true;
                    return self.load_more_tracks_task();
                }
                Task::none()
            }

            Message::MoreTracksLoaded(tracks) => {
                self.is_loading_more = false;
                if !tracks.is_empty() {
                    self.visible_tracks.extend(tracks);
                }
                Task::none()
            }

            Message::AlbumSortChanged(field) => {
                if self.album_sort_field == field {
                    self.album_sort_dir = match self.album_sort_dir {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                } else {
                    self.album_sort_field = field;
                    self.album_sort_dir = match field {
                        AlbumSortField::Year
                        | AlbumSortField::TrackCount
                        | AlbumSortField::Duration
                        | AlbumSortField::DateAdded => SortDirection::Descending,
                        _ => SortDirection::Ascending,
                    };
                }
                Self::sort_albums(&mut self.albums, self.album_sort_field, self.album_sort_dir);
                self.visible_albums_count = DEFAULT_CATALOG_CHUNK_SIZE;
                let save_task = self.save_config_task();
                let covers_task = self.load_visible_album_covers_task();
                Task::batch([save_task, covers_task])
            }

            Message::ArtistSortChanged(field) => {
                if self.artist_sort_field == field {
                    self.artist_sort_dir = match self.artist_sort_dir {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                } else {
                    self.artist_sort_field = field;
                    self.artist_sort_dir = match field {
                        ArtistSortField::TrackCount | ArtistSortField::AlbumCount => {
                            SortDirection::Descending
                        }
                        _ => SortDirection::Ascending,
                    };
                }
                Self::sort_artists(&mut self.artists, self.artist_sort_field, self.artist_sort_dir);
                self.visible_artists_count = DEFAULT_CATALOG_CHUNK_SIZE;
                self.save_config_task()
            }

            Message::AlbumsLoaded(mut albums) => {
                Self::sort_albums(&mut albums, self.album_sort_field, self.album_sort_dir);
                self.albums = albums;
                self.visible_albums_count = DEFAULT_CATALOG_CHUNK_SIZE;
                if self.active_tab == ViewTab::Albums {
                    self.load_visible_album_covers_task()
                } else {
                    Task::none()
                }
            }

            Message::AlbumsScrolled(viewport) => {
                let rel_offset = viewport.relative_offset();
                if rel_offset.y >= 0.75 && self.visible_albums_count < self.albums.len() {
                    self.visible_albums_count = (self.visible_albums_count
                        + DEFAULT_CATALOG_CHUNK_SIZE)
                        .min(self.albums.len());
                    return self.load_visible_album_covers_task();
                }
                Task::none()
            }

            Message::ArtistsScrolled(viewport) => {
                let rel_offset = viewport.relative_offset();
                if rel_offset.y >= 0.75 && self.visible_artists_count < self.artists.len() {
                    self.visible_artists_count = (self.visible_artists_count
                        + DEFAULT_CATALOG_CHUNK_SIZE)
                        .min(self.artists.len());
                }
                Task::none()
            }

            Message::AlbumCoversLoaded(loaded) => {
                for (key, bytes) in loaded {
                    let handle = image::Handle::from_bytes(bytes);
                    self.album_covers.insert(key, handle);
                }
                Task::none()
            }

            Message::AlbumCoverChunkLoaded(chunk_results, remaining) => {
                for (key, bytes) in chunk_results {
                    let handle = image::Handle::from_bytes(bytes);
                    self.album_covers.insert(key, handle);
                }

                if !remaining.is_empty() {
                    self.load_cover_chunk_task(remaining)
                } else {
                    Task::none()
                }
            }

            Message::ArtistsLoaded(mut artists) => {
                Self::sort_artists(&mut artists, self.artist_sort_field, self.artist_sort_dir);
                self.artists = artists;
                self.visible_artists_count = DEFAULT_CATALOG_CHUNK_SIZE;
                Task::none()
            }

            Message::ConfigSaved => Task::none(),
        }
    }

    pub fn get_next_track(&self) -> Option<Track> {
        if self.is_repeat {
            return self.current_track.clone();
        }

        if self.is_shuffle && !self.visible_tracks.is_empty() {
            let rand_idx =
                (self.position_ms as usize ^ 0x9e3779b9) % self.visible_tracks.len();
            return Some(self.visible_tracks[rand_idx].clone());
        }

        if let Some(cur_id) = self.current_track_id {
            // Try to find next track from currently loaded window
            if let Some(pos) = self.visible_tracks.iter().position(|t| t.id == cur_id) {
                if pos + 1 < self.visible_tracks.len() {
                    return Some(self.visible_tracks[pos + 1].clone());
                }
            }
            // If at end of window or not in visible tracks, query database
            if let Ok(Some(next)) = self.db.get_next_track(cur_id) {
                if Some(next.id) != self.current_track_id {
                    return Some(next);
                }
            }
        }

        // Fallback: first track if available
        if let Some(first) = self.visible_tracks.first() {
            if Some(first.id) != self.current_track_id {
                return Some(first.clone());
            }
        }
        None
    }

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            active_tab: self.active_tab,
            settings_tab: self.settings_tab,
            theme_name: self.theme_manager.active_theme_name.clone(),
            primary_font: self.primary_font.clone(),
            library_view_mode: self.library_view_mode,
            albums_view_mode: self.albums_view_mode,
            artists_view_mode: self.artists_view_mode,
            page_size: self.page_size,
            album_sort_field: self.album_sort_field,
            artist_sort_field: self.artist_sort_field,
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
                        if let Some(bytes) = player_core::get_or_create_thumbnail(&path, 200) {
                            results.push((key, bytes));
                        }
                    }
                    (results, remaining)
                })
                .await
                .unwrap_or_else(|_| (Vec::new(), Vec::new()))
            },
            |(results, remaining)| Message::AlbumCoverChunkLoaded(results, remaining),
        )
    }

    fn load_visible_album_covers_task(&self) -> Task<Message> {
        let visible_slice = &self.albums[..self.visible_albums_count.min(self.albums.len())];
        let to_load: Vec<(String, PathBuf)> = visible_slice
            .iter()
            .filter_map(|alb| {
                let key = format!("{}:{}", alb.artist, alb.album);
                if self.album_covers.contains_key(&key) {
                    None
                } else {
                    alb.cover_track_path
                        .as_ref()
                        .map(|p| (key, PathBuf::from(p)))
                }
            })
            .collect();

        if to_load.is_empty() {
            Task::none()
        } else {
            self.load_cover_chunk_task(to_load)
        }
    }

    fn sort_albums(albums: &mut [AlbumSummary], field: AlbumSortField, dir: SortDirection) {
        albums.sort_by(|a, b| {
            let ord = match field {
                AlbumSortField::Title => a.album.to_lowercase().cmp(&b.album.to_lowercase()),
                AlbumSortField::Artist => a
                    .artist
                    .to_lowercase()
                    .cmp(&b.artist.to_lowercase())
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::Year => a
                    .year
                    .cmp(&b.year)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::Duration => a
                    .total_duration_ms
                    .cmp(&b.total_duration_ms)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::DateAdded => a
                    .added_timestamp
                    .cmp(&b.added_timestamp)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    fn sort_artists(artists: &mut [ArtistSummary], field: ArtistSortField, dir: SortDirection) {
        artists.sort_by(|a, b| {
            let ord = match field {
                ArtistSortField::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
                ArtistSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| a.artist.to_lowercase().cmp(&b.artist.to_lowercase())),
                ArtistSortField::AlbumCount => a
                    .album_count
                    .cmp(&b.album_count)
                    .then_with(|| a.artist.to_lowercase().cmp(&b.artist.to_lowercase())),
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    fn reload_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let page_size = self.page_size;
        let query = if self.search_query.trim().is_empty() {
            None
        } else {
            Some(self.search_query.clone())
        };
        let sort_field = self.sort_field;
        let sort_dir = self.sort_dir;

        Task::perform(
            async move {
                let total = db.get_total_track_count(query.as_deref()).unwrap_or(0);
                let tracks = db
                    .get_tracks_window(0, page_size, query.as_deref(), sort_field, sort_dir)
                    .unwrap_or_default();
                (tracks, total)
            },
            |(tracks, total)| Message::TracksLoaded(tracks, total),
        )
    }

    fn reload_after_scan_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let page_size = self.page_size;
        let query = if self.search_query.trim().is_empty() {
            None
        } else {
            Some(self.search_query.clone())
        };
        let sort_field = self.sort_field;
        let sort_dir = self.sort_dir;

        Task::batch([
            Task::perform(
                async move {
                    let total = db.get_total_track_count(query.as_deref()).unwrap_or(0);
                    let tracks = db
                        .get_tracks_window(0, page_size, query.as_deref(), sort_field, sort_dir)
                        .unwrap_or_default();
                    (tracks, total)
                },
                |(tracks, total)| Message::TracksLoaded(tracks, total),
            ),
            {
                let db = self.db.clone();
                Task::perform(
                    async move { db.get_albums().unwrap_or_default() },
                    Message::AlbumsLoaded,
                )
            },
            {
                let db = self.db.clone();
                Task::perform(
                    async move { db.get_artists().unwrap_or_default() },
                    Message::ArtistsLoaded,
                )
            },
        ])
    }

    fn load_more_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let offset = self.visible_tracks.len();
        let limit = self.page_size;
        let query = if self.search_query.trim().is_empty() {
            None
        } else {
            Some(self.search_query.clone())
        };
        let sort_field = self.sort_field;
        let sort_dir = self.sort_dir;

        Task::perform(
            async move {
                db.get_tracks_window(offset, limit, query.as_deref(), sort_field, sort_dir)
                    .unwrap_or_default()
            },
            Message::MoreTracksLoaded,
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let theme_cfg = &self.theme_manager.current_config;

        // Top Navigation & Tab Bar
        let tab_btn = |label: &'static str, tab: ViewTab| {
            let is_selected = self.active_tab == tab;
            button(text(theme_cfg, label).size(11).color(if is_selected {
                theme_cfg.primary_color()
            } else {
                theme_cfg.text_muted_color()
            }))
            .padding([5, 12])
            .style(styles::header_tab_button(theme_cfg, is_selected))
            .on_press(Message::SelectTab(tab))
        };

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

        let top_bar = container(column![
            row![
                tab_btn("Library", ViewTab::Tracks),
                tab_btn("Now Playing", ViewTab::Lyrics),
                tab_btn("Albums", ViewTab::Albums),
                tab_btn("Artists", ViewTab::Artists),
                Space::with_width(Length::Fill),
                settings_btn,
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            .padding([4, 12])
            .height(32),
            border_vert(1.0, theme_cfg.border_color()),
        ])
        .style(styles::top_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(33);

        let main_view: Element<'_, Message> = match self.active_tab {
            ViewTab::Tracks => views::library::view(
                theme_cfg,
                &self.visible_tracks,
                self.current_track_id,
                self.selected_track_id,
                self.hovered_track_id,
                self.is_playing,
                self.library_view_mode,
                &self.search_query,
                self.sort_field,
                self.sort_dir,
                self.page,
                self.page_size,
            ),
            ViewTab::Albums => {
                let visible_slice =
                    &self.albums[..self.visible_albums_count.min(self.albums.len())];
                views::albums::view(
                    theme_cfg,
                    visible_slice,
                    &self.album_covers,
                    self.albums_view_mode,
                    self.album_sort_field,
                    self.album_sort_dir,
                )
            }
            ViewTab::Artists => {
                let visible_slice =
                    &self.artists[..self.visible_artists_count.min(self.artists.len())];
                views::artists::view(
                    theme_cfg,
                    visible_slice,
                    self.artists_view_mode,
                    self.artist_sort_field,
                    self.artist_sort_dir,
                )
            }
            ViewTab::Lyrics => {
                let next_track = self.get_next_track();
                let is_loved = self
                    .current_track_id
                    .map(|id| self.loved_tracks.contains(&id))
                    .unwrap_or(false);

                views::lyrics::view(
                    theme_cfg,
                    self.current_track.as_ref(),
                    self.current_cover.as_ref(),
                    self.current_blurred_cover.as_ref(),
                    self.current_lyrics.as_deref(),
                    next_track,
                    self.current_next_cover.as_ref(),
                    self.is_repeat,
                    self.is_shuffle,
                    is_loved,
                    self.current_tech_info.as_ref(),
                )
            }
            ViewTab::Settings => {
                let cache_stats = player_core::get_thumbnail_cache_stats();
                let db_size_bytes = std::fs::metadata(&self.db_path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                views::settings::view(
                    theme_cfg,
                    views::settings::SettingsViewData {
                        active_tab: self.settings_tab,
                        theme_names: self.theme_manager.theme_names(),
                        active_theme_name: &self.theme_manager.active_theme_name,
                        library_view_mode: self.library_view_mode,
                        albums_view_mode: self.albums_view_mode,
                        artists_view_mode: self.artists_view_mode,
                        page_size: self.page_size,
                        primary_font: self.primary_font.as_deref(),
                        available_fonts: self.available_fonts.as_slice(),
                        custom_font_input: self.custom_font_input.clone(),
                        library_folders: &self.library_folders,
                        total_tracks: self.total_tracks,
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
                        scrobble_test_status: self.scrobble_test_status.as_ref(),

                        volume: self.volume,
                        active_plugins_count: self.plugin_manager.plugin_count(),
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

        let player_bar = views::player_bar::view(
            theme_cfg,
            self.current_track.as_ref(),
            self.current_cover.as_ref(),
            self.is_playing,
            self.position_ms,
            self.duration_ms,
            self.volume,
            self.is_seeking,
            self.seek_pos_ms,
            self.is_shuffle,
            self.is_repeat,
            is_loved,
        );

        let status_bar_ctx = views::status_bar::StatusBarContext {
            active_tab: self.active_tab,
            page: self.page,
            page_size: self.page_size,
            total_tracks: self.total_tracks,
            visible_count: self.visible_tracks.len(),
            search_query: &self.search_query,
            albums: &self.albums,
            artists: &self.artists,
            current_track: self.current_track.as_ref(),
            is_playing: self.is_playing,
            is_scanning: self.is_scanning,
        };

        let status_bar = views::status_bar::view(theme_cfg, status_bar_ctx);

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
    if let Some(proj_dirs) = ProjectDirs::from("moe", "curstantine", "vibecore") {
        let data_dir = proj_dirs.data_dir();
        let _ = std::fs::create_dir_all(data_dir);
        data_dir.join("library.db")
    } else {
        PathBuf::from("library.db")
    }
}
