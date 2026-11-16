use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::warn;
use turso::params;

use model::PlaylistSummary;

use crate::helpers::sanitize_db_text;
use crate::{DbError, LibraryDatabase, SQL_IN_CLAUSE_CHUNK_SIZE};
use playlist::{parse_m3u, PlaylistItem, PlaylistResolution};

impl LibraryDatabase {
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

        let managed_dir = path::playlists_dir();
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

    /// Parses an M3U file and matches its entries against indexed library tracks.
    pub async fn resolve_playlist(&self, file_path: &str) -> PlaylistResolution {
        let parsed = match parse_m3u(file_path) {
            Ok(parsed) => parsed,
            Err(e) => {
                warn!("Failed to parse playlist {}: {}", file_path, e);
                return PlaylistResolution {
                    items: Vec::new(),
                    entries: Vec::new(),
                    tracks: Vec::new(),
                };
            }
        };

        let path_strings: Vec<String> = parsed
            .entries
            .iter()
            .map(|e| e.resolved_path.to_string_lossy().to_string())
            .collect();
        let id_map = self
            .get_track_ids_by_paths(&path_strings)
            .await
            .unwrap_or_default();
        let ids: Vec<i64> = path_strings
            .iter()
            .filter_map(|p| id_map.get(p).copied())
            .collect();
        let rows = self.get_track_rows_by_ids(&ids).await.unwrap_or_default();

        let mut items = Vec::with_capacity(parsed.entries.len());
        let mut tracks = Vec::new();
        for entry in &parsed.entries {
            let resolved = entry.resolved_path.to_string_lossy().to_string();
            if let Some(id) = id_map.get(&resolved).copied() {
                if let Some(row) = rows.get(&id) {
                    items.push(PlaylistItem {
                        title: row.title.clone(),
                        artist: row.display_artist().to_string(),
                        duration_label: row.duration_label.clone(),
                        track_id: Some(id),
                    });
                    tracks.push(row.clone());
                    continue;
                }
            }

            let title = entry
                .title
                .clone()
                .or_else(|| {
                    entry
                        .resolved_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| entry.raw_path.clone());
            items.push(PlaylistItem {
                title,
                artist: "Not in library".to_string(),
                duration_label: entry
                    .duration_ms
                    .map(model::format_duration)
                    .unwrap_or_default(),
                track_id: None,
            });
        }

        PlaylistResolution {
            items,
            entries: parsed.entries,
            tracks,
        }
    }
}
