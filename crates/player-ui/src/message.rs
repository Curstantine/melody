use std::path::PathBuf;

use player_core::{AlbumSummary, ArtistSummary, ScanSummary, SortField, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewTab {
    #[default]
    Tracks,
    Albums,
    Artists,
    Lyrics,
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

    // Navigation & Views
    SelectTab(ViewTab),

    // Library & Pagination
    SearchInputChanged(String),
    SortChanged(SortField),
    NextPage,
    PrevPage,
    GoToPage(usize),

    // Scanning & File Picking
    ScanFolderClicked,
    FolderSelected(Option<PathBuf>),
    ScanFinished(ScanSummary),

    // Album / Artist Selection
    SelectAlbum(String),
    SelectArtist(String),

    // Themes
    SelectTheme(String),
    ReloadThemes,

    // Loaded data callbacks
    TracksLoaded(Vec<Track>, usize),
    AlbumsLoaded(Vec<AlbumSummary>),
    ArtistsLoaded(Vec<ArtistSummary>),
}
