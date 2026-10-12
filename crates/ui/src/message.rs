use std::path::PathBuf;

use core_engine::{AlbumSummary, ArtistSummary, ScanSummary, SortField, Track, TrackRow};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewTab {
    #[default]
    Tracks,
    Albums,
    Artists,
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
            ViewTab::Settings => "Settings",
        }
    }

    pub fn icon_svg(&self) -> &'static str {
        match self {
            ViewTab::Tracks => crate::icons::MUSIC_NOTE_SVG,
            ViewTab::Lyrics => crate::icons::SPEAKER_PLAYING_SVG,
            ViewTab::Albums => crate::icons::ALBUM_SVG,
            ViewTab::Artists => crate::icons::USER_AVATAR_SVG,
            ViewTab::Settings => crate::icons::SETTINGS_SVG,
        }
    }
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
#[allow(dead_code)]
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
    MoveTab(ViewTab, bool),
    ResetTabOrder,
    SetAlbumsViewMode(ViewMode),
    SetArtistsViewMode(ViewMode),
    SetTileWidth(u32),
    SetAlbumTileWidth(u32),
    SetArtistTileWidth(u32),
    SetNowPlayingCoverSize(u32),

    // Track interactions
    TrackClicked(i64),

    // Library & Pagination
    SearchInputChanged(String),
    SortChanged(SortField),
    LibraryScrolled(iced::widget::scrollable::Viewport),
    TrackWindowLoaded {
        offset: usize,
        rows: Vec<TrackRow>,
    },
    SetPageSize(usize),

    // Scanning & File Picking
    ScanFolderClicked,
    FolderSelected(Option<PathBuf>),
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
}
