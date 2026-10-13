use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Connection, Database, Error as TursoError, Row};
use walkdir::WalkDir;

use crate::intern;
use crate::metadata::extract_metadata;
use crate::model::{
    format_duration, AlbumSummary, ArtistSummary, NewTrack, PlaylistSummary, SortDirection,
    SortField, Track, TrackIdHeader, TrackRow,
};
use crate::paths;

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

    pub async fn init_schema(&self) -> Result<(), DbError> {
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

        info!("Database schema initialized successfully");
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
                    track.album_artist.as_deref().map(sanitize_db_text),
                    sanitize_db_text(&track.album),
                    track.genre.as_deref().map(sanitize_db_text),
                    track.duration_ms as i64,
                    track.track_num.map(|n| n as i64),
                    track.disc_num.map(|n| n as i64),
                    track.year.map(|n| n as i64),
                    track.composer.as_deref().map(sanitize_db_text),
                    track.label.as_deref().map(sanitize_db_text),
                    track.musicbrainz_track_id.as_deref().map(sanitize_db_text),
                    track.musicbrainz_artist_id.as_deref().map(sanitize_db_text),
                    track.musicbrainz_album_id.as_deref().map(sanitize_db_text),
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
                    track.album_artist.as_deref().map(sanitize_db_text),
                    sanitize_db_text(&track.album),
                    track.genre.as_deref().map(sanitize_db_text),
                    track.duration_ms as i64,
                    track.track_num.map(|n| n as i64),
                    track.disc_num.map(|n| n as i64),
                    track.year.map(|n| n as i64),
                    track.composer.as_deref().map(sanitize_db_text),
                    track.label.as_deref().map(sanitize_db_text),
                    track.musicbrainz_track_id.as_deref().map(sanitize_db_text),
                    track.musicbrainz_artist_id.as_deref().map(sanitize_db_text),
                    track.musicbrainz_album_id.as_deref().map(sanitize_db_text),
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

    pub async fn get_next_track(&self, current_id: i64) -> Result<Option<Track>, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks WHERE id > ?1 ORDER BY id ASC LIMIT 1
            "#,
                params![current_id],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            return Ok(Some(map_track_row(&row)?));
        }

        // Wrap around to first track
        let mut rows_first = self
            .conn
            .query(
                r#"
            SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                   track_num, disc_num, year, composer, label,
                   musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
            FROM tracks ORDER BY id ASC LIMIT 1
            "#,
                (),
            )
            .await?;

        if let Some(row) = rows_first.next().await? {
            Ok(Some(map_track_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn get_total_track_count(&self, filter: Option<&str>) -> Result<usize, DbError> {
        let mut rows = match filter {
            Some(f) if !f.trim().is_empty() => {
                let pattern = format!("%{}%", f.trim());
                self.conn
                    .query(
                        "SELECT COUNT(*) FROM tracks WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1",
                        params![pattern],
                    )
                    .await?
            }
            _ => self.conn.query("SELECT COUNT(*) FROM tracks", ()).await?,
        };

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
        filter: Option<&str>,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Result<Vec<Track>, DbError> {
        let sort_col = match sort_field {
            SortField::Title => "title",
            SortField::Artist => "artist",
            SortField::Album => "album",
            SortField::Duration => "duration_ms",
            SortField::TrackNum => "track_num NULLS LAST",
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
                ORDER BY {} {}
                LIMIT ?2 OFFSET ?3
                "#,
                sort_col, dir_str
            );

            let mut rows = self
                .conn
                .query(&query, params![pattern, limit as i64, offset as i64])
                .await?;

            while let Some(row) = rows.next().await? {
                tracks.push(map_track_row(&row)?);
            }
        } else {
            let query = format!(
                r#"
                SELECT id, file_path, title, artist, album_artist, album, genre, duration_ms,
                       track_num, disc_num, year, composer, label,
                       musicbrainz_track_id, musicbrainz_artist_id, musicbrainz_album_id, added_timestamp
                FROM tracks
                ORDER BY {} {}
                LIMIT ?1 OFFSET ?2
                "#,
                sort_col, dir_str
            );

            let mut rows = self
                .conn
                .query(&query, params![limit as i64, offset as i64])
                .await?;

            while let Some(row) = rows.next().await? {
                tracks.push(map_track_row(&row)?);
            }
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
        filter: Option<&str>,
        sort_field: SortField,
        sort_dir: SortDirection,
    ) -> Result<Vec<TrackRow>, DbError> {
        let sort_col = match sort_field {
            SortField::Title => "title",
            SortField::Artist => "artist",
            SortField::Album => "album",
            SortField::Duration => "duration_ms",
            SortField::TrackNum => "track_num NULLS LAST",
            SortField::AddedDate => "added_timestamp",
        };

        let dir_str = match sort_dir {
            SortDirection::Ascending => "ASC",
            SortDirection::Descending => "DESC",
        };

        let mut rows_out = Vec::with_capacity(limit);

        if let Some(f) = filter.filter(|s| !s.trim().is_empty()) {
            let pattern = format!("%{}%", f.trim());
            let query = format!(
                r#"
                SELECT id, title, artist, album, duration_ms, file_path
                FROM tracks
                WHERE title LIKE ?1 OR artist LIKE ?1 OR album LIKE ?1
                ORDER BY {} {}
                LIMIT ?2 OFFSET ?3
                "#,
                sort_col, dir_str
            );

            let mut rows = self
                .conn
                .query(&query, params![pattern, limit as i64, offset as i64])
                .await?;

            while let Some(row) = rows.next().await? {
                rows_out.push(map_track_row_summary(&row)?);
            }
        } else {
            let query = format!(
                r#"
                SELECT id, title, artist, album, duration_ms, file_path
                FROM tracks
                ORDER BY {} {}
                LIMIT ?1 OFFSET ?2
                "#,
                sort_col, dir_str
            );

            let mut rows = self
                .conn
                .query(&query, params![limit as i64, offset as i64])
                .await?;

            while let Some(row) = rows.next().await? {
                rows_out.push(map_track_row_summary(&row)?);
            }
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

        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT id, file_path FROM tracks WHERE id IN ({})",
            placeholders
        );

        let mut rows = self
            .conn
            .query(&query, turso::params_from_iter(ids.iter().copied()))
            .await?;

        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0)?;
            let path: String = row.get(1)?;
            out.insert(id, path);
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

        const CHUNK: usize = 500;
        for chunk in ids.chunks(CHUNK) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT id, title, artist, album, duration_ms, file_path FROM tracks WHERE id IN ({})",
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

    /// Fetches only the filesystem path for a track id (used to resolve album
    /// cover art for the visible catalog window).
    pub async fn get_track_path_by_id(&self, id: i64) -> Result<Option<String>, DbError> {
        let mut rows = self
            .conn
            .query("SELECT file_path FROM tracks WHERE id = ?1", params![id])
            .await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
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

            let managed = Path::new(&file_path).starts_with(&managed_dir);
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

        // Chunk to stay clear of SQLite's variable limit for very large playlists.
        const CHUNK: usize = 500;
        for chunk in paths.chunks(CHUNK) {
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

    /// Recursively scan directory and index all supported audio files.
    ///
    /// The directory walk is streamed lazily (so peak memory stays proportional
    /// to a single batch rather than the whole library), while the blocking
    /// per-file FFmpeg metadata probe runs on `spawn_blocking` workers so the
    /// async runtime driving the UI is never stalled.
    pub async fn scan_directory<P: AsRef<Path>, F: Fn(usize, usize)>(
        &self,
        dir_path: P,
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut summary = ScanSummary::default();
        let supported_exts = [
            "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "alac", "opus", "aiff",
        ];

        let mut batch: Vec<NewTrack> = Vec::with_capacity(100);

        for entry in WalkDir::new(dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            let ext_lower = ext.to_lowercase();
            if !supported_exts.contains(&ext_lower.as_str()) {
                continue;
            }

            summary.scanned_files += 1;

            let parse_path = path.to_path_buf();
            let parsed = tokio::task::spawn_blocking(move || extract_metadata(&parse_path)).await;

            match parsed {
                Ok(Ok(track)) => {
                    batch.push(track);
                    if batch.len() >= 100 {
                        self.flush_scan_batch(&batch, &mut summary).await;
                        batch.clear();
                        if let Some(ref cb) = progress_cb {
                            cb(summary.scanned_files, summary.inserted_tracks);
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("Failed to extract metadata for {:?}: {}", path, e);
                    summary.failed_files += 1;
                }
                Err(e) => {
                    warn!("Metadata task failed for {:?}: {}", path, e);
                    summary.failed_files += 1;
                }
            }
        }

        if !batch.is_empty() {
            self.flush_scan_batch(&batch, &mut summary).await;
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
    Ok(TrackRow {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: intern::intern(&artist),
        album: intern::intern(&album),
        duration_ms: duration as u64,
        file_path: row.get(5)?,
        duration_label: format_duration(duration as u64),
    })
}

fn sanitize_db_text(s: &str) -> String {
    let bytes = s.as_bytes();
    if !bytes.contains(&0) {
        return s.to_string();
    }
    let filtered: Vec<u8> = bytes.iter().copied().filter(|&b| b != 0).collect();
    if let Ok(v) = std::str::from_utf8(&filtered) {
        v.to_string()
    } else {
        String::from_utf8_lossy(&filtered).to_string()
    }
}
