use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};

use turso::Row;

use model::{format_duration_stack, intern, SortField, Track, TrackFilter, TrackPath, TrackRow};

use crate::DbError;

/// Current Unix timestamp in whole seconds.
pub(crate) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Maps a sort field to its SQL ordering column expression.
pub(crate) fn sort_column(sort_field: SortField) -> &'static str {
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
pub(crate) fn filter_clause(filter: Option<&TrackFilter>, binds: &mut Vec<turso::Value>) -> String {
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
    if let Some(value) = &filter.composer {
        clauses.push("composer = ?".to_string());
        binds.push(turso::Value::from(value.clone()));
    }
    if let Some(value) = &filter.label {
        clauses.push("label = ?".to_string());
        binds.push(turso::Value::from(value.clone()));
    }
    if let Some(track_num) = filter.track_num {
        clauses.push("track_num = CAST(? AS INTEGER)".to_string());
        binds.push(turso::Value::from(track_num.to_string()));
    }
    if let Some(disc_num) = filter.disc_num {
        clauses.push("disc_num = CAST(? AS INTEGER)".to_string());
        binds.push(turso::Value::from(disc_num.to_string()));
    }

    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

pub(crate) fn map_track_row(row: &Row) -> Result<Track, DbError> {
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

pub(crate) fn map_track_row_summary(row: &Row) -> Result<TrackRow, DbError> {
    let artist: String = row.get(2)?;
    let album: String = row.get(3)?;
    let duration: i64 = row.get(4)?;
    let file_path: String = row.get(5)?;
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
        file_path: TrackPath::new(&file_path),
        duration_label: format_duration_stack(duration as u64),
        year: year.map(|y| y as u32),
        track_num: track_num.map(|n| n as u32),
        disc_num: disc_num.map(|n| n as u32),
        album_artist: album_artist.as_deref().map(intern::intern),
        genre: genre.as_deref().map(intern::intern),
    })
}

pub(crate) fn sanitize_db_text(s: &str) -> Cow<'_, str> {
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

pub(crate) fn sanitize_db_opt(s: Option<&str>) -> Option<turso::Value> {
    s.map(|v| match sanitize_db_text(v) {
        Cow::Borrowed(b) => turso::Value::from(b),
        Cow::Owned(o) => turso::Value::from(o),
    })
}
