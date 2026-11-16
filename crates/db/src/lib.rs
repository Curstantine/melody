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

/// Maximum bound variables per `WHERE ... IN (...)` lookup. Chunked lookups
/// stay clear of SQLite's variable limit for very large id/path sets.
pub(crate) const SQL_IN_CLAUSE_CHUNK_SIZE: usize = 500;

/// Rows per multi-row `INSERT` when replacing a queue's membership (3 bound
/// variables per row). Keeps huge queues such as "All Songs" to a few round
/// trips while staying clear of SQLite's variable limit.
pub(crate) const QUEUE_TRACKS_INSERT_CHUNK_ROWS: usize = 300;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Turso error: {0}")]
    Turso(#[from] TursoError),
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
    pub failed_files: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LibraryCounts {
    pub tracks: usize,
    pub albums: usize,
    pub artists: usize,
}
