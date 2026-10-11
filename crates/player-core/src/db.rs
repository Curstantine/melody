use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::metadata::extract_metadata;
use crate::model::{
    AlbumSummary, ArtistSummary, NewTrack, SortDirection, SortField, Track, TrackIdHeader,
};

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Lock error: {0}")]
    Lock(String),
}

#[derive(Debug, Clone)]
pub struct LibraryDatabase {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanSummary {
    pub scanned_files: usize,
    pub inserted_tracks: usize,
    pub updated_tracks: usize,
    pub failed_files: usize,
}

impl LibraryDatabase {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, DbError> {
        let conn = Connection::open(db_path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn init_schema(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;

        // Enable WAL mode and synchronous=NORMAL for optimal performance & concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        conn.execute_batch(
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

            CREATE TABLE IF NOT EXISTS playlists (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                created_timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS playlist_tracks (
                playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
                track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
                position INTEGER NOT NULL,
                PRIMARY KEY (playlist_id, track_id)
            );

            CREATE TABLE IF NOT EXISTS library_folders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                added_timestamp INTEGER NOT NULL
            );
            "#,
        )?;

        // Safe migrations for databases created before schema additions
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN album_artist TEXT", []);
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN disc_num INTEGER", []);
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN year INTEGER", []);
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN composer TEXT", []);
        let _ = conn.execute("ALTER TABLE tracks ADD COLUMN label TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE tracks ADD COLUMN musicbrainz_track_id TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE tracks ADD COLUMN musicbrainz_artist_id TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE tracks ADD COLUMN musicbrainz_album_id TEXT",
            [],
        );

        info!("Database schema initialized successfully");
        Ok(())
    }

    pub fn insert_or_update_track(&self, track: &NewTrack) -> Result<i64, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        conn.execute(
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
                track.file_path,
                track.title,
                track.artist,
                track.album_artist,
                track.album,
                track.genre,
                track.duration_ms,
                track.track_num,
                track.disc_num,
                track.year,
                track.composer,
                track.label,
                track.musicbrainz_track_id,
                track.musicbrainz_artist_id,
                track.musicbrainz_album_id,
                now,
            ],
        )?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    pub fn insert_tracks_batch(&self, tracks: &[NewTrack]) -> Result<usize, DbError> {
        if tracks.is_empty() {
            return Ok(0);
        }

        let mut conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let tx = conn.transaction()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut count = 0;
        {
            let mut stmt = tx.prepare_cached(
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
            )?;

            for track in tracks {
                stmt.execute(params![
                    track.file_path,
                    track.title,
                    track.artist,
                    track.album_artist,
                    track.album,
                    track.genre,
                    track.duration_ms,
                    track.track_num,
                    track.disc_num,
                    track.year,
                    track.composer,
                    track.label,
                    track.musicbrainz_track_id,
                    track.musicbrainz_artist_id,
                    track.musicbrainz_album_id,
                    now,
                ])?;
                count += 1;
            }
        }

        tx.commit()?;
        Ok(count)
    }
}

fn map_track_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: row.get(0)?,
        file_path: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        album_artist: row.get(4)?,
        album: row.get(5)?,
        genre: row.get(6)?,
        duration_ms: row.get(7)?,
        track_num: row.get(8)?,
        disc_num: row.get(9)?,
        year: row.get(10)?,
        composer: row.get(11)?,
        label: row.get(12)?,
        musicbrainz_track_id: row.get(13)?,
        musicbrainz_artist_id: row.get(14)?,
        musicbrainz_album_id: row.get(15)?,
        added_timestamp: row.get(16)?,
    })
}

impl LibraryDatabase {
    pub fn get_track_by_id(&self, id: i64) -> Result<Option<Track>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks WHERE id = ?1
            "#,
        )?;

        let track = stmt.query_row(params![id], map_track_row).optional()?;

        Ok(track)
    }

    pub fn get_track_by_path(&self, file_path: &str) -> Result<Option<Track>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks WHERE file_path = ?1
            "#,
        )?;

        let track = stmt
            .query_row(params![file_path], map_track_row)
            .optional()?;

        Ok(track)
    }

    pub fn get_next_track(&self, current_id: i64) -> Result<Option<Track>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks WHERE id > ?1 ORDER BY id ASC LIMIT 1
            "#,
        )?;

        let track = stmt.query_row(params![current_id], map_track_row).optional()?;
        if track.is_some() {
            return Ok(track);
        }

        // Wrap around to first track
        let mut stmt_first = conn.prepare_cached(
            r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks ORDER BY id ASC LIMIT 1
            "#,
        )?;

        let first_track = stmt_first.query_row([], map_track_row).optional()?;
        Ok(first_track)
    }

    pub fn get_total_track_count(&self, filter: Option<&str>) -> Result<usize, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let count: i64 = match filter {
            Some(f) if !f.trim().is_empty() => {
                let pattern = format!("%{}%", f.trim());
                conn.query_row(
                    "SELECT COUNT(*) FROM tracks WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1",
                    params![pattern],
                    |row| row.get(0),
                )?
            }
            _ => conn.query_row("SELECT COUNT(*) FROM tracks", [], |row| row.get(0))?,
        };

        Ok(count as usize)
    }

    /// Read queries with LIMIT and OFFSET based on the active viewport
    /// Never pull all tracks into a `Vec<Track>`
    pub fn get_tracks_window(
        &self,
        offset: usize,
        limit: usize,
        filter: Option<&str>,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Result<Vec<Track>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;

        let sort_col = match sort_field {
            SortField::Title => "title",
            SortField::Artist => "artist",
            SortField::Album => "album",
            SortField::Duration => "duration_ms",
            SortField::TrackNum => "track_num",
            SortField::AddedDate => "added_timestamp",
        };

        let dir_str = match sort_dir {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };

        let mut tracks = Vec::with_capacity(limit);

        if let Some(f) = filter.filter(|s| !s.trim().is_empty()) {
            let pattern = format!("%{}%", f.trim());
            let query = format!(
                r#"
                SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                       track_num, disc_num, year, composer, label,
                       musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
                FROM tracks
                WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1
                ORDER BY {} {} NULLS LAST
                LIMIT ?2 OFFSET ?3
                "#,
                sort_col, dir_str
            );

            let mut stmt = conn.prepare(&query)?;
            let rows =
                stmt.query_map(params![pattern, limit as i64, offset as i64], map_track_row)?;

            for row in rows {
                tracks.push(row?);
            }
        } else {
            let query = format!(
                r#"
                SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                       track_num, disc_num, year, composer, label,
                       musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
                FROM tracks
                ORDER BY {} {} NULLS LAST
                LIMIT ?1 OFFSET ?2
                "#,
                sort_col, dir_str
            );

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map(params![limit as i64, offset as i64], map_track_row)?;

            for row in rows {
                tracks.push(row?);
            }
        }

        Ok(tracks)
    }

    /// Lightweight in-memory index struct for global shuffle, playlist generation, or totals
    pub fn get_all_track_headers(&self) -> Result<Vec<TrackIdHeader>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached("SELECT id, duration_ms FROM tracks ORDER BY id ASC")?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let dur: i64 = row.get(1)?;
            Ok(TrackIdHeader {
                id,
                duration_ms: dur as u32,
            })
        })?;

        let mut headers = Vec::new();
        for r in rows {
            headers.push(r?);
        }
        Ok(headers)
    }

    pub fn get_albums(&self) -> Result<Vec<AlbumSummary>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached(
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
                MIN(file_path) as cover_track_path,
                MAX(added_timestamp) as added_timestamp
            FROM tracks
            GROUP BY album, COALESCE(NULLIF(album_artist, ''), '')
            ORDER BY album ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let album: String = row.get(0)?;
            let artist: String = row.get(1)?;
            let count: i64 = row.get(2)?;
            let duration: i64 = row.get(3)?;
            let year: Option<u32> = row.get(4)?;
            let cover_track_path: Option<String> = row.get(5)?;
            let added_timestamp: i64 = row.get(6)?;
            Ok(AlbumSummary {
                album,
                artist,
                track_count: count as usize,
                total_duration_ms: duration as u64,
                year,
                cover_track_path,
                added_timestamp,
            })
        })?;

        let mut albums = Vec::new();
        for r in rows {
            albums.push(r?);
        }
        Ok(albums)
    }

    pub fn get_artists(&self) -> Result<Vec<ArtistSummary>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached(
            r#"
            SELECT artist, COUNT(*) as track_count, COUNT(DISTINCT album) as album_count
            FROM tracks
            GROUP BY artist
            ORDER BY artist ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let artist: String = row.get(0)?;
            let track_count: i64 = row.get(1)?;
            let album_count: i64 = row.get(2)?;
            Ok(ArtistSummary {
                artist,
                track_count: track_count as usize,
                album_count: album_count as usize,
            })
        })?;

        let mut artists = Vec::new();
        for r in rows {
            artists.push(r?);
        }
        Ok(artists)
    }

    pub fn delete_track(&self, id: i64) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let affected = conn.execute("DELETE FROM tracks WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    /// Retrieves all configured music library scan paths
    pub fn get_library_folders(&self) -> Result<Vec<String>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT path FROM library_folders ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut folders = Vec::new();
        for r in rows {
            folders.push(r?);
        }
        Ok(folders)
    }

    /// Adds a directory path to the library folders list if not already present
    pub fn add_library_folder(&self, path: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let affected = conn.execute(
            "INSERT OR IGNORE INTO library_folders (path, added_timestamp) VALUES (?1, ?2)",
            params![path, now],
        )?;
        Ok(affected > 0)
    }

    /// Removes a directory path from the library folders list
    pub fn remove_library_folder(&self, path: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let affected =
            conn.execute("DELETE FROM library_folders WHERE path = ?1", params![path])?;
        Ok(affected > 0)
    }

    /// Wipes all indexed tracks and playlist associations from the database
    pub fn clear_all_tracks(&self) -> Result<usize, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Lock(e.to_string()))?;
        let count = conn.execute("DELETE FROM tracks", [])?;
        let _ = conn.execute("DELETE FROM playlist_tracks", []);
        let _ = conn.execute("VACUUM", []);
        Ok(count)
    }

    /// Recursively scan directory and index all supported audio files into SQLite
    pub fn scan_directory<P: AsRef<Path>, F: Fn(usize, usize)>(
        &self,
        dir_path: P,
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut summary = ScanSummary::default();
        let supported_exts = [
            "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "alac", "opus", "aiff",
        ];

        let mut batch = Vec::with_capacity(100);

        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if supported_exts.contains(&ext_lower.as_str()) {
                        summary.scanned_files += 1;
                        match extract_metadata(path) {
                            Ok(track) => {
                                batch.push(track);
                                if batch.len() >= 100 {
                                    let inserted = self.insert_tracks_batch(&batch)?;
                                    summary.inserted_tracks += inserted;
                                    batch.clear();
                                    if let Some(ref cb) = progress_cb {
                                        cb(summary.scanned_files, summary.inserted_tracks);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to extract metadata for {:?}: {}", path, e);
                                summary.failed_files += 1;
                            }
                        }
                    }
                }
            }
        }

        if !batch.is_empty() {
            let inserted = self.insert_tracks_batch(&batch)?;
            summary.inserted_tracks += inserted;
            if let Some(ref cb) = progress_cb {
                cb(summary.scanned_files, summary.inserted_tracks);
            }
        }

        info!(
            "Directory scan finished: {} files scanned, {} tracks indexed, {} errors",
            summary.scanned_files, summary.inserted_tracks, summary.failed_files
        );

        Ok(summary)
    }
}
