use std::path::Path;
use std::sync::Arc;

use tracing::info;

use crate::{DbError, LibraryDatabase};

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

    const SCHEMA_VERSION: i32 = 1;

    async fn init_schema(&self) -> Result<(), DbError> {
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
}
