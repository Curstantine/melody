pub mod db;
pub mod metadata;
pub mod model;
pub mod thumbnail;

pub use db::{DbError, LibraryDatabase, ScanSummary};
pub use metadata::{
    extract_cover_art, extract_metadata, extract_technical_info, AudioTechnicalInfo, MetadataError,
};
pub use model::{
    format_duration, AlbumSummary, ArtistSummary, NewTrack, SortDirection, SortField, Track,
    TrackIdHeader,
};
pub use thumbnail::{
    clear_thumbnail_cache, compute_thumbnail_cache_key, generate_blurred_cover,
    get_or_create_thumbnail, get_thumbnail_cache_stats, thumbnail_cache_dir, CacheStats,
    DEFAULT_THUMBNAIL_SIZE,
};

