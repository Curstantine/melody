use std::path::PathBuf;

use core_engine::{AlbumSummary, ArtistSummary, ScanSummary, SortField, Track};

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
    ToggleLoveCurrentTrack,
    MprisAction(plugins::PlayerAction),

    // Navigation & Views
    SelectTab(ViewTab),
    SetLibraryViewMode(ViewMode),
    SetAlbumsViewMode(ViewMode),
    SetArtistsViewMode(ViewMode),

    // Track interactions
    TrackClicked(i64),

    // Library & Pagination
    SearchInputChanged(String),
    SortChanged(SortField),
    LibraryScrolled(iced::widget::scrollable::Viewport),
    MoreTracksLoaded(Vec<Track>),
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
    AlbumCoverChunkLoaded(Vec<(String, Vec<u8>)>, Vec<(String, PathBuf)>),

    // Themes & Settings
    Settings(SettingsMessage),
    SelectSettingsTab(SettingsTab),
    SelectTheme(String),
    ReloadThemes,
    SelectFont(String),
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
    TracksLoaded(Vec<Track>, usize),
    AlbumsLoaded(Vec<AlbumSummary>),
    ArtistsLoaded(Vec<ArtistSummary>),
    LibraryFoldersLoaded(Vec<String>),
    TrackResolvedForPlayback(Option<Track>),
}
