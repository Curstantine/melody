use std::path::PathBuf;

use core_engine::{
    AlbumSummary, ArtistSummary, AudioTechnicalInfo, CacheStats, PlaylistEntry, PlaylistSummary,
    ScanSummary, SortField, Track, TrackFilter, TrackRow,
};

use serde::{Deserialize, Serialize};

/// A single row in an opened playlist's detail list.
#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub title: String,
    pub artist: String,
    pub duration_label: String,
    /// `None` when the entry could not be matched to an indexed library track.
    pub track_id: Option<i64>,
}

impl PlaylistItem {
    pub fn is_available(&self) -> bool {
        self.track_id.is_some()
    }
}

/// Outcome of resolving an M3U file against the indexed library.
#[derive(Debug, Clone)]
pub struct PlaylistResolution {
    /// One entry per line of the playlist, in file order (including missing).
    pub items: Vec<PlaylistItem>,
    /// Raw file entries 1:1 with `items`, kept so reordered playlists can be
    /// written back without losing unmatched lines or relative paths.
    pub entries: Vec<PlaylistEntry>,
    /// Indexed tracks that were successfully matched, in playlist order.
    pub tracks: Vec<TrackRow>,
}

/// What a resolved playlist should be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistOpenIntent {
    View,
    Play { start_index: usize },
    AddToQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewTab {
    #[default]
    Tracks,
    Albums,
    Artists,
    Playlists,
    Lyrics,
    Settings,
}

impl ViewTab {
    pub fn label(&self) -> &'static str {
        match self {
            ViewTab::Tracks => "Library",
            ViewTab::Lyrics => "Now Playing",
            ViewTab::Albums => "Albums",
            ViewTab::Artists => "Artists",
            ViewTab::Playlists => "Playlists",
            ViewTab::Settings => "Settings",
        }
    }

    pub fn icon_svg(&self) -> &'static str {
        match self {
            ViewTab::Tracks => crate::icons::MUSIC_NOTE_SVG,
            ViewTab::Lyrics => crate::icons::SPEAKER_PLAYING_SVG,
            ViewTab::Albums => crate::icons::ALBUM_SVG,
            ViewTab::Artists => crate::icons::USER_AVATAR_SVG,
            ViewTab::Playlists => crate::icons::PLAYLIST_SVG,
            ViewTab::Settings => crate::icons::SETTINGS_SVG,
        }
    }
}

/// Sub-section selector within the Playlists tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlaylistSection {
    #[default]
    Playlists,
    Queues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PlaylistSortField {
    #[default]
    Name,
    TrackCount,
    DateAdded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewMode {
    #[default]
    List,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlbumSortField {
    #[default]
    Title,
    Artist,
    Year,
    TrackCount,
    Duration,
    DateAdded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArtistSortField {
    #[default]
    Artist,
    TrackCount,
    AlbumCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SettingsTab {
    #[default]
    Appearance,
    Library,
    Scrobbling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsMessage {
    CustomFontInputChanged(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    // Playback Controls
    PlayTrack(i64),
    TogglePlayPause,
    Stop,
    NextTrack,
    PreviousTrack,
    Seek(f32),
    SeekFinished,
    VolumeChanged(f32),
    AudioTick,
    ToggleShuffle,
    ToggleRepeat,
    MprisAction(plugins::PlayerAction),

    // Navigation & Views
    SelectTab(ViewTab),
    TabDragStarted(ViewTab),
    TabDraggedOver(ViewTab),
    TabDragReleased,
    TabHovered(Option<ViewTab>),
    SetAlbumsViewMode(ViewMode),
    SetArtistsViewMode(ViewMode),
    SetPlaylistsViewMode(ViewMode),
    SetAlbumTileWidth(u32),
    SetArtistTileWidth(u32),
    SetNowPlayingCoverSize(u32),

    // Track interactions
    TrackClicked(i64),

    // Library & Pagination
    SearchInputChanged(String),
    SearchDebounced(u64),
    SortChanged(SortField),
    LibraryScrolled(iced::widget::scrollable::Viewport),
    TrackWindowLoaded {
        offset: usize,
        rows: Vec<TrackRow>,
    },
    SetPageSize(usize),

    // Scanning & File Picking
    ScanFinished(ScanSummary),

    // Album / Artist Selection & Sorting
    SelectAlbum(String),
    SelectArtist(String),
    AlbumSortChanged(AlbumSortField),
    ArtistSortChanged(ArtistSortField),
    AlbumsScrolled(iced::widget::scrollable::Viewport),
    ArtistsScrolled(iced::widget::scrollable::Viewport),
    EnsureAlbumCovers,
    AlbumCoverChunkLoaded {
        request_id: u64,
        results: Vec<(String, Vec<u8>)>,
        remaining: Vec<(String, i64)>,
    },

    // Playlists
    PlaylistSectionChanged(PlaylistSection),
    PlaylistsLoaded(Vec<PlaylistSummary>),
    ImportPlaylistsClicked,
    PlaylistsImported(Option<Vec<PathBuf>>),
    NewPlaylistClicked,
    NewPlaylistNameChanged(String),
    NewPlaylistNameSubmitted(String),
    NewPlaylistCancelled,
    SelectPlaylist(i64),
    PlaylistResolved {
        playlist_id: i64,
        intent: PlaylistOpenIntent,
        resolution: PlaylistResolution,
    },
    PlayPlaylist(i64),
    PlayPlaylistItem(i64, usize),
    PlaylistToQueue(i64),
    PlaylistDetailClicked {
        playlist_id: i64,
        index: usize,
    },
    PlaylistDetailDragStarted {
        playlist_id: i64,
        index: usize,
    },
    PlaylistDetailDraggedOver {
        playlist_id: i64,
        index: usize,
    },
    PlaylistDetailDragReleased,
    PlaylistDetailHovered(Option<(i64, usize)>),
    PlaylistDetailPersisted {
        playlist_id: i64,
        success: bool,
    },
    RenamePlaylist(i64),
    RenamePlaylistNameChanged(String),
    RenamePlaylistSubmitted(i64, String),
    RenamePlaylistCancelled,
    DeletePlaylist(i64),
    PlaylistSortChanged(PlaylistSortField),

    // Queues
    QueueNew,
    QueueSelect(u64),
    QueueDelete(u64),
    QueueActivated {
        id: u64,
        tracks: Vec<TrackRow>,
    },
    LibraryQueueBuilt {
        track_id: i64,
        name: String,
        /// Serialized [`crate::queue::QueueSource`], kept as a string so this
        /// module stays decoupled from the queue module.
        source_kind: String,
        tracks: Vec<TrackRow>,
    },
    PlayScope {
        filter: TrackFilter,
        name: String,
        shuffle: bool,
    },
    QueueRemoveTrack {
        queue_id: u64,
        index: usize,
    },
    QueueClicked {
        queue_id: u64,
        index: usize,
    },
    QueueRowHovered(Option<(u64, usize)>),
    QueueDragStarted {
        queue_id: u64,
        index: usize,
    },
    QueueDraggedOver {
        queue_id: u64,
        index: usize,
    },
    QueueDragReleased,
    QueueSaveClicked,
    QueueSaveNameChanged(String),
    QueueSaveSubmitted(String),
    QueueSaveCancelled,
    QueuesPersisted,
    QueueScrolled(iced::widget::scrollable::Viewport),

    // Themes & Settings
    Settings(SettingsMessage),
    SelectSettingsTab(SettingsTab),
    SelectTheme(String),
    ReloadThemes,
    SelectFont(String),
    SaveConfig,
    DebouncedConfigSave(u64),
    ConfigSaved,

    // Scrobbling
    ToggleListenBrainz(bool),
    ListenBrainzTokenChanged(String),
    ListenBrainzUrlChanged(String),
    ToggleLastFm(bool),
    LastFmApiKeyChanged(String),
    LastFmApiSecretChanged(String),
    LastFmSessionKeyChanged(String),
    LastFmUrlChanged(String),
    ToggleSubmitNowPlaying(bool),
    SetScrobblePercent(u32),
    TestScrobble(plugins::scrobbler::ScrobbleService),
    ScrobbleTestFinished(plugins::scrobbler::ScrobbleService, Result<String, String>),

    // Library Maintenance
    AddLibraryFolderClicked,
    LibraryFolderAdded(Option<PathBuf>),
    RemoveLibraryFolder(String),
    RescanLibraryFolder(PathBuf),
    ReindexLibrary,
    ClearLibrary,
    ClearThumbnailCache,

    // Loaded data callbacks
    TracksLoaded(Vec<TrackRow>, usize),
    AlbumsLoaded(Vec<AlbumSummary>),
    ArtistsLoaded(Vec<ArtistSummary>),
    LibraryFoldersLoaded(Vec<String>),
    TrackResolvedForPlayback(Box<Option<Track>>),
    PlaybackAssetsLoaded {
        track_id: i64,
        cover: Option<Vec<u8>>,
        blurred: Option<Vec<u8>>,
        next_cover: Option<Vec<u8>>,
        tech_info: Option<AudioTechnicalInfo>,
        lyrics: Option<String>,
    },
    StorageStatsLoaded(CacheStats, u64),
}
