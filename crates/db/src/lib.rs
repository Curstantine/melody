use std::path::Path;
use std::sync::Arc;

use thiserror::Error;
use turso::{Connection, Database, Error as TursoError};

mod catalog;
mod folders;
mod helpers;
mod playlists;
mod queues;
mod scanner;
mod schema;
mod tracks;

#[cfg(test)]
mod tests;

/// Supported audio file extensions for library indexing.
pub const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "alac", "opus", "aiff",
];

/// Returns `true` if the given path has a supported audio file extension.
pub fn is_supported_audio_path<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            SUPPORTED_AUDIO_EXTENSIONS
                .iter()
                .any(|&s| ext.eq_ignore_ascii_case(s))
        })
        .unwrap_or(false)
}

/// Maximum bound variables per `WHERE ... IN (...)` lookup. Chunked lookups
/// stay clear of SQLite's variable limit for very large id/path sets.
pub const SQL_IN_CLAUSE_CHUNK_SIZE: usize = 500;

/// Rows per multi-row `INSERT` when replacing a queue's membership (3 bound
/// variables per row). Keeps huge queues such as "All Songs" to a few round
/// trips while staying clear of SQLite's variable limit.
pub const QUEUE_TRACKS_INSERT_CHUNK_ROWS: usize = 300;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Turso error: {0}")]
    Turso(#[from] TursoError),
    #[error("Lock error: {0}")]
    Lock(String),
}

#[derive(Debug, Clone)]
pub struct LibraryDatabase {
    _db: Arc<Database>,
    conn: Connection,
}

#[derive(Debug, Clone, Default)]
pub struct ScanSummary {
    pub scanned_files: usize,
    pub inserted_tracks: usize,
    pub updated_tracks: usize,
    pub failed_files: usize,
}
