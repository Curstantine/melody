use std::borrow::Cow;

use crate::model::{AlbumSummary, ArtistSummary, PlaylistSummary, QueueRecord, TrackRow};

/// Trait for resolving field values by string key.
pub trait FieldLookup {
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>>;
}

impl<F> FieldLookup for F
where
    F: Fn(&str) -> Option<String>,
{
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
        self(key).map(Cow::Owned)
    }
}

impl FieldLookup for AlbumSummary {
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
        match key {
            "album" | "title" => Some(Cow::Borrowed(self.album.as_str())),
            "artist" | "album_artist" => Some(Cow::Borrowed(self.artist.as_str())),
            "track_count" | "tracks_count" => Some(Cow::Owned(self.track_count.to_string())),
            "tracks_label" | "tracks" => Some(Cow::Borrowed(&self.tracks_label)),
            "year" => self.year.map(|y| Cow::Owned(y.to_string())),
            "year_label" => Some(Cow::Borrowed(&self.year_label)),
            "duration_label" | "duration" => Some(Cow::Borrowed(&self.duration_label)),
            "total_duration_ms" => Some(Cow::Owned(self.total_duration_ms.to_string())),
            "cover_track_id" => self.cover_track_id.map(|id| Cow::Owned(id.to_string())),
            "added_timestamp" => Some(Cow::Owned(self.added_timestamp.to_string())),
            _ => None,
        }
    }
}

impl FieldLookup for ArtistSummary {
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
        match key {
            "artist" | "name" => Some(Cow::Borrowed(self.artist.as_str())),
            "track_count" | "tracks" => Some(Cow::Owned(self.track_count.to_string())),
            "album_count" | "albums" => Some(Cow::Owned(self.album_count.to_string())),
            "stats_label" | "stats" => Some(Cow::Borrowed(&self.stats_label)),
            _ => None,
        }
    }
}

impl FieldLookup for PlaylistSummary {
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
        match key {
            "id" => Some(Cow::Owned(self.id.to_string())),
            "name" | "title" => Some(Cow::Borrowed(&self.name)),
            "file_path" | "path" => Some(Cow::Borrowed(&self.file_path)),
            "track_count" => Some(Cow::Owned(self.track_count.to_string())),
            "tracks_label" | "tracks" => Some(Cow::Borrowed(&self.tracks_label)),
            "managed" => Some(Cow::Borrowed(if self.managed {
                "Managed"
            } else {
                "External"
            })),
            "created_timestamp" => Some(Cow::Owned(self.created_timestamp.to_string())),
            "modified_timestamp" => Some(Cow::Owned(self.modified_timestamp.to_string())),
            _ => None,
        }
    }
}

impl FieldLookup for TrackRow {
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
        match key {
            "id" => Some(Cow::Owned(self.id.to_string())),
            "title" => Some(Cow::Borrowed(&self.title)),
            "artist" => Some(Cow::Borrowed(self.display_artist())),
            "album" => Some(Cow::Borrowed(self.album.as_str())),
            "album_artist" => self
                .album_artist
                .as_ref()
                .map(|aa| Cow::Borrowed(aa.as_str())),
            "genre" => self.genre.as_ref().map(|g| Cow::Borrowed(g.as_str())),
            "year" => self.year.map(|y| Cow::Owned(y.to_string())),
            "track_num" | "track_number" => self.track_num.map(|n| Cow::Owned(n.to_string())),
            "disc_num" | "disc_number" => self.disc_num.map(|d| Cow::Owned(d.to_string())),
            "duration_ms" => Some(Cow::Owned(self.duration_ms.to_string())),
            "duration_label" | "duration" => Some(Cow::Borrowed(&self.duration_label)),
            "file_path" | "path" => Some(Cow::Borrowed(&self.file_path)),
            _ => None,
        }
    }
}

impl FieldLookup for QueueRecord {
    fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
        match key {
            "id" => Some(Cow::Owned(self.id.to_string())),
            "name" | "title" => Some(Cow::Borrowed(&self.name)),
            "current" => Some(Cow::Owned(self.current.to_string())),
            "source_kind" | "source" => Some(Cow::Borrowed(&self.source_kind)),
            "source_value" => self.source_value.as_deref().map(Cow::Borrowed),
            "source_playlist" => self.source_playlist.as_deref().map(Cow::Borrowed),
            "track_count" | "tracks" => Some(Cow::Owned(self.track_count.to_string())),
            _ => None,
        }
    }
}

/// Formats a template string by substituting `{field}` placeholders using the provided lookup.
///
/// Supported syntax:
/// - `{field}`: Replaced with the value of `field`. If empty or not found, replaced with `""`.
/// - `[...{field}...]`: Conditional group. Evaluated only if all placeholders inside resolve to non-empty values.
/// - `{{` and `}}`: Escaped literal `{` and `}`.
///
/// Post-processing cleans up consecutive spaces, extra whitespace, and dangling separators (`•`, `-`, `,`, `|`, `/`).
pub fn format_template(template: &str, lookup: &impl FieldLookup) -> String {
    let mut result = String::with_capacity(template.len() + 16);
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            result.push('{');
            i += 2;
        } else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            result.push('}');
            i += 2;
        } else if chars[i] == '[' {
            // Find matching ']'
            let start = i + 1;
            let mut depth = 1;
            let mut end = start;
            while end < chars.len() && depth > 0 {
                if chars[end] == '[' {
                    depth += 1;
                } else if chars[end] == ']' {
                    depth -= 1;
                }
                if depth > 0 {
                    end += 1;
                }
            }

            if depth == 0 {
                let inner_slice: String = chars[start..end].iter().collect();
                // Check if all placeholders in inner_slice are satisfied
                if all_placeholders_satisfied(&inner_slice, lookup) {
                    let formatted_inner = format_template_direct(&inner_slice, lookup);
                    result.push_str(&formatted_inner);
                }
                i = end + 1;
            } else {
                // Unclosed '[', emit as literal
                result.push('[');
                i += 1;
            }
        } else if chars[i] == '{' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                let key: String = chars[start..end].iter().collect();
                if let Some(val) = lookup.lookup(&key) {
                    result.push_str(val.trim());
                }
                i = end + 1;
            } else {
                result.push('{');
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    clean_formatted_string(&result)
}

/// Checks whether all `{placeholders}` within a conditional section have non-empty values.
fn all_placeholders_satisfied(sub_template: &str, lookup: &impl FieldLookup) -> bool {
    let chars: Vec<char> = sub_template.chars().collect();
    let mut i = 0;
    let mut has_at_least_one = false;

    while i < chars.len() {
        if chars[i] == '{' && (i + 1 >= chars.len() || chars[i + 1] != '{') {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                let key: String = chars[start..end].iter().collect();
                has_at_least_one = true;
                match lookup.lookup(&key) {
                    Some(val) if !val.trim().is_empty() => {}
                    _ => return false,
                }
                i = end + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    has_at_least_one
}

/// Direct placeholder replacement without bracket recursion.
fn format_template_direct(template: &str, lookup: &impl FieldLookup) -> String {
    let mut result = String::with_capacity(template.len());
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
            result.push('{');
            i += 2;
        } else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            result.push('}');
            i += 2;
        } else if chars[i] == '{' {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                let key: String = chars[start..end].iter().collect();
                if let Some(val) = lookup.lookup(&key) {
                    result.push_str(val.trim());
                }
                i = end + 1;
            } else {
                result.push('{');
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Normalizes whitespace and strips dangling separators like `•`, `-`, `,`, `|`, `/`.
fn clean_formatted_string(raw: &str) -> String {
    // 1. Collapse multiple whitespace characters into single space
    let mut words = Vec::new();
    for word in raw.split_whitespace() {
        words.push(word);
    }
    let collapsed = words.join(" ");

    // 2. Strip leading / trailing separators
    let trimmed = collapsed.trim();
    let is_separator = |c: char| matches!(c, '•' | '-' | ',' | '|' | '/' | '–' | '—');

    let mut start = 0;
    let chars: Vec<char> = trimmed.chars().collect();
    while start < chars.len() && (chars[start].is_whitespace() || is_separator(chars[start])) {
        start += 1;
    }

    let mut end = chars.len();
    while end > start && (chars[end - 1].is_whitespace() || is_separator(chars[end - 1])) {
        end -= 1;
    }

    if start >= end {
        return String::new();
    }

    let s: String = chars[start..end].iter().collect();
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MapLookup(HashMap<String, String>);

    impl FieldLookup for MapLookup {
        fn lookup(&self, key: &str) -> Option<Cow<'_, str>> {
            self.0.get(key).map(|s| Cow::Borrowed(s.as_str()))
        }
    }

    #[test]
    fn test_direct_formatting() {
        let mut map = HashMap::new();
        map.insert("title".to_string(), "Echoes".to_string());
        map.insert("artist".to_string(), "Pink Floyd".to_string());
        let lookup = MapLookup(map);

        assert_eq!(
            format_template("{title} - {artist}", &lookup),
            "Echoes - Pink Floyd"
        );
    }

    #[test]
    fn test_conditional_brackets() {
        let mut map = HashMap::new();
        map.insert("artist".to_string(), "Pink Floyd".to_string());
        map.insert("year".to_string(), "1971".to_string());
        let lookup = MapLookup(map);

        assert_eq!(
            format_template("{artist}[ • {year}]", &lookup),
            "Pink Floyd • 1971"
        );

        let mut map_no_year = HashMap::new();
        map_no_year.insert("artist".to_string(), "Pink Floyd".to_string());
        let lookup_no_year = MapLookup(map_no_year);

        assert_eq!(
            format_template("{artist}[ • {year}]", &lookup_no_year),
            "Pink Floyd"
        );
    }

    #[test]
    fn test_dangling_separator_cleanup() {
        let mut map = HashMap::new();
        map.insert("artist".to_string(), "Pink Floyd".to_string());
        let lookup = MapLookup(map);

        assert_eq!(format_template("{artist} • {year}", &lookup), "Pink Floyd");
    }

    #[test]
    fn test_album_summary_lookup() {
        let album = AlbumSummary {
            album: "Meddle".into(),
            artist: "Pink Floyd".into(),
            track_count: 6,
            total_duration_ms: 2792000,
            year: Some(1971),
            cover_track_id: Some(1),
            added_timestamp: 123456,
            year_label: "1971".into(),
            tracks_label: "6 tracks".into(),
            duration_label: "46:32".into(),
        };

        assert_eq!(format_template("{title}", &album), "Meddle");
        assert_eq!(
            format_template("{album_artist}[ • {year}]", &album),
            "Pink Floyd • 1971"
        );
        assert_eq!(
            format_template("{tracks_label} • {duration_label}", &album),
            "6 tracks • 46:32"
        );
    }

    #[test]
    fn test_track_row_lookup() {
        let track = TrackRow {
            id: 1,
            title: "One of These Days".into(),
            artist: "Pink Floyd".into(),
            album: "Meddle".into(),
            duration_ms: 357000,
            file_path: "/music/one.flac".into(),
            duration_label: "5:57".into(),
            year: Some(1971),
            track_num: Some(1),
            disc_num: Some(1),
            album_artist: Some("Pink Floyd".into()),
            genre: Some("Progressive Rock".into()),
        };

        assert_eq!(
            format_template("{track_num}. {title}[ ({year})]", &track),
            "1. One of These Days (1971)"
        );
        assert_eq!(
            format_template("{artist} - {album} - {genre}", &track),
            "Pink Floyd - Meddle - Progressive Rock"
        );
    }
}
