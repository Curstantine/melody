/// A single timestamped line from a synced LRC lyrics file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LrcLine {
    pub time_ms: u64,
    pub text: String,
}

/// Parsed representation of track lyrics, either timestamp-synced or plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedLyrics {
    Synced(Vec<LrcLine>),
    Plain(String),
}

impl ParsedLyrics {
    /// Parses raw lyrics text into synced LRC lines, falling back to plain text
    /// when no valid timestamps are present.
    pub fn parse(raw: &str) -> Self {
        let mut offset_ms: i64 = 0;
        let mut entries: Vec<(i64, String)> = Vec::new();

        for raw_line in raw.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            let mut rest = line;
            let mut timestamps = Vec::new();
            let mut is_metadata_only = false;

            while rest.starts_with('[') {
                let Some(close_idx) = rest.find(']') else {
                    break;
                };
                let tag = &rest[1..close_idx];
                if let Some(ts_ms) = parse_lrc_timestamp(tag) {
                    timestamps.push(ts_ms);
                    rest = &rest[close_idx + 1..];
                } else if let Some(off) = parse_lrc_offset(tag) {
                    offset_ms = off;
                    is_metadata_only = true;
                    rest = &rest[close_idx + 1..];
                } else if is_lrc_metadata_tag(tag) {
                    is_metadata_only = true;
                    rest = &rest[close_idx + 1..];
                } else {
                    break;
                }
            }

            if !timestamps.is_empty() {
                let lyric_text = rest.trim().to_string();
                for ts in timestamps {
                    entries.push((ts, lyric_text.clone()));
                }
            } else if is_metadata_only && rest.trim().is_empty() {
                continue;
            }
        }

        if entries.is_empty() {
            return Self::Plain(raw.to_string());
        }

        let mut lines: Vec<LrcLine> = entries
            .into_iter()
            .map(|(ts, text)| LrcLine {
                time_ms: (ts + offset_ms).max(0) as u64,
                text,
            })
            .collect();
        lines.sort_by_key(|l| l.time_ms);

        Self::Synced(lines)
    }

    /// Returns the index of the active lyric line for `position_ms`, if synced.
    pub fn active_line_index(&self, position_ms: u64) -> Option<usize> {
        match self {
            Self::Synced(lines) if !lines.is_empty() => {
                let idx = lines.partition_point(|l| l.time_ms <= position_ms);
                idx.checked_sub(1)
            }
            _ => None,
        }
    }
}

fn parse_lrc_timestamp(tag: &str) -> Option<i64> {
    let (min_str, sec_part) = tag.split_once(':')?;
    if min_str.is_empty() || !min_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let minutes: i64 = min_str.parse().ok()?;

    let (sec_str, frac_ms) = if let Some((s, f)) = sec_part.split_once('.') {
        (s, parse_lrc_fraction_ms(f)?)
    } else if let Some((s, f)) = sec_part.split_once(':') {
        (s, parse_lrc_fraction_ms(f)?)
    } else {
        (sec_part, 0)
    };

    if sec_str.is_empty() || !sec_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let seconds: i64 = sec_str.parse().ok()?;
    Some(minutes * 60_000 + seconds * 1_000 + frac_ms)
}

fn parse_lrc_fraction_ms(frac: &str) -> Option<i64> {
    if frac.is_empty() || frac.len() > 3 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let val: i64 = frac.parse().ok()?;
    match frac.len() {
        1 => Some(val * 100),
        2 => Some(val * 10),
        3 => Some(val),
        _ => None,
    }
}

fn parse_lrc_offset(tag: &str) -> Option<i64> {
    let (key, val) = tag.split_once(':')?;
    if key.trim().eq_ignore_ascii_case("offset") {
        val.trim().parse::<i64>().ok()
    } else {
        None
    }
}

fn is_lrc_metadata_tag(tag: &str) -> bool {
    let Some((key, _)) = tag.split_once(':') else {
        return false;
    };
    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "ar" | "ti" | "al" | "au" | "by" | "length" | "re" | "ve" | "tool"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lrc_parsing_and_active_line() {
        let raw = "\
[ar:Synthwave Master]
[ti:Midnight Drive]
[offset:500]
[00:01.00] First line
[00:05.25][00:12.50] Chorus line
[00:08.00] Second line
";
        let parsed = ParsedLyrics::parse(raw);
        let ParsedLyrics::Synced(lines) = &parsed else {
            panic!("Expected synced lyrics");
        };
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].time_ms, 1_500);
        assert_eq!(lines[0].text, "First line");
        assert_eq!(lines[1].time_ms, 5_750);
        assert_eq!(lines[1].text, "Chorus line");
        assert_eq!(lines[2].time_ms, 8_500);
        assert_eq!(lines[2].text, "Second line");
        assert_eq!(lines[3].time_ms, 13_000);
        assert_eq!(lines[3].text, "Chorus line");

        assert_eq!(parsed.active_line_index(1_000), None);
        assert_eq!(parsed.active_line_index(1_500), Some(0));
        assert_eq!(parsed.active_line_index(6_000), Some(1));
        assert_eq!(parsed.active_line_index(99_000), Some(3));
    }

    #[test]
    fn test_lrc_plain_text_fallback() {
        let parsed = ParsedLyrics::parse("Just plain lyrics\nNo timestamps here");
        assert_eq!(
            parsed,
            ParsedLyrics::Plain("Just plain lyrics\nNo timestamps here".to_string())
        );
        assert_eq!(parsed.active_line_index(1_000), None);
    }
}
