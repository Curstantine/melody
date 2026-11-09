use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Connection, Database, Error as TursoError, Row};
use walkdir::WalkDir;

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

use metadata::extract_metadata;
use model::{
    format_duration, intern, AlbumSummary, ArtistSummary, NewTrack, PlaylistSummary, QueueRecord,
    SortDirection, SortField, Track, TrackFilter, TrackIdHeader, TrackRow,
};

pub mod playlist;
pub use playlist::{
    append_tracks_to_m3u, is_supported_playlist_file, normalize_path, parse_m3u,
    parse_m3u_preview_paths, parse_m3u_str, playlist_name_from_path, resolve_entry_path,
    sanitize_playlist_filename, write_m3u, ParsedPlaylist, PlaylistEntry, PlaylistError,
};

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

impl LibraryDatabase {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, DbError> {
        let path_str = db_path.as_ref().to_string_lossy();
        let db = turso::Builder::new_local(&path_str).build().await?;
        let conn = db.connect()?;
        let library_db = Self {
            _db: Arc::new(db),
            conn,
        };
        library_db.init_schema().await?;
        Ok(library_db)
    }

    pub async fn in_memory() -> Result<Self, DbError> {
        let db = turso::Builder::new_local(":memory:").build().await?;
        let conn = db.connect()?;
        let library_db = Self {
            _db: Arc::new(db),
            conn,
        };
        library_db.init_schema().await?;
        Ok(library_db)
    }

    pub const SCHEMA_VERSION: i32 = 1;

    pub async fn init_schema(&self) -> Result<(), DbError> {
        let current_version: i32 = {
            let mut rows = self.conn.query("PRAGMA user_version;", ()).await?;
            if let Some(row) = rows.next().await? {
                row.get::<i32>(0).unwrap_or(0)
            } else {
                0
            }
        };

        if current_version >= Self::SCHEMA_VERSION {
            return Ok(());
        }

        self.conn
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                album_artist TEXT,
                album TEXT NOT NULL,
                genre TEXT,
                duration_ms INTEGER NOT NULL,
                track_num INTEGER,
                disc_num INTEGER,
                year INTEGER,
                composer TEXT,
                label TEXT,
                musicbrainz_track_id TEXT,
                musicbrainz_artist_id TEXT,
                musicbrainz_album_id TEXT,
                added_timestamp INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album_artist ON tracks(album_artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album);
            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);
            CREATE INDEX IF NOT EXISTS idx_tracks_path ON tracks(file_path);

            CREATE TABLE IF NOT EXISTS library_folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                added_timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                track_count INTEGER NOT NULL DEFAULT 0,
                created_timestamp INTEGER NOT NULL,
                modified_timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playback_queues (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                current INTEGER NOT NULL DEFAULT 0,
                source_kind TEXT NOT NULL DEFAULT 'manual',
                source_value TEXT,
                source_playlist TEXT,
                created_timestamp INTEGER NOT NULL,
                modified_timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playback_queue_tracks (
                queue_id INTEGER NOT NULL,
                position INTEGER NOT NULL,
                track_id INTEGER NOT NULL,
                PRIMARY KEY (queue_id, position)
            );
            CREATE INDEX IF NOT EXISTS idx_queue_tracks_queue ON playback_queue_tracks(queue_id);

            CREATE TABLE IF NOT EXISTS playback_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                active_queue_id INTEGER
            );
            "#,
            )
            .await?;

        // The original `playlists` / `playlist_tracks` tables were never written
        // to by any code path; playlist contents are now owned by the M3U files
        // themselves and the database only keeps a `playlist_files` reference.
        let _ = self
            .conn
            .execute("DROP TABLE IF EXISTS playlist_tracks", ())
            .await;
        let _ = self
            .conn
            .execute("DROP TABLE IF EXISTS playlists", ())
            .await;

        // Safe migrations for databases created before schema additions
        let _ = self
            .conn
            .execute("ALTER TABLE tracks ADD COLUMN album_artist TEXT", ())
            .await;
        let _ = self
            .conn
            .execute("ALTER TABLE tracks ADD COLUMN disc_num INTEGER", ())
            .await;
        let _ = self
            .conn
            .execute("ALTER TABLE tracks ADD COLUMN year INTEGER", ())
            .await;
        let _ = self
            .conn
            .execute("ALTER TABLE tracks ADD COLUMN composer TEXT", ())
            .await;
        let _ = self
            .conn
            .execute("ALTER TABLE tracks ADD COLUMN label TEXT", ())
            .await;
        let _ = self
            .conn
            .execute(
                "ALTER TABLE tracks ADD COLUMN musicbrainz_track_id TEXT",
                (),
            )
            .await;
        let _ = self
            .conn
            .execute(
                "ALTER TABLE tracks ADD COLUMN musicbrainz_artist_id TEXT",
                (),
            )
            .await;
        let _ = self
            .conn
            .execute(
                "ALTER TABLE tracks ADD COLUMN musicbrainz_album_id TEXT",
                (),
            )
            .await;

        let _ = self
            .conn
            .execute(
                &format!("PRAGMA user_version = {};", Self::SCHEMA_VERSION),
                (),
            )
            .await;

        info!(
            "Database schema initialized successfully (version {})",
            Self::SCHEMA_VERSION
        );
        Ok(())
    }

    pub async fn insert_or_update_track(&self, track: &NewTrack) -> Result<i64, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        self.conn
            .execute(
                r#"
            INSERT INTO tracks (
                file_path, title, artist, album_artist, album, genre, duration_ms,
                track_num, disc_num, year, composer, label,
                musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(file_path) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album_artist = excluded.album_artist,
                album = excluded.album,
                genre = excluded.genre,
                duration_ms = excluded.duration_ms,
                track_num = excluded.track_num,
                disc_num = excluded.disc_num,
                year = excluded.year,
                composer = excluded.composer,
                label = excluded.label,
                musicbrainz_track_id = excluded.musicbrainz_track_id,
                musicbrainz_artist_id = excluded.musicbrainz_artist_id,
                musicbrainz_album_id = excluded.musicbrainz_album_id
            "#,
                params![
                    sanitize_db_text(&track.file_path),
                    sanitize_db_text(&track.title),
                    sanitize_db_text(&track.artist),
                    sanitize_db_opt(track.album_artist.as_deref()),
                    sanitize_db_text(&track.album),
                    sanitize_db_opt(track.genre.as_deref()),
                    track.duration_ms as i64,
                    track.track_num.map(|n| n as i64),
                    track.disc_num.map(|n| n as i64),
                    track.year.map(|n| n as i64),
                    sanitize_db_opt(track.composer.as_deref()),
                    sanitize_db_opt(track.label.as_deref()),
                    sanitize_db_opt(track.musicbrainz_track_id.as_deref()),
                    sanitize_db_opt(track.musicbrainz_artist_id.as_deref()),
                    sanitize_db_opt(track.musicbrainz_album_id.as_deref()),
                    now,
                ],
            )
            .await?;

        let id = self.conn.last_insert_rowid();
        Ok(id)
    }

    pub async fn insert_tracks_batch(&self, tracks: &[NewTrack]) -> Result<usize, DbError> {
        if tracks.is_empty() {
            return Ok(0);
        }

        let tx = self.conn.unchecked_transaction().await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut count = 0;
        for track in tracks {
            tx.execute(
                r#"
                INSERT INTO tracks (
                    file_path, title, artist, album_artist, album, genre, duration_ms,
                    track_num, disc_num, year, composer, label,
                    musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                ON CONFLICT(file_path) DO UPDATE SET
                    title = excluded.title,
                    artist = excluded.artist,
                    album_artist = excluded.album_artist,
                    album = excluded.album,
                    genre = excluded.genre,
                    duration_ms = excluded.duration_ms,
                    track_num = excluded.track_num,
                    disc_num = excluded.disc_num,
                    year = excluded.year,
                    composer = excluded.composer,
                    label = excluded.label,
                    musicbrainz_track_id = excluded.musicbrainz_track_id,
                    musicbrainz_artist_id = excluded.musicbrainz_artist_id,
                    musicbrainz_album_id = excluded.musicbrainz_album_id
                "#,
                params![
                    sanitize_db_text(&track.file_path),
                    sanitize_db_text(&track.title),
                    sanitize_db_text(&track.artist),
                    sanitize_db_opt(track.album_artist.as_deref()),
                    sanitize_db_text(&track.album),
                    sanitize_db_opt(track.genre.as_deref()),
                    track.duration_ms as i64,
                    track.track_num.map(|n| n as i64),
                    track.disc_num.map(|n| n as i64),
                    track.year.map(|n| n as i64),
                    sanitize_db_opt(track.composer.as_deref()),
                    sanitize_db_opt(track.label.as_deref()),
                    sanitize_db_opt(track.musicbrainz_track_id.as_deref()),
                    sanitize_db_opt(track.musicbrainz_artist_id.as_deref()),
                    sanitize_db_opt(track.musicbrainz_album_id.as_deref()),
                    now,
                ],
            )
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    pub async fn get_track_by_id(&self, id: i64) -> Result<Option<Track>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks WHERE id = ?1
            "#,
                params![id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(map_track_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_track_by_path(&self, file_path: &str) -> Result<Option<Track>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks WHERE file_path = ?1
            "#,
                params![file_path.to_string()],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(map_track_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_total_track_count(
        &self,
        filter: Option<&TrackFilter>,
    ) -> Result<usize, DbError> {
        let mut binds: Vec<turso::Value> = Vec::new();
        let clause = filter_clause(filter, &mut binds);
        let query = format!("SELECT COUNT(*) FROM tracks{}", clause);

        let mut rows = self
            .conn
            .query(&query, turso::params_from_iter(binds))
            .await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count as usize)
        } else {
            Ok(0)
        }
    }

    /// Read queries with LIMIT and OFFSET based on the active viewport
    /// Never pull all tracks into a `Vec<Track>`
    pub async fn get_tracks_window(
        &self,
        offset: usize,
        limit: usize,
        filter: Option<&TrackFilter>,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Result<Vec<Track>, DbError> {
        let sort_col = sort_column(sort_field);

        let dir_str = match sort_dir {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };

        let mut tracks = Vec::with_capacity(limit);
        let mut binds: Vec<turso::Value> = Vec::new();
        let clause = filter_clause(filter, &mut binds);
        binds.push(turso::Value::from(limit as i64));
        binds.push(turso::Value::from(offset as i64));

        let query = format!(
            r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks{}
            ORDER BY {} {}
            LIMIT ? OFFSET ?
            "#,
            clause, sort_col, dir_str
        );

        let mut rows = self
            .conn
            .query(&query, turso::params_from_iter(binds))
            .await?;

        while let Some(row) = rows.next().await? {
            tracks.push(map_track_row(&row)?);
        }

        Ok(tracks)
    }

    /// Reads a narrow slice of track rows for the virtualized library list.
    ///
    /// Only the rendered columns are selected and the repeated `artist`/`album`
    /// values are interned, so a bounded window costs a few hundred KB
    /// regardless of total library size.
    pub async fn get_track_rows_window(
        &self,
        offset: usize,
        limit: usize,
        filter: Option<&TrackFilter>,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Result<Vec<TrackRow>, DbError> {
        let sort_col = sort_column(sort_field);

        let dir_str = match sort_dir {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };

        let mut rows_out = Vec::with_capacity(limit);
        let mut binds: Vec<turso::Value> = Vec::new();
        let clause = filter_clause(filter, &mut binds);
        binds.push(turso::Value::from(limit as i64));
        binds.push(turso::Value::from(offset as i64));

        let query = format!(
            r#"
            SELECT id, title, artist, album, duration_ms, file_path, year, track_num, disc_num, album_artist, genre
            FROM tracks{}
            ORDER BY {} {}
            LIMIT ? OFFSET ?
            "#,
            clause, sort_col, dir_str
        );

        let mut rows = self
            .conn
            .query(&query, turso::params_from_iter(binds))
            .await?;

        while let Some(row) = rows.next().await? {
            rows_out.push(map_track_row_summary(&row)?);
        }

        Ok(rows_out)
    }

    /// Loads every matching [`TrackRow`] without pagination.
    ///
    /// Used when materializing a playback queue from the active library scope
    /// (album/artist/search). Callers should keep the filter reasonably narrow:
    /// an empty filter loads the entire library.
    pub async fn get_all_track_rows_filtered(
        &self,
        filter: Option<&TrackFilter>,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Result<Vec<TrackRow>, DbError> {
        let sort_col = sort_column(sort_field);
        let dir_str = match sort_dir {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };

        let mut binds: Vec<turso::Value> = Vec::new();
        let clause = filter_clause(filter, &mut binds);
        let query = format!(
            r#"
            SELECT id, title, artist, album, duration_ms, file_path, year, track_num, disc_num, album_artist, genre
            FROM tracks{}
            ORDER BY {} {}
            "#,
            clause, sort_col, dir_str
        );

        let mut rows = self
            .conn
            .query(&query, turso::params_from_iter(binds))
            .await?;

        let mut rows_out = Vec::new();
        while let Some(row) = rows.next().await? {
            rows_out.push(map_track_row_summary(&row)?);
        }
        Ok(rows_out)
    }

    /// Fetches the filesystem path for every requested track id in a single
    /// `WHERE id IN (...)` query, replacing the per-id round trips used by the
    /// album cover loader.
    pub async fn get_track_paths_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<HashMap<i64, String>, DbError> {
        let mut out = HashMap::with_capacity(ids.len());
        if ids.is_empty() {
            return Ok(out);
        }

        for chunk in ids.chunks(SQL_IN_CLAUSE_CHUNK_SIZE) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT id, file_path FROM tracks WHERE id IN ({})",
                placeholders
            );

            let mut rows = self
                .conn
                .query(&query, turso::params_from_iter(chunk.iter().copied()))
                .await?;

            while let Some(row) = rows.next().await? {
                let id: i64 = row.get(0)?;
                let path: String = row.get(1)?;
                out.insert(id, path);
            }
        }
        Ok(out)
    }

    /// Fetches lightweight [`TrackRow`]s for every requested id in chunked
    /// `WHERE id IN (...)` queries. Returns a map so callers can restore their
    /// own ordering (e.g. playlist order).
    pub async fn get_track_rows_by_ids(
        &self,
        ids: &[i64],
    ) -> Result<HashMap<i64, TrackRow>, DbError> {
        let mut out = HashMap::with_capacity(ids.len());
        if ids.is_empty() {
            return Ok(out);
        }

        for chunk in ids.chunks(SQL_IN_CLAUSE_CHUNK_SIZE) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT id, title, artist, album, duration_ms, file_path, year, track_num, disc_num, album_artist, genre FROM tracks WHERE id IN ({})",
                placeholders
            );

            let mut rows = self
                .conn
                .query(&query, turso::params_from_iter(chunk.iter().copied()))
                .await?;

            while let Some(row) = rows.next().await? {
                let summary = map_track_row_summary(&row)?;
                out.insert(summary.id, summary);
            }
        }
        Ok(out)
    }

    /// Lightweight in-memory index struct for global shuffle, playlist generation, or totals
    pub async fn get_all_track_headers(&self) -> Result<Vec<TrackIdHeader>, DbError> {
        let mut rows = self
            .conn
            .query("SELECT id, duration_ms FROM tracks ORDER BY id ASC", ())
            .await?;

        let mut headers = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let dur: i64 = row.get(1)?;
            headers.push(TrackIdHeader {
                id,
                duration_ms: dur as u32,
            });
        }
        Ok(headers)
    }

    pub async fn get_albums(&self) -> Result<Vec<AlbumSummary>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
            SELECT 
                album,
                COALESCE(
                    NULLIF(album_artist, ''),
                    CASE WHEN COUNT(DISTINCT artist) = 1 THEN MIN(artist) ELSE 'Various Artists' END
                ) as dominant_artist,
                COUNT(*) as track_count,
                SUM(duration_ms) as total_duration,
                MIN(year) as year,
                MIN(id) as cover_track_id,
                MAX(added_timestamp) as added_timestamp
            FROM tracks
            GROUP BY album, COALESCE(NULLIF(album_artist, ''), '')
            ORDER BY album ASC
            "#,
                (),
            )
            .await?;

        let mut albums = Vec::new();
        while let Some(row) = rows.next().await? {
            let album: String = row.get(0)?;
            let artist: String = row.get(1)?;
            let count: i64 = row.get(2)?;
            let duration: i64 = row.get(3)?;
            let year: Option<u32> = row.get(4)?;
            let cover_track_id: Option<i64> = row.get(5)?;
            let added_timestamp: i64 = row.get(6)?;
            albums.push(AlbumSummary {
                album: intern::intern(&album),
                artist: intern::intern(&artist),
                track_count: count as usize,
                total_duration_ms: duration as u64,
                year,
                cover_track_id,
                added_timestamp,
                year_label: year.map(|y| format!("{}", y)).unwrap_or_default(),
                tracks_label: format!("{} tracks", count),
                duration_label: format_duration(duration as u64),
            });
        }
        Ok(albums)
    }

    pub async fn get_artists(&self) -> Result<Vec<ArtistSummary>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
            SELECT artist, COUNT(*) as track_count, COUNT(DISTINCT album) as album_count
            FROM tracks
            GROUP BY artist
            ORDER BY artist ASC
            "#,
                (),
            )
            .await?;

        let mut artists = Vec::new();
        while let Some(row) = rows.next().await? {
            let artist: String = row.get(0)?;
            let track_count: i64 = row.get(1)?;
            let album_count: i64 = row.get(2)?;
            artists.push(ArtistSummary {
                artist: intern::intern(&artist),
                track_count: track_count as usize,
                album_count: album_count as usize,
                stats_label: format!("{} albums \u{2022} {} tracks", album_count, track_count),
            });
        }
        Ok(artists)
    }

    pub async fn delete_track(&self, id: i64) -> Result<bool, DbError> {
        let affected = self
            .conn
            .execute("DELETE FROM tracks WHERE id = ?1", params![id])
            .await?;
        Ok(affected > 0)
    }

    /// Retrieves all configured music library scan paths
    pub async fn get_library_folders(&self) -> Result<Vec<String>, DbError> {
        let mut rows = self
            .conn
            .query("SELECT path FROM library_folders ORDER BY id ASC", ())
            .await?;

        let mut folders = Vec::new();
        while let Some(row) = rows.next().await? {
            let path: String = row.get(0)?;
            folders.push(path);
        }
        Ok(folders)
    }

    /// Adds a directory path to the library folders list if not already present
    pub async fn add_library_folder(&self, path: &str) -> Result<bool, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO library_folders (path, added_timestamp) VALUES (?1, ?2)",
                params![path.to_string(), now],
            )
            .await?;
        Ok(affected > 0)
    }

    /// Removes a directory path from the library folders list
    pub async fn remove_library_folder(&self, path: &str) -> Result<bool, DbError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM library_folders WHERE path = ?1",
                params![path.to_string()],
            )
            .await?;
        Ok(affected > 0)
    }

    /// Wipes all indexed tracks from the database.
    ///
    /// Playlist file references are intentionally preserved: the referenced
    /// `.m3u` files remain valid on disk and are re-resolved against the
    /// library on the next scan.
    pub async fn clear_all_tracks(&self) -> Result<u64, DbError> {
        let count = self.conn.execute("DELETE FROM tracks", ()).await?;
        let _ = self.conn.execute("VACUUM", ()).await;
        intern::clear();
        Ok(count)
    }

    // ── Playlist file references ─────────────────────────────────────────────

    /// Registers or refreshes a playlist file reference.
    ///
    /// `created_timestamp` is only applied on first insert; the modification
    /// time and cached track count are always updated.
    pub async fn upsert_playlist_file(
        &self,
        file_path: &str,
        name: &str,
        track_count: usize,
    ) -> Result<i64, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        self.conn
            .execute(
                r#"
            INSERT INTO playlist_files (file_path, name, track_count, created_timestamp, modified_timestamp)
            VALUES (?1, ?2, ?3, ?4, ?4)
            ON CONFLICT(file_path) DO UPDATE SET
                name = excluded.name,
                track_count = excluded.track_count,
                modified_timestamp = excluded.modified_timestamp
            "#,
                params![
                    sanitize_db_text(file_path),
                    sanitize_db_text(name),
                    track_count as i64,
                    now,
                ],
            )
            .await?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Removes a playlist file reference. The file on disk is left untouched.
    pub async fn remove_playlist_file(&self, id: i64) -> Result<bool, DbError> {
        let affected = self
            .conn
            .execute("DELETE FROM playlist_files WHERE id = ?1", params![id])
            .await?;
        Ok(affected > 0)
    }

    /// Removes a playlist file reference by path. The file is left untouched.
    pub async fn remove_playlist_file_by_path(&self, file_path: &str) -> Result<bool, DbError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM playlist_files WHERE file_path = ?1",
                params![file_path.to_string()],
            )
            .await?;
        Ok(affected > 0)
    }

    /// Renames the display name of a referenced playlist.
    pub async fn rename_playlist_file(&self, id: i64, name: &str) -> Result<bool, DbError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let affected = self
            .conn
            .execute(
                "UPDATE playlist_files SET name = ?1, modified_timestamp = ?2 WHERE id = ?3",
                params![sanitize_db_text(name), now, id],
            )
            .await?;
        Ok(affected > 0)
    }

    /// Loads every referenced playlist, ordered by name.
    pub async fn get_playlists(&self) -> Result<Vec<PlaylistSummary>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
            SELECT id, file_path, name, track_count, created_timestamp, modified_timestamp
            FROM playlist_files
            ORDER BY name COLLATE NOCASE ASC
            "#,
                (),
            )
            .await?;

        let managed_dir = paths::playlists_dir();
        let mut playlists = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let file_path: String = row.get(1)?;
            let name: String = row.get(2)?;
            let track_count: i64 = row.get(3)?;
            let created_timestamp: i64 = row.get(4)?;
            let modified_timestamp: i64 = row.get(5)?;

            let managed = Path::new(&file_path).starts_with(managed_dir);
            playlists.push(PlaylistSummary {
                id,
                name,
                file_path,
                track_count: track_count as usize,
                created_timestamp,
                modified_timestamp,
                managed,
                tracks_label: format!("{} tracks", track_count),
            });
        }
        Ok(playlists)
    }

    /// Resolves absolute filesystem paths to track ids in a single query.
    ///
    /// Used to map M3U playlist entries onto indexed library tracks when
    /// building a playback queue. Unmatched paths are simply absent from the map.
    pub async fn get_track_ids_by_paths(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, i64>, DbError> {
        let mut out = HashMap::new();
        if paths.is_empty() {
            return Ok(out);
        }

        for chunk in paths.chunks(SQL_IN_CLAUSE_CHUNK_SIZE) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT id, file_path FROM tracks WHERE file_path IN ({})",
                placeholders
            );

            let mut rows = self
                .conn
                .query(&query, turso::params_from_iter(chunk.iter().cloned()))
                .await?;

            while let Some(row) = rows.next().await? {
                let id: i64 = row.get(0)?;
                let path: String = row.get(1)?;
                out.insert(path, id);
            }
        }
        Ok(out)
    }

    /// Extracts metadata and indexes audio files in parallel using a bounded worker pool.
    ///
    /// The workload is dynamically distributed across worker threads matching system parallelism,
    /// while database writes are batched and executed sequentially on the async connection.
    pub async fn index_files_parallel<F: FnMut(usize, usize)>(
        &self,
        files: &[PathBuf],
        mut progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let total = files.len();
        if let Some(ref mut cb) = progress_cb {
            cb(0, total);
        }

        if files.is_empty() {
            return Ok(ScanSummary::default());
        }

        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 16)
            .min(total);

        let files = Arc::new(files.to_vec());
        let next_idx = Arc::new(AtomicUsize::new(0));
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);

        let mut handles = Vec::with_capacity(num_workers);
        for w in 0..num_workers {
            let files = Arc::clone(&files);
            let next_idx = Arc::clone(&next_idx);
            let tx = tx.clone();
            let handle = std::thread::Builder::new()
                .name(format!("melody-indexer-{}", w))
                .spawn(move || loop {
                    let idx = next_idx.fetch_add(1, Ordering::Relaxed);
                    if idx >= files.len() {
                        break;
                    }
                    let path = &files[idx];
                    let parsed = extract_metadata(path);
                    if tx.blocking_send((path.clone(), parsed)).is_err() {
                        break;
                    }
                });
            if let Ok(h) = handle {
                handles.push(h);
            }
        }
        drop(tx);

        let mut summary = ScanSummary::default();
        let mut batch: Vec<NewTrack> = Vec::with_capacity(100);
        let notify_interval = (total / 100).max(5);

        while let Some((path, parsed)) = rx.recv().await {
            summary.scanned_files += 1;

            match parsed {
                Ok(track) => {
                    batch.push(track);
                    if batch.len() >= 100 {
                        self.flush_scan_batch(&batch, &mut summary).await;
                        batch.clear();
                    }
                }
                Err(e) => {
                    warn!("Failed to extract metadata for {:?}: {}", path, e);
                    summary.failed_files += 1;
                }
            }

            if let Some(ref mut cb) = progress_cb {
                if total <= 100
                    || summary.scanned_files == total
                    || summary.scanned_files % notify_interval == 0
                {
                    cb(summary.scanned_files, total);
                }
            }
        }

        if !batch.is_empty() {
            self.flush_scan_batch(&batch, &mut summary).await;
        }

        for h in handles {
            let _ = h.join();
        }

        if let Some(ref mut cb) = progress_cb {
            cb(summary.scanned_files, total);
        }

        Ok(summary)
    }

    /// Recursively scan directories and index all supported audio files.
    ///
    /// Candidate audio files are gathered first to determine the total workload,
    /// enabling the progress callback to report `(current, total)`. Metadata
    /// extraction is dispatched to parallel worker threads to maximize CPU utilization.
    pub async fn scan_directories<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        dir_paths: &[P],
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut candidate_files: Vec<PathBuf> = Vec::new();
        for dir_path in dir_paths {
            for entry in WalkDir::new(dir_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && is_supported_audio_path(path) {
                    candidate_files.push(path.to_path_buf());
                }
            }
        }

        candidate_files.sort();
        candidate_files.dedup();

        let summary = self
            .index_files_parallel(&candidate_files, progress_cb)
            .await?;

        info!(
            "Directory scan finished: {} files scanned, {} tracks indexed, {} errors",
            summary.scanned_files, summary.inserted_tracks, summary.failed_files
        );

        Ok(summary)
    }

    /// Recursively scan directory and index all supported audio files.
    pub async fn scan_directory<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        dir_path: P,
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        self.scan_directories(&[dir_path], progress_cb).await
    }

    /// Filters a list of file paths, returning only those that do NOT currently exist in the database.
    pub async fn filter_new_paths<P: AsRef<Path>>(
        &self,
        paths: &[P],
    ) -> Result<Vec<PathBuf>, DbError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        let string_paths: Vec<String> = paths
            .iter()
            .map(|p| p.as_ref().to_string_lossy().to_string())
            .collect();

        let mut existing_set = HashSet::new();
        for chunk in string_paths.chunks(SQL_IN_CLAUSE_CHUNK_SIZE) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT file_path FROM tracks WHERE file_path IN ({})",
                placeholders
            );
            let mut rows = self
                .conn
                .query(&query, turso::params_from_iter(chunk.iter().cloned()))
                .await?;
            while let Some(row) = rows.next().await? {
                let p: String = row.get(0)?;
                existing_set.insert(p);
            }
        }

        let mut new_paths = Vec::new();
        for path in paths {
            let p_ref = path.as_ref();
            let p_str = p_ref.to_string_lossy();
            if !existing_set.contains(p_str.as_ref()) {
                new_paths.push(p_ref.to_path_buf());
            }
        }

        new_paths.sort();
        new_paths.dedup();
        Ok(new_paths)
    }

    /// Recursively scans candidate paths (files or directories) and indexes ONLY new supported audio files.
    ///
    /// Candidate files that are already indexed are skipped without extracting metadata,
    /// avoiding redundant FFmpeg demuxing and database writes.
    pub async fn scan_new_files<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        paths: &[P],
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut candidate_files: Vec<PathBuf> = Vec::new();

        for p in paths {
            let path = p.as_ref();
            if path.is_dir() {
                for entry in WalkDir::new(path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path();
                    if entry_path.is_file() && is_supported_audio_path(entry_path) {
                        candidate_files.push(entry_path.to_path_buf());
                    }
                }
            } else if path.is_file() && is_supported_audio_path(path) {
                candidate_files.push(path.to_path_buf());
            }
        }

        candidate_files.sort();
        candidate_files.dedup();

        let new_files = self.filter_new_paths(&candidate_files).await?;
        let summary = self.index_files_parallel(&new_files, progress_cb).await?;

        info!(
            "New files scan finished: {} candidate files checked, {} new files scanned, {} tracks inserted, {} errors",
            candidate_files.len(),
            summary.scanned_files,
            summary.inserted_tracks,
            summary.failed_files
        );

        Ok(summary)
    }

    /// Recursively scan directories and index ONLY new supported audio files.
    pub async fn scan_directories_new<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        dir_paths: &[P],
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        self.scan_new_files(dir_paths, progress_cb).await
    }

    /// Inserts a batch within a transaction, falling back to individual inserts
    /// (and counting failures) when the batched statement errors out.
    async fn flush_scan_batch(&self, batch: &[NewTrack], summary: &mut ScanSummary) {
        match self.insert_tracks_batch(batch).await {
            Ok(inserted) => {
                summary.inserted_tracks += inserted;
            }
            Err(e) => {
                warn!(
                    "Batch insertion failed for {} tracks: {}. Falling back to individual inserts...",
                    batch.len(),
                    e
                );
                for t in batch {
                    match self.insert_or_update_track(t).await {
                        Ok(_) => summary.inserted_tracks += 1,
                        Err(err) => {
                            warn!("Failed to insert track {:?}: {}", t.file_path, err);
                            summary.failed_files += 1;
                        }
                    }
                }
            }
        }
    }

    // ── Playback queues ────────────────────────────────────────────────────

    /// Lists every queue's metadata and member count, without loading tracks.
    pub async fn list_queues(&self) -> Result<Vec<QueueRecord>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
                SELECT q.id, q.name, q.current, q.source_kind, q.source_value,
                       q.source_playlist, COUNT(t.track_id)
                FROM playback_queues q
                LEFT JOIN playback_queue_tracks t ON t.queue_id = q.id
                GROUP BY q.id
                ORDER BY q.id ASC
                "#,
                (),
            )
            .await?;

        let mut queues = Vec::new();
        while let Some(row) = rows.next().await? {
            let count: i64 = row.get(6)?;
            queues.push(QueueRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                current: row.get(2)?,
                source_kind: row.get(3)?,
                source_value: row.get(4)?,
                source_playlist: row.get(5)?,
                track_count: count as usize,
            });
        }
        Ok(queues)
    }

    /// Loads the ordered track rows of a single queue.
    pub async fn load_queue_tracks(&self, queue_id: i64) -> Result<Vec<TrackRow>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
                SELECT tr.id, tr.title, tr.artist, tr.album, tr.duration_ms, tr.file_path, tr.year, tr.track_num, tr.disc_num, tr.album_artist, tr.genre
                FROM playback_queue_tracks qt
                JOIN tracks tr ON tr.id = qt.track_id
                WHERE qt.queue_id = ?1
                ORDER BY qt.position ASC
                "#,
                params![queue_id],
            )
            .await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(map_track_row_summary(&row)?);
        }
        Ok(out)
    }

    /// Inserts or updates a queue's metadata row.
    pub async fn upsert_queue(&self, record: &QueueRecord) -> Result<(), DbError> {
        let now = now_secs();
        self.conn
            .execute(
                r#"
                INSERT INTO playback_queues (
                    id, name, current, source_kind, source_value, source_playlist,
                    created_timestamp, modified_timestamp
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    current = excluded.current,
                    source_kind = excluded.source_kind,
                    source_value = excluded.source_value,
                    source_playlist = excluded.source_playlist,
                    modified_timestamp = excluded.modified_timestamp
                "#,
                params![
                    record.id,
                    sanitize_db_text(&record.name),
                    record.current,
                    sanitize_db_text(&record.source_kind),
                    sanitize_db_opt(record.source_value.as_deref()),
                    sanitize_db_opt(record.source_playlist.as_deref()),
                    now,
                ],
            )
            .await?;
        Ok(())
    }

    /// Replaces a queue's ordered membership with the given track ids.
    ///
    /// Membership is written with chunked multi-row `INSERT`s (one statement
    /// per chunk instead of one per track) so persisting huge queues such as
    /// "All Songs" stays proportional to a few round trips.
    pub async fn replace_queue_tracks(&self, queue_id: i64, ids: &[i64]) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction().await?;
        tx.execute(
            "DELETE FROM playback_queue_tracks WHERE queue_id = ?1",
            params![queue_id],
        )
        .await?;
        let mut position: i64 = 0;
        for chunk in ids.chunks(QUEUE_TRACKS_INSERT_CHUNK_ROWS) {
            let mut sql = String::from(
                "INSERT INTO playback_queue_tracks (queue_id, position, track_id) VALUES ",
            );
            let mut binds: Vec<turso::Value> = Vec::with_capacity(chunk.len() * 3);
            for (i, id) in chunk.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str("(?, ?, ?)");
                binds.push(turso::Value::from(queue_id));
                binds.push(turso::Value::from(position));
                binds.push(turso::Value::from(*id));
                position += 1;
            }
            tx.execute(sql.as_str(), turso::params_from_iter(binds))
                .await?;
        }
        tx.execute(
            "UPDATE playback_queues SET modified_timestamp = ?2 WHERE id = ?1",
            params![queue_id, now_secs()],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Updates only the cursor position of a queue.
    pub async fn set_queue_current(&self, queue_id: i64, current: i64) -> Result<(), DbError> {
        self.conn
            .execute(
                "UPDATE playback_queues SET current = ?2 WHERE id = ?1",
                params![queue_id, current],
            )
            .await?;
        Ok(())
    }

    /// Removes a queue and its tracks.
    pub async fn delete_queue(&self, queue_id: i64) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction().await?;
        tx.execute(
            "DELETE FROM playback_queue_tracks WHERE queue_id = ?1",
            params![queue_id],
        )
        .await?;
        tx.execute(
            "DELETE FROM playback_queues WHERE id = ?1",
            params![queue_id],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_active_queue_id(&self) -> Result<Option<i64>, DbError> {
        let mut rows = self
            .conn
            .query(
                "SELECT active_queue_id FROM playback_state WHERE id = 1",
                (),
            )
            .await?;
        if let Some(row) = rows.next().await? {
            let active: Option<i64> = row.get(0)?;
            Ok(active)
        } else {
            Ok(None)
        }
    }

    pub async fn set_active_queue_id(&self, id: Option<i64>) -> Result<(), DbError> {
        self.conn
            .execute(
                r#"
                INSERT INTO playback_state (id, active_queue_id) VALUES (1, ?1)
                ON CONFLICT(id) DO UPDATE SET active_queue_id = excluded.active_queue_id
                "#,
                params![id],
            )
            .await?;
        Ok(())
    }
}

/// Current Unix timestamp in whole seconds.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Maps a sort field to its SQL ordering column expression.
fn sort_column(sort_field: SortField) -> &'static str {
    match sort_field {
        SortField::Title => "title",
        SortField::Artist => "artist",
        SortField::Album => "album",
        SortField::Duration => "duration_ms",
        SortField::TrackNum => "track_num",
        SortField::AddedDate => "added_timestamp",
    }
}

/// Builds a `WHERE ...` clause from a [`TrackFilter`], pushing bound values in
/// placeholder order. Returns an empty string when no filtering is needed.
fn filter_clause(filter: Option<&TrackFilter>, binds: &mut Vec<turso::Value>) -> String {
    let Some(filter) = filter.filter(|f| !f.is_empty()) else {
        return String::new();
    };

    let mut clauses: Vec<String> = Vec::new();

    for term in &filter.text {
        clauses.push("(title LIKE ? OR artist LIKE ? OR album LIKE ?)".to_string());
        let pattern = format!("%{}%", term);
        binds.push(turso::Value::from(pattern.clone()));
        binds.push(turso::Value::from(pattern.clone()));
        binds.push(turso::Value::from(pattern));
    }
    if let Some(value) = &filter.title {
        clauses.push("title LIKE ?".to_string());
        binds.push(turso::Value::from(format!("%{}%", value)));
    }
    if let Some(value) = &filter.album {
        clauses.push("album = ?".to_string());
        binds.push(turso::Value::from(value.clone()));
    }
    if let Some(value) = &filter.album_artist {
        clauses.push("album_artist = ?".to_string());
        binds.push(turso::Value::from(value.clone()));
    }
    if let Some(value) = &filter.artist {
        clauses.push("artist = ?".to_string());
        binds.push(turso::Value::from(value.clone()));
    }
    if let Some(value) = &filter.genre {
        clauses.push("genre = ?".to_string());
        binds.push(turso::Value::from(value.clone()));
    }
    if let Some(year) = filter.year {
        clauses.push("year = CAST(? AS INTEGER)".to_string());
        binds.push(turso::Value::from(year.to_string()));
    }

    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

fn map_track_row(row: &Row) -> Result<Track, DbError> {
    Ok(Track {
        id: row.get(0)?,
        file_path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album_artist: row.get(4)?,
        album: row.get(5)?,
        genre: row.get(6)?,
        duration_ms: {
            let dur: i64 = row.get(7)?;
            dur as u64
        },
        track_num: {
            let num: Option<i64> = row.get(8)?;
            num.map(|n| n as u32)
        },
        disc_num: {
            let disc: Option<i64> = row.get(9)?;
            disc.map(|n| n as u32)
        },
        year: {
            let yr: Option<i64> = row.get(10)?;
            yr.map(|n| n as u32)
        },
        composer: row.get(11)?,
        label: row.get(12)?,
        musicbrainz_track_id: row.get(13)?,
        musicbrainz_artist_id: row.get(14)?,
        musicbrainz_album_id: row.get(15)?,
        added_timestamp: row.get(16)?,
    })
}

fn map_track_row_summary(row: &Row) -> Result<TrackRow, DbError> {
    let artist: String = row.get(2)?;
    let album: String = row.get(3)?;
    let duration: i64 = row.get(4)?;
    let year: Option<i64> = row.get(6)?;
    let track_num: Option<i64> = row.get(7)?;
    let disc_num: Option<i64> = row.get(8)?;
    let album_artist: Option<String> = row.get(9)?;
    let genre: Option<String> = row.get(10)?;
    Ok(TrackRow {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: intern::intern(&artist),
        album: intern::intern(&album),
        duration_ms: duration as u64,
        file_path: row.get(5)?,
        duration_label: format_duration(duration as u64),
        year: year.map(|y| y as u32),
        track_num: track_num.map(|n| n as u32),
        disc_num: disc_num.map(|n| n as u32),
        album_artist: album_artist.as_deref().map(intern::intern),
        genre: genre.as_deref().map(intern::intern),
    })
}

fn sanitize_db_text(s: &str) -> Cow<'_, str> {
    let bytes = s.as_bytes();
    if !bytes.contains(&0) {
        return Cow::Borrowed(s);
    }
    let filtered: Vec<u8> = bytes.iter().copied().filter(|&b| b != 0).collect();
    if let Ok(v) = String::from_utf8(filtered.clone()) {
        Cow::Owned(v)
    } else {
        Cow::Owned(String::from_utf8_lossy(&filtered).into_owned())
    }
}

fn sanitize_db_opt(s: Option<&str>) -> Option<turso::Value> {
    s.map(|v| match sanitize_db_text(v) {
        Cow::Borrowed(b) => turso::Value::from(b),
        Cow::Owned(o) => turso::Value::from(o),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_audio_extensions_and_path_check() {
        assert!(SUPPORTED_AUDIO_EXTENSIONS.contains(&"mp3"));
        assert!(SUPPORTED_AUDIO_EXTENSIONS.contains(&"flac"));
        assert!(SUPPORTED_AUDIO_EXTENSIONS.contains(&"ogg"));

        assert!(is_supported_audio_path("song.mp3"));
        assert!(is_supported_audio_path("artist/track.FLAC"));
        assert!(is_supported_audio_path("/music/album/01.opus"));
        assert!(!is_supported_audio_path("image.jpg"));
        assert!(!is_supported_audio_path("notes.txt"));
        assert!(!is_supported_audio_path("file_without_ext"));
    }

    #[tokio::test]
    async fn test_database_lifecycle_and_pagination() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        // Insert test tracks
        for i in 1..=105 {
            let track = NewTrack {
                file_path: format!("/music/track_{:03}.flac", i),
                title: format!("Song {:03}", i),
                artist: if i % 2 == 0 {
                    "Artist Beta".to_string()
                } else {
                    "Artist Alpha".to_string()
                },
                album: if i <= 50 {
                    "Album One".to_string()
                } else {
                    "Album Two".to_string()
                },
                genre: Some("Electronic".to_string()),
                duration_ms: (i as u64) * 2000,
                track_num: Some(i as u32),
                year: Some(2000),
                ..Default::default()
            };
            db.insert_or_update_track(&track)
                .await
                .expect("Failed to insert track");
        }

        // Test total count
        let total = db.get_total_track_count(None).await.expect("Count failed");
        assert_eq!(total, 105);

        // Test filtered count
        let alpha_filter = TrackFilter::parse("Alpha");
        let alpha_count = db
            .get_total_track_count(Some(&alpha_filter))
            .await
            .expect("Filter count failed");
        assert_eq!(alpha_count, 53); // odd numbers from 1 to 105

        // Scoped album filter is exact (not a substring of title/artist)
        let album_one = TrackFilter::parse("album:\"Album One\"");
        let album_one_count = db
            .get_total_track_count(Some(&album_one))
            .await
            .expect("Album filter count failed");
        assert_eq!(album_one_count, 50);

        let album_rows = db
            .get_all_track_rows_filtered(
                Some(&album_one),
                SortField::TrackNum,
                SortDirection::Ascending,
            )
            .await
            .expect("Album rows failed");
        assert_eq!(album_rows.len(), 50);
        assert!(album_rows.iter().all(|t| t.album.as_ref() == "Album One"));

        // Year filter binds an integer column through a text placeholder.
        let year_filter = TrackFilter::parse("year:2000");
        assert_eq!(
            db.get_total_track_count(Some(&year_filter)).await.unwrap(),
            105
        );

        // Test window pagination (limit 20, offset 0)
        let page1 = db
            .get_tracks_window(0, 20, None, SortField::Title, SortDirection::Ascending)
            .await
            .expect("Window failed");
        assert_eq!(page1.len(), 20);
        assert_eq!(page1[0].title, "Song 001");
        assert_eq!(page1[19].title, "Song 020");

        // Test second page (limit 20, offset 20)
        let page2 = db
            .get_tracks_window(20, 20, None, SortField::Title, SortDirection::Ascending)
            .await
            .expect("Window failed");
        assert_eq!(page2.len(), 20);
        assert_eq!(page2[0].title, "Song 021");

        // Test last partial page (offset 100, limit 20 -> 5 tracks)
        let page6 = db
            .get_tracks_window(100, 20, None, SortField::Title, SortDirection::Ascending)
            .await
            .expect("Window failed");
        assert_eq!(page6.len(), 5);

        // Test lightweight headers
        let headers = db.get_all_track_headers().await.expect("Headers failed");
        assert_eq!(headers.len(), 105);
        assert_eq!(headers[0].id, 1);

        // Test album aggregation
        let albums = db.get_albums().await.expect("Albums failed");
        assert!(!albums.is_empty());

        // Test artist aggregation
        let artists = db.get_artists().await.expect("Artists failed");
        assert_eq!(artists.len(), 2);
    }

    #[tokio::test]
    async fn test_album_grouping_and_compilations() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        // Compilation album with different track artists, no album_artist set
        let compilation_tracks = vec![
            NewTrack {
                file_path: "/music/comp_01.mp3".to_string(),
                title: "Summer Beat".to_string(),
                artist: "DJ Sol".to_string(),
                album_artist: None,
                album: "Summer Vibes 2024".to_string(),
                duration_ms: 180000,
                track_num: Some(1),
                year: Some(2024),
                ..Default::default()
            },
            NewTrack {
                file_path: "/music/comp_02.mp3".to_string(),
                title: "Ocean Waves".to_string(),
                artist: "Aqua Sound".to_string(),
                album_artist: None,
                album: "Summer Vibes 2024".to_string(),
                duration_ms: 210000,
                track_num: Some(2),
                year: Some(2024),
                ..Default::default()
            },
            NewTrack {
                file_path: "/music/comp_03.mp3".to_string(),
                title: "Sunset Glow".to_string(),
                artist: "Chill Master".to_string(),
                album_artist: None,
                album: "Summer Vibes 2024".to_string(),
                duration_ms: 195000,
                track_num: Some(3),
                year: Some(2024),
                ..Default::default()
            },
        ];

        for t in &compilation_tracks {
            db.insert_or_update_track(t)
                .await
                .expect("Insert track failed");
        }

        // Single-artist album
        let solo_tracks = vec![
            NewTrack {
                file_path: "/music/solo_01.mp3".to_string(),
                title: "Intro".to_string(),
                artist: "Luna".to_string(),
                album_artist: Some("Luna".to_string()),
                album: "Moonlight".to_string(),
                duration_ms: 120000,
                track_num: Some(1),
                year: Some(2022),
                ..Default::default()
            },
            NewTrack {
                file_path: "/music/solo_02.mp3".to_string(),
                title: "Starry Night".to_string(),
                artist: "Luna".to_string(),
                album_artist: Some("Luna".to_string()),
                album: "Moonlight".to_string(),
                duration_ms: 240000,
                track_num: Some(2),
                year: Some(2022),
                ..Default::default()
            },
        ];

        for t in &solo_tracks {
            db.insert_or_update_track(t)
                .await
                .expect("Insert track failed");
        }

        let albums = db.get_albums().await.expect("get_albums failed");
        assert_eq!(albums.len(), 2, "Should have exactly 2 aggregated albums");

        let comp_album = albums
            .iter()
            .find(|a| a.album.as_ref() == "Summer Vibes 2024")
            .unwrap();
        assert_eq!(comp_album.track_count, 3);
        assert_eq!(comp_album.artist.as_ref(), "Various Artists");
        assert_eq!(comp_album.year, Some(2024));
        assert!(comp_album.cover_track_id.is_some());

        let solo_album = albums
            .iter()
            .find(|a| a.album.as_ref() == "Moonlight")
            .unwrap();
        assert_eq!(solo_album.track_count, 2);
        assert_eq!(solo_album.artist.as_ref(), "Luna");
        assert_eq!(solo_album.year, Some(2022));
    }

    #[tokio::test]
    async fn test_extra_metadata_tags_persistence() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        let track = NewTrack {
            file_path: "/music/classical_01.flac".to_string(),
            title: "Symphony No. 5 in C Minor, Op. 67: I. Allegro con brio".to_string(),
            artist: "Vienna Philharmonic".to_string(),
            album_artist: Some("Ludwig van Beethoven".to_string()),
            album: "Beethoven: The Symphonies".to_string(),
            genre: Some("Classical".to_string()),
            duration_ms: 432000,
            track_num: Some(1),
            disc_num: Some(2),
            year: Some(1975),
            composer: Some("Ludwig van Beethoven".to_string()),
            label: Some("Deutsche Grammophon".to_string()),
            musicbrainz_track_id: Some("9b48c4fa-4d64-4e78-9e6b-0b5c1ec824c1".to_string()),
            musicbrainz_artist_id: Some("1f9a2795-2ec3-4412-b250-9d0071582216".to_string()),
            musicbrainz_album_id: Some("d8329ecb-3e58-450f-a4ec-4f123c52a925".to_string()),
        };

        let id = db
            .insert_or_update_track(&track)
            .await
            .expect("Insert track failed");
        let fetched = db
            .get_track_by_id(id)
            .await
            .expect("Query failed")
            .expect("Track not found");

        assert_eq!(fetched.title, track.title);
        assert_eq!(
            fetched.album_artist,
            Some("Ludwig van Beethoven".to_string())
        );
        assert_eq!(fetched.disc_num, Some(2));
        assert_eq!(fetched.year, Some(1975));
        assert_eq!(fetched.composer, Some("Ludwig van Beethoven".to_string()));
        assert_eq!(fetched.label, Some("Deutsche Grammophon".to_string()));
        assert_eq!(
            fetched.musicbrainz_track_id,
            Some("9b48c4fa-4d64-4e78-9e6b-0b5c1ec824c1".to_string())
        );
        assert_eq!(
            fetched.musicbrainz_artist_id,
            Some("1f9a2795-2ec3-4412-b250-9d0071582216".to_string())
        );
        assert_eq!(
            fetched.musicbrainz_album_id,
            Some("d8329ecb-3e58-450f-a4ec-4f123c52a925".to_string())
        );
    }

    #[tokio::test]
    async fn test_library_folders_and_clear_tracks() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        // Initially empty
        let folders = db.get_library_folders().await.expect("get folders failed");
        assert!(folders.is_empty());

        // Add folders
        let added = db
            .add_library_folder("/home/user/Music")
            .await
            .expect("add folder failed");
        assert!(added);
        let added_dup = db
            .add_library_folder("/home/user/Music")
            .await
            .expect("add dup failed");
        assert!(!added_dup); // Ignored due to UNIQUE constraint
        db.add_library_folder("/mnt/external/Audio")
            .await
            .expect("add 2nd folder failed");

        let folders = db.get_library_folders().await.expect("get folders failed");
        assert_eq!(folders.len(), 2);
        assert!(folders.contains(&"/home/user/Music".to_string()));
        assert!(folders.contains(&"/mnt/external/Audio".to_string()));

        // Remove folder
        let removed = db
            .remove_library_folder("/home/user/Music")
            .await
            .expect("remove failed");
        assert!(removed);
        let folders = db.get_library_folders().await.expect("get folders failed");
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0], "/mnt/external/Audio");

        // Insert track and verify clear_all_tracks
        let track = NewTrack {
            file_path: "/mnt/external/Audio/test.mp3".to_string(),
            title: "Test".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            genre: None,
            duration_ms: 60000,
            ..Default::default()
        };
        db.insert_or_update_track(&track)
            .await
            .expect("insert track failed");
        assert_eq!(db.get_total_track_count(None).await.unwrap(), 1);

        let cleared = db.clear_all_tracks().await.expect("clear failed");
        assert_eq!(cleared, 1);
        assert_eq!(db.get_total_track_count(None).await.unwrap(), 0);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_scan_directory_symlink_traversal() {
        let base_temp = std::env::temp_dir().join(format!(
            "melody_test_scan_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let real_dir = base_temp.join("real_music");
        let scan_dir = base_temp.join("library_root");
        let _ = std::fs::create_dir_all(&real_dir);
        let _ = std::fs::create_dir_all(&scan_dir);

        // Create a symlink to real_dir inside scan_dir
        let symlinked_folder = scan_dir.join("symlinked_artists");
        std::os::unix::fs::symlink(&real_dir, &symlinked_folder).expect("symlink creation failed");

        // Put a dummy audio file in real_dir
        let track_path = real_dir.join("song.mp3");
        std::fs::write(&track_path, b"test audio content").expect("write failed");

        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        let summary = db
            .scan_directory(&scan_dir, None::<fn(usize, usize)>)
            .await
            .expect("Scan directory failed");

        // The file inside the symlinked directory was traversed and processed
        assert!(summary.scanned_files >= 1);

        let _ = std::fs::remove_dir_all(&base_temp);
    }

    #[tokio::test]
    async fn test_scan_directories_progress_callback() {
        let base_temp = std::env::temp_dir().join(format!(
            "melody_test_progress_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dir1 = base_temp.join("folder1");
        let dir2 = base_temp.join("folder2");
        let _ = std::fs::create_dir_all(&dir1);
        let _ = std::fs::create_dir_all(&dir2);

        std::fs::write(dir1.join("song1.mp3"), b"audio").unwrap();
        std::fs::write(dir1.join("song2.flac"), b"audio").unwrap();
        std::fs::write(dir2.join("song3.ogg"), b"audio").unwrap();

        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        let progress_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_clone = progress_events.clone();

        let summary = db
            .scan_directories(
                &[&dir1, &dir2],
                Some(move |cur, tot| {
                    progress_clone.lock().unwrap().push((cur, tot));
                }),
            )
            .await
            .expect("scan_directories failed");

        assert_eq!(summary.scanned_files, 3);
        let events = progress_events.lock().unwrap().clone();
        assert!(!events.is_empty());
        // Initial event reports (0, 3)
        assert_eq!(events[0], (0, 3));
        // Final event reports (3, 3)
        assert_eq!(events.last().copied(), Some((3, 3)));

        let _ = std::fs::remove_dir_all(&base_temp);
    }

    #[tokio::test]
    async fn test_playlist_file_references() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        // Initially empty
        assert!(db.get_playlists().await.unwrap().is_empty());

        let id = db
            .upsert_playlist_file("/playlists/Chill.m3u8", "Chill", 3)
            .await
            .expect("upsert failed");
        assert!(id > 0);

        // Upsert same path refreshes count/name rather than duplicating
        db.upsert_playlist_file("/playlists/Chill.m3u8", "Chill Vibes", 4)
            .await
            .expect("re-upsert failed");
        let playlists = db.get_playlists().await.unwrap();
        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].name, "Chill Vibes");
        assert_eq!(playlists[0].track_count, 4);
        assert_eq!(playlists[0].tracks_label, "4 tracks");

        // Rename by id
        assert!(db.rename_playlist_file(id, "Focus").await.unwrap());
        assert_eq!(db.get_playlists().await.unwrap()[0].name, "Focus");

        // Path -> id resolution (only indexed tracks match)
        let track = NewTrack {
            file_path: "/music/a.flac".to_string(),
            title: "A".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 1000,
            ..Default::default()
        };
        db.insert_or_update_track(&track).await.unwrap();

        let map = db
            .get_track_ids_by_paths(&[
                "/music/a.flac".to_string(),
                "/music/missing.flac".to_string(),
            ])
            .await
            .unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("/music/a.flac"));

        // Remove reference
        assert!(db.remove_playlist_file(id).await.unwrap());
        assert!(db.get_playlists().await.unwrap().is_empty());

        // Removing a track does not touch playlist references
        db.upsert_playlist_file("/playlists/Keep.m3u8", "Keep", 1)
            .await
            .unwrap();
        db.clear_all_tracks().await.unwrap();
        assert_eq!(db.get_playlists().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_playback_queue_persistence() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        let mut ids = Vec::new();
        for i in 1..=3 {
            let track = NewTrack {
                file_path: format!("/music/queue_{}.flac", i),
                title: format!("Queue Song {}", i),
                artist: "Artist".to_string(),
                album: "Album".to_string(),
                duration_ms: 1000 * i as u64,
                ..Default::default()
            };
            ids.push(db.insert_or_update_track(&track).await.unwrap());
        }

        let record = QueueRecord {
            id: 1,
            name: "My Queue".to_string(),
            current: 0,
            source_kind: "manual".to_string(),
            source_value: None,
            source_playlist: None,
            track_count: 0,
        };
        db.upsert_queue(&record).await.unwrap();
        db.replace_queue_tracks(1, &ids).await.unwrap();
        db.set_active_queue_id(Some(1)).await.unwrap();

        let queues = db.list_queues().await.unwrap();
        assert_eq!(queues.len(), 1);
        assert_eq!(queues[0].name, "My Queue");
        assert_eq!(queues[0].track_count, 3);

        let tracks = db.load_queue_tracks(1).await.unwrap();
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].id, ids[0]);
        assert_eq!(tracks[2].id, ids[2]);

        assert_eq!(db.get_active_queue_id().await.unwrap(), Some(1));

        // Cursor updates persist.
        db.set_queue_current(1, 2).await.unwrap();
        let queues = db.list_queues().await.unwrap();
        assert_eq!(queues[0].current, 2);

        // Replacing membership rewrites ordering.
        db.replace_queue_tracks(1, &[ids[2], ids[1], ids[0]])
            .await
            .unwrap();
        let tracks = db.load_queue_tracks(1).await.unwrap();
        assert_eq!(tracks[0].id, ids[2]);

        // Large memberships span multiple insert chunks without losing order.
        let big: Vec<i64> = (0..650).map(|i| ids[i % ids.len()]).collect();
        db.replace_queue_tracks(1, &big).await.unwrap();
        let tracks = db.load_queue_tracks(1).await.unwrap();
        assert_eq!(tracks.len(), big.len());
        for (pos, track) in tracks.iter().enumerate() {
            assert_eq!(track.id, big[pos], "order preserved at position {pos}");
        }

        // Deletion clears the queue and can clear the active selection.
        db.delete_queue(1).await.unwrap();
        db.set_active_queue_id(None).await.unwrap();
        assert!(db.list_queues().await.unwrap().is_empty());
        assert_eq!(db.get_active_queue_id().await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_insert_track_with_non_utf8_and_null_bytes() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("Failed to create in-memory db");

        // Construct a title with CP1252 / invalid UTF-8 byte (0x92 = right single quote in CP1252) and null byte
        let raw_bytes = b"the 8 o\x92clock\x00 news is going crazy";
        let sanitized = metadata::sanitize_tag_bytes(raw_bytes);
        assert_eq!(sanitized, "the 8 o’clock news is going crazy");

        let track = NewTrack {
            file_path: "/music/test_track\x00.wav".to_string(),
            title: sanitized,
            artist: "KRAL".to_string(),
            album: "全部大コンピ, Vol.05".to_string(),
            duration_ms: 180000,
            ..Default::default()
        };

        // Both single insert and batch insert must succeed without "TEXT value contains invalid UTF-8"
        let id = db
            .insert_or_update_track(&track)
            .await
            .expect("Single insert failed");
        assert!(id > 0);

        let batch = vec![track.clone()];
        let inserted = db
            .insert_tracks_batch(&batch)
            .await
            .expect("Batch insert failed");
        assert_eq!(inserted, 1);
    }

    #[tokio::test]
    async fn test_filter_new_paths() {
        let db = LibraryDatabase::in_memory()
            .await
            .expect("in-memory db should initialize");

        let p1 = "/music/existing1.flac";
        let p2 = "/music/existing2.mp3";
        let p3 = "/music/brand_new.opus";

        let track1 = NewTrack {
            file_path: p1.to_string(),
            title: "Track 1".to_string(),
            artist: "Artist".to_string(),
            album_artist: None,
            album: "Album".to_string(),
            genre: None,
            duration_ms: 180_000,
            track_num: Some(1),
            disc_num: Some(1),
            year: Some(2025),
            composer: None,
            label: None,
            musicbrainz_track_id: None,
            musicbrainz_artist_id: None,
            musicbrainz_album_id: None,
        };

        let track2 = NewTrack {
            file_path: p2.to_string(),
            title: "Track 2".to_string(),
            artist: "Artist".to_string(),
            album_artist: None,
            album: "Album".to_string(),
            genre: None,
            duration_ms: 200_000,
            track_num: Some(2),
            disc_num: Some(1),
            year: Some(2025),
            composer: None,
            label: None,
            musicbrainz_track_id: None,
            musicbrainz_artist_id: None,
            musicbrainz_album_id: None,
        };

        db.insert_tracks_batch(&[track1, track2])
            .await
            .expect("batch insert");

        let test_paths = vec![PathBuf::from(p1), PathBuf::from(p2), PathBuf::from(p3)];

        let new_only = db
            .filter_new_paths(&test_paths)
            .await
            .expect("filter_new_paths");
        assert_eq!(new_only.len(), 1);
        assert_eq!(new_only[0], PathBuf::from(p3));
    }

    #[tokio::test]
    async fn test_scan_directories_new_skips_existing_files() {
        let base_temp = std::env::temp_dir().join(format!(
            "melody_test_scan_new_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&base_temp);

        let song1 = base_temp.join("song1.mp3");
        let song2 = base_temp.join("song2.mp3");
        std::fs::write(&song1, b"audio 1").unwrap();

        let db = LibraryDatabase::in_memory().await.unwrap();

        let track1 = NewTrack {
            file_path: song1.to_string_lossy().to_string(),
            title: "Song 1".to_string(),
            artist: "Artist".to_string(),
            album_artist: None,
            album: "Album".to_string(),
            genre: None,
            duration_ms: 120_000,
            track_num: Some(1),
            disc_num: Some(1),
            year: Some(2025),
            composer: None,
            label: None,
            musicbrainz_track_id: None,
            musicbrainz_artist_id: None,
            musicbrainz_album_id: None,
        };
        db.insert_tracks_batch(&[track1]).await.unwrap();

        // First scan: song1 is already in DB, so 0 new files are scanned
        let summary1 = db
            .scan_directories_new(&[&base_temp], None::<fn(usize, usize)>)
            .await
            .unwrap();
        assert_eq!(summary1.scanned_files, 0);
        assert_eq!(summary1.inserted_tracks, 0);

        // Add song2 and scan again
        std::fs::write(&song2, b"audio 2").unwrap();
        let summary2 = db
            .scan_directories_new(&[&base_temp], None::<fn(usize, usize)>)
            .await
            .unwrap();
        // Only the new song2 is scanned!
        assert_eq!(summary2.scanned_files, 1);

        let _ = std::fs::remove_dir_all(&base_temp);
    }
}
