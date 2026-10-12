pub mod db;
pub mod metadata;
pub mod model;
pub mod paths;
pub mod thumbnail;

pub use db::{DbError, LibraryDatabase, ScanSummary};
pub use metadata::{
    extract_cover_art, extract_metadata, extract_technical_info, AudioTechnicalInfo, MetadataError,
};
pub use model::{
    format_duration, AlbumSummary, ArtistSummary, NewTrack, SortDirection, SortField, Track,
    TrackIdHeader,
};
pub use paths::{
    cache_dir, config_dir, config_file, data_dir, database_file, project_dirs, thumbnail_cache_dir,
    user_themes_dir,
};
pub use thumbnail::{
    clear_thumbnail_cache, compute_thumbnail_cache_key, generate_blurred_cover,
    get_or_create_thumbnail, get_thumbnail_cache_stats, CacheStats, THUMB_DIM_DEFAULT,
    THUMB_DIM_HIGH,
};
