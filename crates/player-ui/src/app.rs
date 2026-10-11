use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use directories::ProjectDirs;
use iced::widget::{button, column, container, image, row, text, Space};
use iced::{Element, Length, Subscription, Task, Theme};
use player_audio::{AudioController, AudioEngine};
use player_core::{
    AlbumSummary, ArtistSummary, LibraryDatabase, SortDirection, SortField, Track,
};
use player_plugins::{PluginManager, TrackMetadata};
use tracing::{error, info};

use crate::icons;
use crate::message::{AlbumSortField, Message, ViewMode, ViewTab};
use crate::theme::{styles, ThemeManager};
use crate::views;

pub const DEFAULT_PAGE_SIZE: usize = 50;

pub struct App {
    db: LibraryDatabase,
    _audio_engine: AudioEngine,
    audio_controller: AudioController,
    plugin_manager: PluginManager,
    theme_manager: ThemeManager,

    // Navigation & Tabs
    active_tab: ViewTab,

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
    search_query: String,
    sort_field: SortField,
    sort_dir: SortDirection,

    // Catalogs
    albums: Vec<AlbumSummary>,
    album_covers: HashMap<String, image::Handle>,
    album_sort_field: AlbumSortField,
    album_sort_dir: SortDirection,
    artists: Vec<ArtistSummary>,

    // Playback State
    current_track: Option<Track>,
    current_track_id: Option<i64>,
    current_cover: Option<image::Handle>,
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

    // Library Scanning
    is_scanning: bool,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        info!("Initializing Vibecore Native Player application");

        let db_path = get_database_path();
        info!("Using SQLite database at: {:?}", db_path);

        let db = LibraryDatabase::new(db_path).unwrap_or_else(|e| {
            error!("Failed to open disk database, falling back to in-memory: {}", e);
            LibraryDatabase::in_memory().expect("In-memory SQLite failed")
        });

        let audio_engine = AudioEngine::new();
        let audio_controller = audio_engine.controller();
        let plugin_manager = PluginManager::new();
        let theme_manager = ThemeManager::new();

        let page_size = DEFAULT_PAGE_SIZE;
        let total_tracks = db.get_total_track_count(None).unwrap_or(0);
        let visible_tracks = db
            .get_tracks_window(0, page_size, None, SortField::Title, SortDirection::Ascending)
            .unwrap_or_default();

        let app = Self {
            db,
            _audio_engine: audio_engine,
            audio_controller,
            plugin_manager,
            theme_manager,

            active_tab: ViewTab::Tracks,

            library_view_mode: ViewMode::List,
            albums_view_mode: ViewMode::Tile,
            artists_view_mode: ViewMode::Tile,
            selected_track_id: None,
            hovered_track_id: None,
            last_track_click: None,

            visible_tracks,
            total_tracks,
            page: 0,
            page_size,
            search_query: String::new(),
            sort_field: SortField::Title,
            sort_dir: SortDirection::Ascending,

            albums: Vec::new(),
            album_covers: HashMap::new(),
            album_sort_field: AlbumSortField::Title,
            album_sort_dir: SortDirection::Ascending,
            artists: Vec::new(),

            current_track: None,
            current_track_id: None,
            current_cover: None,
            is_playing: false,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
            is_seeking: false,
            seek_pos_ms: 0,
            current_lyrics: None,
            is_shuffle: false,
            is_repeat: false,
            loved_tracks: HashSet::new(),

            is_scanning: false,
        };

        (app, Task::none())
    }

    pub fn title(&self) -> String {
        match &self.current_track {
            Some(track) => format!("{} - {} | Vibecore", track.title, track.artist),
            None => "Vibecore Native Music Player".to_string(),
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
                self.is_playing = status.is_playing();

                if !self.is_seeking {
                    self.position_ms = status.position_ms();
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

                Task::none()
            }

            Message::PlayTrack(track_id) => {
                if let Ok(Some(track)) = self.db.get_track_by_id(track_id) {
                    info!("Playing track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    let cover_handle = player_core::get_or_create_thumbnail(&path, 500)
                        .or_else(|| player_core::extract_cover_art(&path))
                        .map(image::Handle::from_bytes);
                    self.current_cover = cover_handle;

                    self.audio_controller.load(path, true);
                    self.current_track = Some(track.clone());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = track.duration_ms;

                    let lyrics = self.plugin_manager.fetch_lyrics(&track.artist, &track.title);
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
                    let rand_idx = (self.position_ms as usize ^ 0x9e3779b9) % self.visible_tracks.len();
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
                Task::none()
            }

            Message::ToggleRepeat => {
                self.is_repeat = !self.is_repeat;
                info!("Repeat toggled: {}", self.is_repeat);
                Task::none()
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
                self.audio_controller.seek(Duration::from_millis(self.seek_pos_ms));
                self.position_ms = self.seek_pos_ms;
                Task::none()
            }

            Message::VolumeChanged(vol) => {
                self.volume = vol;
                self.audio_controller.set_volume(vol);
                Task::none()
            }

            Message::SelectTab(tab) => {
                self.active_tab = tab;
                match tab {
                    ViewTab::Albums => {
                        let db = self.db.clone();
                        Task::perform(
                            async move { db.get_albums().unwrap_or_default() },
                            Message::AlbumsLoaded,
                        )
                    }
                    ViewTab::Artists => {
                        let db = self.db.clone();
                        Task::perform(
                            async move { db.get_artists().unwrap_or_default() },
                            Message::ArtistsLoaded,
                        )
                    }
                    _ => Task::none(),
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
                self.reload_tracks_task()
            }

            Message::SetLibraryViewMode(mode) => {
                self.library_view_mode = mode;
                Task::none()
            }

            Message::SetAlbumsViewMode(mode) => {
                self.albums_view_mode = mode;
                Task::none()
            }

            Message::SetArtistsViewMode(mode) => {
                self.artists_view_mode = mode;
                Task::none()
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

            Message::FolderSelected(maybe_path) => {
                match maybe_path {
                    Some(path) => {
                        info!("Scanning folder: {:?}", path);
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
                }
            }

            Message::ScanFinished(summary) => {
                info!(
                    "Scan completed: {} tracks indexed",
                    summary.inserted_tracks
                );
                self.is_scanning = false;
                self.reload_tracks_task()
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
                Task::none()
            }

            Message::ReloadThemes => {
                self.theme_manager.load_all_themes();
                Task::none()
            }

            Message::TracksLoaded(tracks, total) => {
                self.visible_tracks = tracks;
                self.total_tracks = total;
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
                        | AlbumSortField::Duration => SortDirection::Descending,
                        _ => SortDirection::Ascending,
                    };
                }
                Self::sort_albums(&mut self.albums, self.album_sort_field, self.album_sort_dir);
                Task::none()
            }

            Message::AlbumsLoaded(mut albums) => {
                Self::sort_albums(&mut albums, self.album_sort_field, self.album_sort_dir);
                self.albums = albums;

                let to_load: Vec<(String, PathBuf)> = self
                    .albums
                    .iter()
                    .filter_map(|alb| {
                        let key = format!("{}:{}", alb.artist, alb.album);
                        if self.album_covers.contains_key(&key) {
                            None
                        } else {
                            alb.cover_track_path.as_ref().map(|p| (key, PathBuf::from(p)))
                        }
                    })
                    .collect();

                if to_load.is_empty() {
                    Task::none()
                } else {
                    self.load_cover_chunk_task(to_load)
                }
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

            Message::ArtistsLoaded(artists) => {
                self.artists = artists;
                Task::none()
            }
        }
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
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    fn reload_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let page = self.page;
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
                let offset = page * page_size;
                let total = db.get_total_track_count(query.as_deref()).unwrap_or(0);
                let tracks = db
                    .get_tracks_window(offset, page_size, query.as_deref(), sort_field, sort_dir)
                    .unwrap_or_default();
                (tracks, total)
            },
            |(tracks, total)| Message::TracksLoaded(tracks, total),
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let theme_cfg = &self.theme_manager.current_config;

        // Top Navigation & Tab Bar
        let tab_btn = |label: &'static str, tab: ViewTab| {
            let is_selected = self.active_tab == tab;
            button(
                text(label)
                    .size(11)
                    .color(if is_selected {
                        theme_cfg.primary_color()
                    } else {
                        theme_cfg.text_muted_color()
                    }),
            )
            .padding([5, 12])
            .style(styles::header_tab_button(theme_cfg, is_selected))
            .on_press(Message::SelectTab(tab))
        };

        let top_tabs = row![
            tab_btn("MUSIC", ViewTab::Tracks),
            tab_btn("NOW PLAYING", ViewTab::Lyrics),
            tab_btn("ALBUMS", ViewTab::Albums),
            tab_btn("ARTISTS", ViewTab::Artists),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

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

        let top_bar = container(
            row![
                top_tabs,
                Space::with_width(Length::Fill),
                settings_btn,
            ]
            .align_y(iced::Alignment::Center)
            .padding([4, 12]),
        )
        .style(styles::top_bar_container(theme_cfg))
        .width(Length::Fill)
        .height(34);

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
            ViewTab::Albums => views::albums::view(
                theme_cfg,
                &self.albums,
                &self.album_covers,
                self.albums_view_mode,
                self.album_sort_field,
                self.album_sort_dir,
            ),
            ViewTab::Artists => views::artists::view(theme_cfg, &self.artists, self.artists_view_mode),
            ViewTab::Lyrics => views::lyrics::view(
                theme_cfg,
                self.current_track.as_ref(),
                self.current_cover.as_ref(),
                self.current_lyrics.as_deref(),
            ),
            ViewTab::Settings => views::settings::view(
                theme_cfg,
                self.theme_manager.theme_names(),
                &self.theme_manager.active_theme_name,
                self.total_tracks,
                self.albums.len(),
                self.artists.len(),
                self.is_scanning,
                self.page_size,
                self.library_view_mode,
                self.volume,
            ),
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
    if let Some(proj_dirs) = ProjectDirs::from("org", "vibecore", "vibecore") {
        let data_dir = proj_dirs.data_dir();
        let _ = std::fs::create_dir_all(data_dir);
        data_dir.join("library.db")
    } else {
        PathBuf::from("library.db")
    }
}
