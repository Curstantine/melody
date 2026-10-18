pub mod db;
pub mod intern;
pub mod metadata;
pub mod model;
pub mod paths;
pub mod playlist;
pub mod search;
pub mod thumbnail;
pub mod watcher;

pub use db::{
    is_supported_audio_path, DbError, LibraryDatabase, ScanSummary, SUPPORTED_AUDIO_EXTENSIONS,
};
pub use intern::InternedStr;
pub use metadata::{
    extract_cover_art, extract_metadata, extract_technical_info, AudioTechnicalInfo, MetadataError,
};
pub use model::{
    format_duration, AlbumSummary, ArtistSummary, NewTrack, PlaylistSummary, QueueRecord,
    SortDirection, SortField, Track, TrackIdHeader, TrackRow,
};
pub use paths::{
    cache_dir, config_dir, config_file, data_dir, database_file, playlists_dir, project_dirs,
    state_dir, state_file, thumbnail_cache_dir, user_layouts_dir, user_themes_dir,
};
pub use playlist::{
    append_tracks_to_m3u, is_supported_playlist_file, normalize_path, parse_m3u, parse_m3u_str,
    playlist_name_from_path, resolve_entry_path, sanitize_playlist_filename, write_m3u,
    ParsedPlaylist, PlaylistEntry, PlaylistError,
};
pub use search::TrackFilter;
pub use thumbnail::{
    clear_thumbnail_cache, compute_thumbnail_cache_key, generate_blurred_cover,
    get_or_create_thumbnail, get_thumbnail_cache_stats, CacheStats, THUMB_DIM_DEFAULT,
    THUMB_DIM_HIGH,
};
pub use watcher::{LibraryWatcher, WatcherError};
