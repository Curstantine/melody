//! Structured library search filters.
//!
//! The library search box accepts plain free text as well as field-scoped
//! terms such as `album:"Kind of Blue"`, `artist:Miles` or `genre:Jazz`.
//! Categorical fields (`album`, `artist`, `album_artist`, `genre`, `year`)
//! match exactly, while `title` and free text match as substrings. This is
//! what allows clicking an album to show *only* that album instead of every
//! track that merely contains the album name somewhere.

/// A parsed library filter.
///
/// An empty filter matches the whole library.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackFilter {
    /// Free-text terms, each matched as a substring across title/artist/album.
    pub text: Vec<String>,
    /// Exact title match (substring) — `title:...`.
    pub title: Option<String>,
    /// Exact album match — `album:"..."`
    pub album: Option<String>,
    /// Exact album-artist match — `album_artist:"..."`
    pub album_artist: Option<String>,
    /// Exact artist match — `artist:"..."`
    pub artist: Option<String>,
    /// Exact genre match — `genre:"..."`
    pub genre: Option<String>,
    /// Exact year match — `year:2020`
    pub year: Option<u32>,
}

impl TrackFilter {
    /// Parses a raw search query into a structured filter.
    pub fn parse(input: &str) -> Self {
        let mut filter = TrackFilter::default();
        for token in tokenize(input) {
            let Some((field, value)) = split_field(&token) else {
                filter.text.push(token);
                continue;
            };
            if value.trim().is_empty() {
                continue;
            }
            match field.to_ascii_lowercase().as_str() {
                "album" => filter.album = Some(value),
                "artist" => filter.artist = Some(value),
                "album_artist" | "albumartist" => filter.album_artist = Some(value),
                "genre" => filter.genre = Some(value),
                "title" => filter.title = Some(value),
                "year" => match value.trim().parse::<u32>() {
                    Ok(year) => filter.year = Some(year),
                    // Not a valid year: keep the raw token as free text.
                    Err(_) => filter.text.push(token),
                },
                _ => filter.text.push(token),
            }
        }
        filter
    }

    /// Builds a filter that selects exactly one album.
    pub fn album(name: impl Into<String>) -> Self {
        Self {
            album: Some(name.into()),
            ..Default::default()
        }
    }

    /// Builds a filter that selects exactly one artist.
    pub fn artist(name: impl Into<String>) -> Self {
        Self {
            artist: Some(name.into()),
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
            && self.title.is_none()
            && self.album.is_none()
            && self.album_artist.is_none()
            && self.artist.is_none()
            && self.genre.is_none()
            && self.year.is_none()
    }

    /// Derives a human-readable queue name from the filter's dominant scope.
    pub fn queue_name(&self) -> Option<String> {
        if let Some(album) = &self.album {
            return Some(album.clone());
        }
        if let Some(artist) = &self.artist {
            return Some(artist.clone());
        }
        if let Some(album_artist) = &self.album_artist {
            return Some(album_artist.clone());
        }
        if let Some(genre) = &self.genre {
            return Some(genre.clone());
        }
        match self.text.len() {
            0 => None,
            1 => Some(self.text[0].clone()),
            _ => Some(format!("Search: {}", self.text.join(" "))),
        }
    }

    /// Canonical query string, suitable for displaying in the search box.
    pub fn to_query_string(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(v) = &self.title {
            parts.push(format!("title:{}", format_value(v)));
        }
        if let Some(v) = &self.album {
            parts.push(format!("album:{}", format_value(v)));
        }
        if let Some(v) = &self.album_artist {
            parts.push(format!("album_artist:{}", format_value(v)));
        }
        if let Some(v) = &self.artist {
            parts.push(format!("artist:{}", format_value(v)));
        }
        if let Some(v) = &self.genre {
            parts.push(format!("genre:{}", format_value(v)));
        }
        if let Some(v) = self.year {
            parts.push(format!("year:{}", v));
        }
        for term in &self.text {
            parts.push(format_value(term));
        }
        parts.join(" ")
    }
}

/// Splits `input` into whitespace-separated tokens, honouring double quotes.
///
/// `album:"Foo Bar"` becomes the single token `album:Foo Bar`. A backslash
/// inside quotes escapes the next character.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut has_content = false;
    let mut in_quotes = false;
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        match c {
            '\\' if in_quotes => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    has_content = true;
                }
            }
            '"' => {
                in_quotes = !in_quotes;
                has_content = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_content {
                    tokens.push(std::mem::take(&mut current));
                    has_content = false;
                }
            }
            c => {
                current.push(c);
                has_content = true;
            }
        }
    }

    if has_content {
        tokens.push(current);
    }
    tokens
}

/// Splits a token into a known field prefix and its value, if it looks like one.
fn split_field(token: &str) -> Option<(&str, String)> {
    let (field, value) = token.split_once(':')?;
    if field.is_empty() {
        return None;
    }
    let known = matches!(
        field.to_ascii_lowercase().as_str(),
        "album" | "artist" | "album_artist" | "albumartist" | "genre" | "title" | "year"
    );
    if known {
        Some((field, value.to_string()))
    } else {
        None
    }
}

/// Quotes a value when it contains whitespace or quotes.
fn format_value(value: &str) -> String {
    if value.contains(char::is_whitespace) || value.contains('"') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_free_text() {
        let filter = TrackFilter::parse("kind of blue");
        assert_eq!(filter.text, vec!["kind", "of", "blue"]);
        assert!(filter.album.is_none());
        assert!(!filter.is_empty());
    }

    #[test]
    fn parses_scoped_fields() {
        let filter = TrackFilter::parse("album:\"Kind of Blue\" artist:Miles year:1959");
        assert_eq!(filter.album.as_deref(), Some("Kind of Blue"));
        assert_eq!(filter.artist.as_deref(), Some("Miles"));
        assert_eq!(filter.year, Some(1959));
    }

    #[test]
    fn unknown_prefix_is_free_text() {
        let filter = TrackFilter::parse("http://example.com");
        assert_eq!(filter.text, vec!["http://example.com"]);
    }

    #[test]
    fn queue_name_prefers_album_then_artist() {
        assert_eq!(
            TrackFilter::parse("album:A artist:B")
                .queue_name()
                .as_deref(),
            Some("A")
        );
        assert_eq!(
            TrackFilter::parse("artist:B").queue_name().as_deref(),
            Some("B")
        );
        assert_eq!(TrackFilter::parse("").queue_name(), None);
    }

    #[test]
    fn roundtrips_through_query_string() {
        let original = TrackFilter::parse("album:\"Kind of Blue\" genre:Jazz");
        let roundtrip = TrackFilter::parse(&original.to_query_string());
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn empty_field_values_are_ignored() {
        let filter = TrackFilter::parse("album: genre:");
        assert!(filter.is_empty());
    }
}
