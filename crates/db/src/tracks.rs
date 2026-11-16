use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use turso::params;

use model::{intern, NewTrack, SortDirection, SortField, Track, TrackFilter, TrackRow};

use crate::helpers::{
    filter_clause, map_track_row, map_track_row_summary, sanitize_db_opt, sanitize_db_text,
    sort_column,
};
use crate::{DbError, LibraryDatabase, SQL_IN_CLAUSE_CHUNK_SIZE};

impl LibraryDatabase {
    pub(crate) async fn insert_or_update_track(&self, track: &NewTrack) -> Result<i64, DbError> {
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

    pub(crate) async fn insert_tracks_batch(&self, tracks: &[NewTrack]) -> Result<usize, DbError> {
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
}
