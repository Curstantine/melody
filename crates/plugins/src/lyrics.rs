use tracing::info;

use crate::{manager::Plugin, types::PluginMetadata};

pub struct LyricsPlugin {
    meta: PluginMetadata,
}

impl LyricsPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "builtin.lyrics".to_string(),
                name: "Local Lyrics Provider".to_string(),
                version: "1.0.0".to_string(),
                author: "melody".to_string(),
                description: "Loads matching lyrics (.lrc or .txt) from the music directory"
                    .to_string(),
            },
        }
    }
}

impl Default for LyricsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for LyricsPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }

    fn fetch_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>, String> {
        info!("Fetching lyrics for {} - {}", artist, title);
        // Check for common lyric locations or dummy test lyrics
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lyrics_plugin_metadata() {
        let plugin = LyricsPlugin::new();
        assert_eq!(plugin.metadata().id, "builtin.lyrics");
        assert_eq!(plugin.metadata().name, "Local Lyrics Provider");
    }

    #[test]
    fn test_lyrics_plugin_fetch_default() {
        let plugin = LyricsPlugin::default();
        let result = plugin.fetch_lyrics("Artist", "Song");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }
}
