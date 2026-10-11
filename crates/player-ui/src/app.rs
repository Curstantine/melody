use std::path::PathBuf;
use std::time::Duration;

use directories::ProjectDirs;
use iced::widget::{column, container, image, row};
use iced::{Element, Length, Subscription, Task, Theme};
use player_audio::{AudioController, AudioEngine};
use player_core::{
    AlbumSummary, ArtistSummary, LibraryDatabase, SortDirection, SortField, Track,
};
use player_plugins::{PluginManager, TrackMetadata};
use tracing::{error, info};

use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeManager};
use crate::views;

pub const PAGE_SIZE: usize = 50;

pub struct App {
    db: LibraryDatabase,
    _audio_engine: AudioEngine,
    audio_controller: AudioController,
    plugin_manager: PluginManager,
    theme_manager: ThemeManager,

    // Navigation & Tabs
    active_tab: ViewTab,

    // Virtualized Library View State
    visible_tracks: Vec<Track>,
    total_tracks: usize,
    page: usize,
    search_query: String,
    sort_field: SortField,
    sort_dir: SortDirection,

    // Catalogs
    albums: Vec<AlbumSummary>,
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

        let total_tracks = db.get_total_track_count(None).unwrap_or(0);
        let visible_tracks = db
            .get_tracks_window(0, PAGE_SIZE, None, SortField::Title, SortDirection::Ascending)
            .unwrap_or_default();

        let app = Self {
            db,
            _audio_engine: audio_engine,
            audio_controller,
            plugin_manager,
            theme_manager,

            active_tab: ViewTab::Tracks,

            visible_tracks,
            total_tracks,
            page: 0,
            search_query: String::new(),
            sort_field: SortField::Title,
            sort_dir: SortDirection::Ascending,

            albums: Vec::new(),
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

                    let cover_handle = player_core::extract_cover_art(&path)
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
                let max_pages = (self.total_tracks + PAGE_SIZE - 1) / PAGE_SIZE;
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

            Message::AlbumsLoaded(albums) => {
                self.albums = albums;
                Task::none()
            }

            Message::ArtistsLoaded(artists) => {
                self.artists = artists;
                Task::none()
            }
        }
    }

    fn reload_tracks_task(&self) -> Task<Message> {
        let db = self.db.clone();
        let page = self.page;
        let query = if self.search_query.trim().is_empty() {
            None
        } else {
            Some(self.search_query.clone())
        };
        let sort_field = self.sort_field;
        let sort_dir = self.sort_dir;

        Task::perform(
            async move {
                let offset = page * PAGE_SIZE;
                let total = db.get_total_track_count(query.as_deref()).unwrap_or(0);
                let tracks = db
                    .get_tracks_window(offset, PAGE_SIZE, query.as_deref(), sort_field, sort_dir)
                    .unwrap_or_default();
                (tracks, total)
            },
            |(tracks, total)| Message::TracksLoaded(tracks, total),
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let theme_cfg = &self.theme_manager.current_config;

        let sidebar = views::sidebar::view(
            self.active_tab,
            theme_cfg,
            self.theme_manager.theme_names(),
            self.theme_manager.active_theme_name.clone(),
            self.total_tracks,
            self.is_scanning,
        );

        let main_view: Element<'_, Message> = match self.active_tab {
            ViewTab::Tracks => views::library::view(
                theme_cfg,
                &self.visible_tracks,
                self.current_track_id,
                &self.search_query,
                self.sort_field,
                self.sort_dir,
                self.page,
                PAGE_SIZE,
                self.total_tracks,
            ),
            ViewTab::Albums => views::albums::view(theme_cfg, &self.albums),
            ViewTab::Artists => views::artists::view(theme_cfg, &self.artists),
            ViewTab::Lyrics => views::lyrics::view(
                theme_cfg,
                self.current_track.as_ref(),
                self.current_cover.as_ref(),
                self.current_lyrics.as_deref(),
            ),
        };

        let body = row![sidebar, main_view]
            .width(Length::Fill)
            .height(Length::Fill);

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
        );

        let full_layout = column![body, player_bar]
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
    if let Some(proj_dirs) = ProjectDirs::from("org", "vibecore", "player") {
        let data_dir = proj_dirs.data_dir();
        let _ = std::fs::create_dir_all(data_dir);
        data_dir.join("library.db")
    } else {
        PathBuf::from("library.db")
    }
}
