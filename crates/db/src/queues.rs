use model::{QueueRecord, TrackRow};
use turso::params;

use crate::helpers::{map_track_row_summary, now_secs, sanitize_db_opt, sanitize_db_text};
use crate::{DbError, LibraryDatabase, QUEUE_TRACKS_INSERT_CHUNK_ROWS};

impl LibraryDatabase {
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

    /// Removes every queue and its tracks except `keep_queue_id`.
    pub async fn delete_other_queues(&self, keep_queue_id: i64) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction().await?;
        tx.execute(
            "DELETE FROM playback_queue_tracks WHERE queue_id != ?1",
            params![keep_queue_id],
        )
        .await?;
        tx.execute(
            "DELETE FROM playback_queues WHERE id != ?1",
            params![keep_queue_id],
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
