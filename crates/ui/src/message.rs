use std::path::PathBuf;
use std::sync::Arc;

use core_engine::{
    AlbumSummary, ArtistSummary, AudioTechnicalInfo, CacheStats, PlaylistEntry, PlaylistSummary,
    ScanSummary, SortField, Track, TrackFilter, TrackRow,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackMenuContext {
    Library,
    Playlist { playlist_id: i64, index: usize },
    Queue { queue_id: u64, index: usize },
    PlayerBar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuSubmenu {
    Playlists,
}

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
    PlayNext,
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
            ViewTab::Lyrics => "Now playing",
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
    Playback,
    Library,
    Scrobbling,
}

#[derive(Clone)]
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
    ToggleGapless(bool),
    SetResamplingFilter(audio::ResamplingFilter),
    SetResamplingQuality(audio::ResamplingQuality),
    ToggleResamplingLinearInterp(bool),
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

    // Context Menus
    OpenTrackContextMenu {
        track_id: i64,
        context: TrackMenuContext,
    },
    TrackContextMenuReady {
        track: TrackRow,
        context: TrackMenuContext,
        position: iced::Point,
    },
    OpenAlbumContextMenu(usize),
    OpenArtistContextMenu(usize),
    OpenPlaylistContextMenu(PlaylistSummary),
    OpenQueueContextMenu {
        id: u64,
        name: String,
    },
    OpenPlayerBarContextMenu,
    CloseContextMenu,
    ToggleContextMenuSubmenu(Option<ContextMenuSubmenu>),
    ContextPlayTrack(i64),
    ContextPlayNext(TrackRow),
    ContextAddToQueue(TrackRow),
    ContextAddTracksToQueue(Vec<TrackRow>),
    ContextPlayTracksNext(Vec<TrackRow>),
    ContextPlayAlbumNext(AlbumSummary),
    ContextAddAlbumToQueue(AlbumSummary),
    ContextPlayArtistNext(ArtistSummary),
    ContextAddArtistToQueue(ArtistSummary),
    ContextAddTrackToPlaylist {
        track: TrackRow,
        playlist_id: i64,
    },
    ContextAddAlbumToPlaylist {
        album: AlbumSummary,
        playlist_id: i64,
    },
    ContextAddArtistToPlaylist {
        artist: ArtistSummary,
        playlist_id: i64,
    },
    ContextNewPlaylistWithTracks(Vec<TrackRow>),
    ContextNewPlaylistWithAlbum(AlbumSummary),
    ContextNewPlaylistWithArtist(ArtistSummary),
    WindowResized(iced::Size),
    CancelDuplicatePrompt,
    ConfirmDuplicatePrompt,
    EscapePressed,

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
    ScanProgress {
        current: usize,
        total: usize,
    },
    ScanFinished(ScanSummary),
    ThumbnailsGenerated,

    // Album / Artist Selection & Sorting
    SelectAlbum(Arc<str>),
    SelectArtist(Arc<str>),
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
    PlaylistCoversLoaded(Vec<(i64, Vec<Vec<u8>>)>),
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
    PlaylistPlayNext(i64),
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
    PlaylistRemoveTrack {
        playlist_id: i64,
        index: usize,
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
        name: Arc<str>,
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
    SelectSettingsTab(SettingsTab),
    SelectTheme(String),
    ReloadThemes,
    SelectLayout(String),
    ReloadLayouts,
    SelectFont(String),
    FontsLoaded(Vec<String>),
    SaveConfig,
    DebouncedConfigSave(u64),
    ConfigSaved,

    // Runtime state persistence
    SaveState,
    DebouncedStateSave(u64),
    StateSaved,

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
    ScanLibraryForNewFiles,
    ReindexLibrary,
    ToggleWatchLibrary(bool),
    ClearLibrary,
    ClearThumbnailCache,

    // Loaded data callbacks
    TracksLoaded(Vec<TrackRow>, usize),
    AlbumsLoaded(Vec<AlbumSummary>),
    ArtistsLoaded(Vec<ArtistSummary>),
    LibraryFoldersLoaded(Vec<String>),
    TrackResolvedForPlayback(Box<Option<Track>>),
    TrackResolvedForRestore(Box<Option<Track>>),
    TrackDetailsLoaded(Box<Option<Track>>),
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

// Manual `Debug` so `iced_debug` never formats multi-KB payloads (cover JPEG
// bytes, track/album vectors, lyrics). The derived impl printed every byte of
// every `Vec<u8>`, costing ~2-4ms per message and tripping iced's
// "Slow `Debug` implementation" warning on hot paths like
// `AlbumCoverChunkLoaded`. Summarize with lengths instead.
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn bytes_len(b: &Option<Vec<u8>>) -> Option<usize> {
            b.as_ref().map(Vec::len)
        }
        match self {
            Self::PlayTrack(id) => f.debug_tuple("PlayTrack").field(id).finish(),
            Self::TogglePlayPause => write!(f, "TogglePlayPause"),
            Self::Stop => write!(f, "Stop"),
            Self::NextTrack => write!(f, "NextTrack"),
            Self::PreviousTrack => write!(f, "PreviousTrack"),
            Self::Seek(v) => f.debug_tuple("Seek").field(v).finish(),
            Self::SeekFinished => write!(f, "SeekFinished"),
            Self::VolumeChanged(v) => f.debug_tuple("VolumeChanged").field(v).finish(),
            Self::AudioTick => write!(f, "AudioTick"),
            Self::ToggleShuffle => write!(f, "ToggleShuffle"),
            Self::ToggleRepeat => write!(f, "ToggleRepeat"),
            Self::ToggleGapless(v) => f.debug_tuple("ToggleGapless").field(v).finish(),
            Self::SetResamplingFilter(filter) => {
                f.debug_tuple("SetResamplingFilter").field(filter).finish()
            }
            Self::SetResamplingQuality(quality) => f
                .debug_tuple("SetResamplingQuality")
                .field(quality)
                .finish(),
            Self::ToggleResamplingLinearInterp(v) => f
                .debug_tuple("ToggleResamplingLinearInterp")
                .field(v)
                .finish(),
            Self::MprisAction(a) => f.debug_tuple("MprisAction").field(a).finish(),
            Self::SelectTab(t) => f.debug_tuple("SelectTab").field(t).finish(),
            Self::TabDragStarted(t) => f.debug_tuple("TabDragStarted").field(t).finish(),
            Self::TabDraggedOver(t) => f.debug_tuple("TabDraggedOver").field(t).finish(),
            Self::TabDragReleased => write!(f, "TabDragReleased"),
            Self::TabHovered(t) => f.debug_tuple("TabHovered").field(t).finish(),
            Self::SetAlbumsViewMode(m) => f.debug_tuple("SetAlbumsViewMode").field(m).finish(),
            Self::SetArtistsViewMode(m) => f.debug_tuple("SetArtistsViewMode").field(m).finish(),
            Self::SetPlaylistsViewMode(m) => {
                f.debug_tuple("SetPlaylistsViewMode").field(m).finish()
            }
            Self::SetAlbumTileWidth(w) => f.debug_tuple("SetAlbumTileWidth").field(w).finish(),
            Self::SetArtistTileWidth(w) => f.debug_tuple("SetArtistTileWidth").field(w).finish(),
            Self::SetNowPlayingCoverSize(s) => {
                f.debug_tuple("SetNowPlayingCoverSize").field(s).finish()
            }
            Self::TrackClicked(id) => f.debug_tuple("TrackClicked").field(id).finish(),
            Self::OpenTrackContextMenu { track_id, context } => f
                .debug_struct("OpenTrackContextMenu")
                .field("track_id", track_id)
                .field("context", context)
                .finish(),
            Self::TrackContextMenuReady { track, context, .. } => f
                .debug_struct("TrackContextMenuReady")
                .field("track_id", &track.id)
                .field("context", context)
                .finish(),
            Self::OpenAlbumContextMenu(idx) => f
                .debug_struct("OpenAlbumContextMenu")
                .field("index", idx)
                .finish(),
            Self::OpenArtistContextMenu(idx) => f
                .debug_struct("OpenArtistContextMenu")
                .field("index", idx)
                .finish(),
            Self::OpenPlaylistContextMenu(p) => f
                .debug_struct("OpenPlaylistContextMenu")
                .field("id", &p.id)
                .field("name", &p.name)
                .finish(),
            Self::OpenQueueContextMenu { id, name } => f
                .debug_struct("OpenQueueContextMenu")
                .field("id", id)
                .field("name", name)
                .finish(),
            Self::OpenPlayerBarContextMenu => write!(f, "OpenPlayerBarContextMenu"),
            Self::CloseContextMenu => write!(f, "CloseContextMenu"),
            Self::ToggleContextMenuSubmenu(sub) => f
                .debug_tuple("ToggleContextMenuSubmenu")
                .field(sub)
                .finish(),
            Self::ContextPlayTrack(id) => f.debug_tuple("ContextPlayTrack").field(id).finish(),
            Self::ContextPlayNext(track) => {
                f.debug_tuple("ContextPlayNext").field(&track.id).finish()
            }
            Self::ContextAddToQueue(track) => {
                f.debug_tuple("ContextAddToQueue").field(&track.id).finish()
            }
            Self::ContextAddTracksToQueue(tracks) => f
                .debug_struct("ContextAddTracksToQueue")
                .field("len", &tracks.len())
                .finish(),
            Self::ContextPlayTracksNext(tracks) => f
                .debug_struct("ContextPlayTracksNext")
                .field("len", &tracks.len())
                .finish(),
            Self::ContextPlayAlbumNext(a) => f
                .debug_struct("ContextPlayAlbumNext")
                .field("album", &a.album)
                .finish(),
            Self::ContextAddAlbumToQueue(a) => f
                .debug_struct("ContextAddAlbumToQueue")
                .field("album", &a.album)
                .finish(),
            Self::ContextPlayArtistNext(a) => f
                .debug_struct("ContextPlayArtistNext")
                .field("artist", &a.artist)
                .finish(),
            Self::ContextAddArtistToQueue(a) => f
                .debug_struct("ContextAddArtistToQueue")
                .field("artist", &a.artist)
                .finish(),
            Self::ContextAddTrackToPlaylist { track, playlist_id } => f
                .debug_struct("ContextAddTrackToPlaylist")
                .field("track_id", &track.id)
                .field("playlist_id", playlist_id)
                .finish(),
            Self::ContextAddAlbumToPlaylist { album, playlist_id } => f
                .debug_struct("ContextAddAlbumToPlaylist")
                .field("album", &album.album)
                .field("playlist_id", playlist_id)
                .finish(),
            Self::ContextAddArtistToPlaylist {
                artist,
                playlist_id,
            } => f
                .debug_struct("ContextAddArtistToPlaylist")
                .field("artist", &artist.artist)
                .field("playlist_id", playlist_id)
                .finish(),
            Self::ContextNewPlaylistWithTracks(tracks) => f
                .debug_struct("ContextNewPlaylistWithTracks")
                .field("len", &tracks.len())
                .finish(),
            Self::ContextNewPlaylistWithAlbum(a) => f
                .debug_struct("ContextNewPlaylistWithAlbum")
                .field("album", &a.album)
                .finish(),
            Self::ContextNewPlaylistWithArtist(a) => f
                .debug_struct("ContextNewPlaylistWithArtist")
                .field("artist", &a.artist)
                .finish(),
            Self::WindowResized(size) => f.debug_tuple("WindowResized").field(size).finish(),
            Self::CancelDuplicatePrompt => write!(f, "CancelDuplicatePrompt"),
            Self::ConfirmDuplicatePrompt => write!(f, "ConfirmDuplicatePrompt"),
            Self::EscapePressed => write!(f, "EscapePressed"),
            Self::SearchInputChanged(s) => f
                .debug_struct("SearchInputChanged")
                .field("len", &s.len())
                .finish(),
            Self::SearchDebounced(seq) => f.debug_tuple("SearchDebounced").field(seq).finish(),
            Self::SortChanged(s) => f.debug_tuple("SortChanged").field(s).finish(),
            Self::LibraryScrolled(v) => f.debug_tuple("LibraryScrolled").field(v).finish(),
            Self::TrackWindowLoaded { offset, rows } => f
                .debug_struct("TrackWindowLoaded")
                .field("offset", offset)
                .field("rows_len", &rows.len())
                .finish(),
            Self::SetPageSize(n) => f.debug_tuple("SetPageSize").field(n).finish(),
            Self::ScanProgress { current, total } => f
                .debug_struct("ScanProgress")
                .field("current", current)
                .field("total", total)
                .finish(),
            Self::ScanFinished(s) => f.debug_tuple("ScanFinished").field(s).finish(),
            Self::ThumbnailsGenerated => write!(f, "ThumbnailsGenerated"),
            Self::SelectAlbum(s) => f.debug_tuple("SelectAlbum").field(s).finish(),
            Self::SelectArtist(s) => f.debug_tuple("SelectArtist").field(s).finish(),
            Self::AlbumSortChanged(s) => f.debug_tuple("AlbumSortChanged").field(s).finish(),
            Self::ArtistSortChanged(s) => f.debug_tuple("ArtistSortChanged").field(s).finish(),
            Self::AlbumsScrolled(v) => f.debug_tuple("AlbumsScrolled").field(v).finish(),
            Self::ArtistsScrolled(v) => f.debug_tuple("ArtistsScrolled").field(v).finish(),
            Self::EnsureAlbumCovers => write!(f, "EnsureAlbumCovers"),
            Self::AlbumCoverChunkLoaded {
                request_id,
                results,
                remaining,
            } => {
                let sizes: Vec<(&str, usize)> =
                    results.iter().map(|(k, b)| (k.as_str(), b.len())).collect();
                f.debug_struct("AlbumCoverChunkLoaded")
                    .field("request_id", request_id)
                    .field("results", &sizes)
                    .field("remaining_len", &remaining.len())
                    .finish()
            }
            Self::PlaylistSectionChanged(s) => {
                f.debug_tuple("PlaylistSectionChanged").field(s).finish()
            }
            Self::PlaylistsLoaded(v) => f
                .debug_struct("PlaylistsLoaded")
                .field("len", &v.len())
                .finish(),
            Self::PlaylistCoversLoaded(v) => f
                .debug_struct("PlaylistCoversLoaded")
                .field("playlists_len", &v.len())
                .finish(),
            Self::ImportPlaylistsClicked => write!(f, "ImportPlaylistsClicked"),
            Self::PlaylistsImported(p) => f.debug_tuple("PlaylistsImported").field(p).finish(),
            Self::NewPlaylistClicked => write!(f, "NewPlaylistClicked"),
            Self::NewPlaylistNameChanged(s) => {
                f.debug_tuple("NewPlaylistNameChanged").field(s).finish()
            }
            Self::NewPlaylistNameSubmitted(s) => {
                f.debug_tuple("NewPlaylistNameSubmitted").field(s).finish()
            }
            Self::NewPlaylistCancelled => write!(f, "NewPlaylistCancelled"),
            Self::SelectPlaylist(id) => f.debug_tuple("SelectPlaylist").field(id).finish(),
            Self::PlaylistResolved {
                playlist_id,
                intent,
                resolution,
            } => f
                .debug_struct("PlaylistResolved")
                .field("playlist_id", playlist_id)
                .field("intent", intent)
                .field("items_len", &resolution.items.len())
                .field("entries_len", &resolution.entries.len())
                .field("tracks_len", &resolution.tracks.len())
                .finish(),
            Self::PlayPlaylist(id) => f.debug_tuple("PlayPlaylist").field(id).finish(),
            Self::PlayPlaylistItem(id, i) => f
                .debug_tuple("PlayPlaylistItem")
                .field(id)
                .field(i)
                .finish(),
            Self::PlaylistToQueue(id) => f.debug_tuple("PlaylistToQueue").field(id).finish(),
            Self::PlaylistPlayNext(id) => f.debug_tuple("PlaylistPlayNext").field(id).finish(),
            Self::PlaylistDetailClicked { playlist_id, index } => f
                .debug_struct("PlaylistDetailClicked")
                .field("playlist_id", playlist_id)
                .field("index", index)
                .finish(),
            Self::PlaylistDetailDragStarted { playlist_id, index } => f
                .debug_struct("PlaylistDetailDragStarted")
                .field("playlist_id", playlist_id)
                .field("index", index)
                .finish(),
            Self::PlaylistDetailDraggedOver { playlist_id, index } => f
                .debug_struct("PlaylistDetailDraggedOver")
                .field("playlist_id", playlist_id)
                .field("index", index)
                .finish(),
            Self::PlaylistDetailDragReleased => write!(f, "PlaylistDetailDragReleased"),
            Self::PlaylistDetailHovered(h) => {
                f.debug_tuple("PlaylistDetailHovered").field(h).finish()
            }
            Self::PlaylistDetailPersisted {
                playlist_id,
                success,
            } => f
                .debug_struct("PlaylistDetailPersisted")
                .field("playlist_id", playlist_id)
                .field("success", success)
                .finish(),
            Self::PlaylistRemoveTrack { playlist_id, index } => f
                .debug_struct("PlaylistRemoveTrack")
                .field("playlist_id", playlist_id)
                .field("index", index)
                .finish(),
            Self::RenamePlaylist(id) => f.debug_tuple("RenamePlaylist").field(id).finish(),
            Self::RenamePlaylistNameChanged(s) => {
                f.debug_tuple("RenamePlaylistNameChanged").field(s).finish()
            }
            Self::RenamePlaylistSubmitted(id, s) => f
                .debug_tuple("RenamePlaylistSubmitted")
                .field(id)
                .field(s)
                .finish(),
            Self::RenamePlaylistCancelled => write!(f, "RenamePlaylistCancelled"),
            Self::DeletePlaylist(id) => f.debug_tuple("DeletePlaylist").field(id).finish(),
            Self::PlaylistSortChanged(s) => f.debug_tuple("PlaylistSortChanged").field(s).finish(),
            Self::QueueNew => write!(f, "QueueNew"),
            Self::QueueSelect(id) => f.debug_tuple("QueueSelect").field(id).finish(),
            Self::QueueDelete(id) => f.debug_tuple("QueueDelete").field(id).finish(),
            Self::QueueActivated { id, tracks } => f
                .debug_struct("QueueActivated")
                .field("id", id)
                .field("tracks_len", &tracks.len())
                .finish(),
            Self::LibraryQueueBuilt {
                track_id,
                name,
                source_kind,
                tracks,
            } => f
                .debug_struct("LibraryQueueBuilt")
                .field("track_id", track_id)
                .field("name", name)
                .field("source_kind", source_kind)
                .field("tracks_len", &tracks.len())
                .finish(),
            Self::PlayScope {
                filter,
                name,
                shuffle,
            } => f
                .debug_struct("PlayScope")
                .field("filter", filter)
                .field("name", name)
                .field("shuffle", shuffle)
                .finish(),
            Self::QueueRemoveTrack { queue_id, index } => f
                .debug_struct("QueueRemoveTrack")
                .field("queue_id", queue_id)
                .field("index", index)
                .finish(),
            Self::QueueClicked { queue_id, index } => f
                .debug_struct("QueueClicked")
                .field("queue_id", queue_id)
                .field("index", index)
                .finish(),
            Self::QueueRowHovered(h) => f.debug_tuple("QueueRowHovered").field(h).finish(),
            Self::QueueDragStarted { queue_id, index } => f
                .debug_struct("QueueDragStarted")
                .field("queue_id", queue_id)
                .field("index", index)
                .finish(),
            Self::QueueDraggedOver { queue_id, index } => f
                .debug_struct("QueueDraggedOver")
                .field("queue_id", queue_id)
                .field("index", index)
                .finish(),
            Self::QueueDragReleased => write!(f, "QueueDragReleased"),
            Self::QueueSaveClicked => write!(f, "QueueSaveClicked"),
            Self::QueueSaveNameChanged(s) => {
                f.debug_tuple("QueueSaveNameChanged").field(s).finish()
            }
            Self::QueueSaveSubmitted(s) => f.debug_tuple("QueueSaveSubmitted").field(s).finish(),
            Self::QueueSaveCancelled => write!(f, "QueueSaveCancelled"),
            Self::QueuesPersisted => write!(f, "QueuesPersisted"),
            Self::QueueScrolled(v) => f.debug_tuple("QueueScrolled").field(v).finish(),
            Self::SelectSettingsTab(t) => f.debug_tuple("SelectSettingsTab").field(t).finish(),
            Self::SelectTheme(s) => f.debug_tuple("SelectTheme").field(s).finish(),
            Self::ReloadThemes => write!(f, "ReloadThemes"),
            Self::SelectLayout(s) => f.debug_tuple("SelectLayout").field(s).finish(),
            Self::ReloadLayouts => write!(f, "ReloadLayouts"),
            Self::SelectFont(s) => f.debug_tuple("SelectFont").field(s).finish(),
            Self::FontsLoaded(fonts) => f
                .debug_struct("FontsLoaded")
                .field("count", &fonts.len())
                .finish(),
            Self::SaveConfig => write!(f, "SaveConfig"),
            Self::DebouncedConfigSave(seq) => {
                f.debug_tuple("DebouncedConfigSave").field(seq).finish()
            }
            Self::ConfigSaved => write!(f, "ConfigSaved"),
            Self::SaveState => write!(f, "SaveState"),
            Self::DebouncedStateSave(seq) => {
                f.debug_tuple("DebouncedStateSave").field(seq).finish()
            }
            Self::StateSaved => write!(f, "StateSaved"),
            Self::ToggleListenBrainz(b) => f.debug_tuple("ToggleListenBrainz").field(b).finish(),
            Self::ListenBrainzTokenChanged(s) => f
                .debug_struct("ListenBrainzTokenChanged")
                .field("len", &s.len())
                .finish(),
            Self::ListenBrainzUrlChanged(s) => {
                f.debug_tuple("ListenBrainzUrlChanged").field(s).finish()
            }
            Self::ToggleLastFm(b) => f.debug_tuple("ToggleLastFm").field(b).finish(),
            Self::LastFmApiKeyChanged(s) => f
                .debug_struct("LastFmApiKeyChanged")
                .field("len", &s.len())
                .finish(),
            Self::LastFmApiSecretChanged(s) => f
                .debug_struct("LastFmApiSecretChanged")
                .field("len", &s.len())
                .finish(),
            Self::LastFmSessionKeyChanged(s) => f
                .debug_struct("LastFmSessionKeyChanged")
                .field("len", &s.len())
                .finish(),
            Self::LastFmUrlChanged(s) => f.debug_tuple("LastFmUrlChanged").field(s).finish(),
            Self::ToggleSubmitNowPlaying(b) => {
                f.debug_tuple("ToggleSubmitNowPlaying").field(b).finish()
            }
            Self::SetScrobblePercent(p) => f.debug_tuple("SetScrobblePercent").field(p).finish(),
            Self::TestScrobble(s) => f.debug_tuple("TestScrobble").field(s).finish(),
            Self::ScrobbleTestFinished(s, r) => f
                .debug_tuple("ScrobbleTestFinished")
                .field(s)
                .field(r)
                .finish(),
            Self::AddLibraryFolderClicked => write!(f, "AddLibraryFolderClicked"),
            Self::LibraryFolderAdded(p) => f.debug_tuple("LibraryFolderAdded").field(p).finish(),
            Self::RemoveLibraryFolder(s) => f.debug_tuple("RemoveLibraryFolder").field(s).finish(),
            Self::RescanLibraryFolder(p) => f.debug_tuple("RescanLibraryFolder").field(p).finish(),
            Self::ScanLibraryForNewFiles => write!(f, "ScanLibraryForNewFiles"),
            Self::ReindexLibrary => write!(f, "ReindexLibrary"),
            Self::ToggleWatchLibrary(b) => f.debug_tuple("ToggleWatchLibrary").field(b).finish(),
            Self::ClearLibrary => write!(f, "ClearLibrary"),
            Self::ClearThumbnailCache => write!(f, "ClearThumbnailCache"),
            Self::TracksLoaded(rows, total) => f
                .debug_struct("TracksLoaded")
                .field("rows_len", &rows.len())
                .field("total", total)
                .finish(),
            Self::AlbumsLoaded(v) => f
                .debug_struct("AlbumsLoaded")
                .field("len", &v.len())
                .finish(),
            Self::ArtistsLoaded(v) => f
                .debug_struct("ArtistsLoaded")
                .field("len", &v.len())
                .finish(),
            Self::LibraryFoldersLoaded(v) => f
                .debug_struct("LibraryFoldersLoaded")
                .field("len", &v.len())
                .finish(),
            Self::TrackResolvedForPlayback(t) => f
                .debug_struct("TrackResolvedForPlayback")
                .field("is_some", &t.is_some())
                .finish(),
            Self::TrackResolvedForRestore(t) => f
                .debug_struct("TrackResolvedForRestore")
                .field("is_some", &t.is_some())
                .finish(),
            Self::TrackDetailsLoaded(t) => f
                .debug_struct("TrackDetailsLoaded")
                .field("is_some", &t.is_some())
                .finish(),
            Self::PlaybackAssetsLoaded {
                track_id,
                cover,
                blurred,
                next_cover,
                tech_info,
                lyrics,
            } => f
                .debug_struct("PlaybackAssetsLoaded")
                .field("track_id", track_id)
                .field("cover_len", &bytes_len(cover))
                .field("blurred_len", &bytes_len(blurred))
                .field("next_cover_len", &bytes_len(next_cover))
                .field("has_tech_info", &tech_info.is_some())
                .field("lyrics_len", &lyrics.as_ref().map(String::len))
                .finish(),
            Self::StorageStatsLoaded(stats, size) => f
                .debug_tuple("StorageStatsLoaded")
                .field(stats)
                .field(size)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_library_and_scan_new_messages() {
        let msg1 = Message::ScanLibraryForNewFiles;
        let msg2 = Message::ToggleWatchLibrary(true);
        let msg3 = Message::ToggleWatchLibrary(false);

        assert_eq!(format!("{:?}", msg1), "ScanLibraryForNewFiles");
        assert_eq!(format!("{:?}", msg2), "ToggleWatchLibrary(true)");
        assert_eq!(format!("{:?}", msg3), "ToggleWatchLibrary(false)");
    }
}
