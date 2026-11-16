use model::{format_duration, intern, AlbumSummary, ArtistSummary};

use crate::{DbError, LibraryCounts, LibraryDatabase};

impl LibraryDatabase {
    /// Returns unfiltered catalog totals (`tracks`, `albums`, `artists`) directly
    /// from SQLite so metrics stay accurate even when catalog tabs are unloaded
    /// or a library search filter is active.
    pub async fn get_library_counts(&self) -> Result<LibraryCounts, DbError> {
        let mut rows = self
            .conn
            .query(
                r#"
                SELECT
                    (SELECT COUNT(*) FROM tracks),
                    (SELECT COUNT(*) FROM (SELECT 1 FROM tracks GROUP BY album, COALESCE(NULLIF(album_artist, ''), ''))),
                    (SELECT COUNT(DISTINCT artist) FROM tracks)
                "#,
                (),
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let tracks: i64 = row.get(0)?;
            let albums: i64 = row.get(1)?;
            let artists: i64 = row.get(2)?;
            Ok(LibraryCounts {
                tracks: tracks.max(0) as usize,
                albums: albums.max(0) as usize,
                artists: artists.max(0) as usize,
            })
        } else {
            Ok(LibraryCounts::default())
        }
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
}
