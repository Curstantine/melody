use std::path::PathBuf;

use player_core::{AlbumSummary, ArtistSummary, ScanSummary, SortField, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewTab {
    #[default]
    Tracks,
    Albums,
    Artists,
    Lyrics,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    List,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlbumSortField {
    #[default]
    Title,
    Artist,
    Year,
    TrackCount,
    Duration,
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

    // Navigation & Views
    SelectTab(ViewTab),
    SetLibraryViewMode(ViewMode),
    SetAlbumsViewMode(ViewMode),
    SetArtistsViewMode(ViewMode),

    // Track interactions
    TrackClicked(i64),
    TrackHovered(Option<i64>),

    // Library & Pagination
    SearchInputChanged(String),
    SortChanged(SortField),
    NextPage,
    PrevPage,
    GoToPage(usize),
    SetPageSize(usize),

    // Scanning & File Picking
    ScanFolderClicked,
    FolderSelected(Option<PathBuf>),
    ScanFinished(ScanSummary),

    // Album / Artist Selection & Sorting
    SelectAlbum(String),
    SelectArtist(String),
    AlbumSortChanged(AlbumSortField),
    AlbumCoversLoaded(Vec<(String, Vec<u8>)>),
    AlbumCoverChunkLoaded(Vec<(String, Vec<u8>)>, Vec<(String, PathBuf)>),

    // Themes & Settings
    SelectTheme(String),
    ReloadThemes,

    // Loaded data callbacks
    TracksLoaded(Vec<Track>, usize),
    AlbumsLoaded(Vec<AlbumSummary>),
    ArtistsLoaded(Vec<ArtistSummary>),
}
