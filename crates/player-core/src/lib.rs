pub mod db;
pub mod metadata;
pub mod model;
pub mod thumbnail;

pub use db::{DbError, LibraryDatabase, ScanSummary};
pub use metadata::{extract_cover_art, extract_metadata, MetadataError};
pub use model::{
    format_duration, AlbumSummary, ArtistSummary, NewTrack, SortDirection, SortField, Track,
    TrackIdHeader,
};
pub use thumbnail::{
    compute_thumbnail_cache_key, get_or_create_thumbnail, thumbnail_cache_dir,
    DEFAULT_THUMBNAIL_SIZE,
};
