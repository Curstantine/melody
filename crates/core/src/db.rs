use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;
use tracing::{info, warn};
use turso::{params, Connection, Database, Error as TursoError, Row};
use walkdir::WalkDir;

use crate::metadata::extract_metadata;
use crate::model::{
    AlbumSummary, ArtistSummary, NewTrack, SortDirection, SortField, Track, TrackIdHeader,
};

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
            )
            .await?;

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
                ORDER BY {} {} NULLS LAST
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
                MIN(file_path) as cover_track_path,
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
            let cover_track_path: Option<String> = row.get(5)?;
            let added_timestamp: i64 = row.get(6)?;
            albums.push(AlbumSummary {
                album,
                artist,
                track_count: count as usize,
                total_duration_ms: duration as u64,
                year,
                cover_track_path,
                added_timestamp,
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
                artist,
                track_count: track_count as usize,
                album_count: album_count as usize,
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

    /// Wipes all indexed tracks and playlist associations from the database
    pub async fn clear_all_tracks(&self) -> Result<u64, DbError> {
        let count = self.conn.execute("DELETE FROM tracks", ()).await?;
        let _ = self.conn.execute("DELETE FROM playlist_tracks", ()).await;
        let _ = self.conn.execute("VACUUM", ()).await;
        Ok(count)
    }

    /// Recursively scan directory and index all supported audio files into libSQL
    pub async fn scan_directory<P: AsRef<Path>, F: Fn(usize, usize)>(
        &self,
        dir_path: P,
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut summary = ScanSummary::default();
        let supported_exts = [
            "mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "alac", "opus", "aiff",
        ];

        let mut batch = Vec::with_capacity(100);

        for entry in WalkDir::new(dir_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
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
                                    match self.insert_tracks_batch(&batch).await {
                                        Ok(inserted) => {
                                            summary.inserted_tracks += inserted;
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Batch insertion failed for {} tracks: {}. Falling back to individual inserts...",
                                                batch.len(),
                                                e
                                            );
                                            for t in &batch {
                                                match self.insert_or_update_track(t).await {
                                                    Ok(_) => summary.inserted_tracks += 1,
                                                    Err(err) => {
                                                        warn!(
                                                            "Failed to insert track {:?}: {}",
                                                            t.file_path, err
                                                        );
                                                        summary.failed_files += 1;
                                                    }
                                                }
                                            }
                                        }
                                    }
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
            match self.insert_tracks_batch(&batch).await {
                Ok(inserted) => {
                    summary.inserted_tracks += inserted;
                }
                Err(e) => {
                    warn!(
                        "Final batch insertion failed for {} tracks: {}. Falling back to individual inserts...",
                        batch.len(),
                        e
                    );
                    for t in &batch {
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

fn sanitize_db_text(s: &str) -> String {
    let bytes = s.as_bytes();
    let filtered: Vec<u8> = bytes.iter().copied().filter(|&b| b != 0).collect();
    if let Ok(v) = std::str::from_utf8(&filtered) {
        v.to_string()
    } else {
        String::from_utf8_lossy(&filtered).to_string()
    }
}
