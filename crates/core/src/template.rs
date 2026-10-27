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
            "album" | "title" => Some(Cow::Borrowed(&self.album)),
            "artist" | "album_artist" => Some(Cow::Borrowed(&self.artist)),
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
            "artist" | "name" => Some(Cow::Borrowed(&self.artist)),
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
            "album" => Some(Cow::Borrowed(&self.album)),
            "album_artist" => self.album_artist.as_deref().map(Cow::Borrowed),
            "genre" => self.genre.as_deref().map(Cow::Borrowed),
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
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            result.push('{');
            i += 2;
        } else if bytes[i] == b'}' && i + 1 < bytes.len() && bytes[i + 1] == b'}' {
            result.push('}');
            i += 2;
        } else if bytes[i] == b'[' {
            // Find matching ']'
            let start = i + 1;
            let mut depth = 1;
            let mut end = start;
            while end < bytes.len() && depth > 0 {
                if bytes[end] == b'[' {
                    depth += 1;
                } else if bytes[end] == b']' {
                    depth -= 1;
                }
                if depth > 0 {
                    end += 1;
                }
            }

            if depth == 0 {
                let inner_slice = &template[start..end];
                // Check if all placeholders in inner_slice are satisfied
                if all_placeholders_satisfied(inner_slice, lookup) {
                    format_template_direct_into(&mut result, inner_slice, lookup);
                }
                i = end + 1;
            } else {
                // Unclosed '[', emit as literal
                result.push('[');
                i += 1;
            }
        } else if bytes[i] == b'{' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'}' {
                end += 1;
            }
            if end < bytes.len() {
                let key = &template[start..end];
                if let Some(val) = lookup.lookup(key) {
                    result.push_str(val.trim());
                }
                i = end + 1;
            } else {
                result.push('{');
                i += 1;
            }
        } else {
            let mut next = i + 1;
            while next < bytes.len()
                && bytes[next] != b'{'
                && bytes[next] != b'}'
                && bytes[next] != b'['
            {
                next += 1;
            }
            result.push_str(&template[i..next]);
            i = next;
        }
    }

    clean_formatted_string(&result)
}

/// Checks whether all `{placeholders}` within a conditional section have non-empty values.
fn all_placeholders_satisfied(sub_template: &str, lookup: &impl FieldLookup) -> bool {
    let bytes = sub_template.as_bytes();
    let mut i = 0;
    let mut has_at_least_one = false;

    while i < bytes.len() {
        if bytes[i] == b'{' && (i + 1 >= bytes.len() || bytes[i + 1] != b'{') {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'}' {
                end += 1;
            }
            if end < bytes.len() {
                let key = &sub_template[start..end];
                has_at_least_one = true;
                match lookup.lookup(key) {
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

/// Direct placeholder replacement without bracket recursion into an existing buffer.
fn format_template_direct_into(result: &mut String, template: &str, lookup: &impl FieldLookup) {
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            result.push('{');
            i += 2;
        } else if bytes[i] == b'}' && i + 1 < bytes.len() && bytes[i + 1] == b'}' {
            result.push('}');
            i += 2;
        } else if bytes[i] == b'{' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'}' {
                end += 1;
            }
            if end < bytes.len() {
                let key = &template[start..end];
                if let Some(val) = lookup.lookup(key) {
                    result.push_str(val.trim());
                }
                i = end + 1;
            } else {
                result.push('{');
                i += 1;
            }
        } else {
            let mut next = i + 1;
            while next < bytes.len() && bytes[next] != b'{' && bytes[next] != b'}' {
                next += 1;
            }
            result.push_str(&template[i..next]);
            i = next;
        }
    }
}

/// Normalizes whitespace and strips dangling separators like `•`, `-`, `,`, `|`, `/`.
fn clean_formatted_string(raw: &str) -> String {
    let is_separator = |c: char| matches!(c, '•' | '-' | ',' | '|' | '/' | '–' | '—');
    let trimmed = raw.trim_matches(|c: char| c.is_whitespace() || is_separator(c));
    if trimmed.is_empty() {
        return String::new();
    }
    let mut result = String::with_capacity(trimmed.len());
    let mut words = trimmed.split_whitespace();
    if let Some(first) = words.next() {
        result.push_str(first);
        for word in words {
            result.push(' ');
            result.push_str(word);
        }
    }
    result
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
        map.insert(
            "title".to_string(),
            "Yūaku no Inori - Anima immortalis est.".to_string(),
        );
        map.insert(
            "artist".to_string(),
            "Imperial Circus Dead Decadence".to_string(),
        );
        let lookup = MapLookup(map);

        assert_eq!(
            format_template("{title} - {artist}", &lookup),
            "Yūaku no Inori - Anima immortalis est. - Imperial Circus Dead Decadence"
        );
    }

    #[test]
    fn test_conditional_brackets() {
        let mut map = HashMap::new();
        map.insert(
            "artist".to_string(),
            "Imperial Circus Dead Decadence".to_string(),
        );
        map.insert("year".to_string(), "2025".to_string());
        let lookup = MapLookup(map);

        assert_eq!(
            format_template("{artist}[ • {year}]", &lookup),
            "Imperial Circus Dead Decadence • 2025"
        );

        let mut map_no_year = HashMap::new();
        map_no_year.insert(
            "artist".to_string(),
            "Imperial Circus Dead Decadence".to_string(),
        );
        let lookup_no_year = MapLookup(map_no_year);

        assert_eq!(
            format_template("{artist}[ • {year}]", &lookup_no_year),
            "Imperial Circus Dead Decadence"
        );
    }

    #[test]
    fn test_dangling_separator_cleanup() {
        let mut map = HashMap::new();
        map.insert(
            "artist".to_string(),
            "Imperial Circus Dead Decadence".to_string(),
        );
        let lookup = MapLookup(map);

        assert_eq!(
            format_template("{artist} • {year}", &lookup),
            "Imperial Circus Dead Decadence"
        );
    }

    #[test]
    fn test_album_summary_lookup() {
        let album = AlbumSummary {
            album: "Yūaku no Inori - Anima immortalis est.".into(),
            artist: "Imperial Circus Dead Decadence".into(),
            track_count: 1,
            total_duration_ms: 472_000,
            year: Some(2025),
            cover_track_id: Some(101),
            added_timestamp: 1733529600,
            year_label: "2025".into(),
            tracks_label: "1 track".into(),
            duration_label: "7:52".into(),
        };

        assert_eq!(
            format_template("{title}", &album),
            "Yūaku no Inori - Anima immortalis est."
        );
        assert_eq!(
            format_template("{album_artist}[ • {year}]", &album),
            "Imperial Circus Dead Decadence • 2025"
        );
        assert_eq!(
            format_template("{tracks_label} • {duration_label}", &album),
            "1 track • 7:52"
        );
    }

    #[test]
    fn test_track_row_lookup() {
        let track = TrackRow {
            id: 101,
            title: "Yūaku no Inori - Anima immortalis est.".into(),
            artist: "Imperial Circus Dead Decadence".into(),
            album: "Yūaku no Inori - Anima immortalis est.".into(),
            duration_ms: 472_000,
            file_path: "/music/ICDD - Anima.flac".into(),
            duration_label: "7:52".into(),
            year: Some(2025),
            track_num: Some(1),
            disc_num: Some(1),
            album_artist: Some("Imperial Circus Dead Decadence".into()),
            genre: Some("Symphonic Melodic Death Metal".into()),
        };

        assert_eq!(
            format_template("{track_num}. {title}[ ({year})]", &track),
            "1. Yūaku no Inori - Anima immortalis est. (2025)"
        );
        assert_eq!(
            format_template("{artist} - {album} - {genre}", &track),
            "Imperial Circus Dead Decadence - Yūaku no Inori - Anima immortalis est. - Symphonic Melodic Death Metal"
        );
    }
}
