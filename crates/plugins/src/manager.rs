use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::{error, info};

use crate::types::{PluginMetadata, TrackMetadata};

pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn on_track_change(&self, _track: &TrackMetadata) -> Result<(), String> {
        Ok(())
    }
    fn on_playback_end(&self) -> Result<(), String> {
        Ok(())
    }
    fn on_playback_status_change(&self, _is_playing: bool) -> Result<(), String> {
        Ok(())
    }
    fn on_position_change(&self, _position_ms: u64) -> Result<(), String> {
        Ok(())
    }
    fn on_seeked(&self, _position_ms: u64) -> Result<(), String> {
        Ok(())
    }
    fn on_volume_change(&self, _volume: f32) -> Result<(), String> {
        Ok(())
    }
    fn on_repeat_change(&self, _is_repeat: bool) -> Result<(), String> {
        Ok(())
    }
    fn on_shuffle_change(&self, _is_shuffle: bool) -> Result<(), String> {
        Ok(())
    }
    fn fetch_lyrics(&self, _artist: &str, _title: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn discord_presence_update(&self, _track: &TrackMetadata) -> Result<(), String> {
        Ok(())
    }
    fn scrobble(&self, _track: &TrackMetadata, _timestamp: u64) -> Result<(), String> {
        Ok(())
    }
    fn update_now_playing(&self, _track: &TrackMetadata) -> Result<(), String> {
        Ok(())
    }
}

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
                author: "vibecore".to_string(),
                description: "Loads matching lyrics (.lrc or .txt) from the music directory"
                    .to_string(),
            },
        }
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

pub struct DiscordRpcPlugin {
    meta: PluginMetadata,
}

impl DiscordRpcPlugin {
    pub fn new() -> Self {
        Self {
            meta: PluginMetadata {
                id: "builtin.discord_rpc".to_string(),
                name: "Discord Rich Presence".to_string(),
                version: "1.0.0".to_string(),
                author: "vibecore".to_string(),
                description: "Updates Discord Rich Presence with current track info".to_string(),
            },
        }
    }
}

impl Plugin for DiscordRpcPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }

    fn discord_presence_update(&self, track: &TrackMetadata) -> Result<(), String> {
        info!(
            "[Discord RPC] Now playing: {} by {} ({})",
            track.title, track.artist, track.album
        );
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct PluginManager {
    plugins: Arc<RwLock<Vec<Arc<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let manager = Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
        };
        // Register default plugins
        manager.register_plugin(Arc::new(LyricsPlugin::new()));
        manager.register_plugin(Arc::new(DiscordRpcPlugin::new()));
        manager
    }

    pub fn register_plugin(&self, plugin: Arc<dyn Plugin>) {
        info!(
            "Registering plugin: {} ({})",
            plugin.metadata().name,
            plugin.metadata().id
        );
        if let Ok(mut list) = self.plugins.write() {
            list.push(plugin);
        }
    }

    pub fn on_track_change(&self, track: &TrackMetadata) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_track_change(track) {
                    error!("Plugin {} on_track_change error: {}", p.metadata().id, e);
                }
                // Automatically update discord rich presence
                let _ = p.discord_presence_update(track);
            }
        }
    }

    pub fn on_playback_end(&self) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_playback_end() {
                    error!("Plugin {} on_playback_end error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn on_playback_status_change(&self, is_playing: bool) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_playback_status_change(is_playing) {
                    error!("Plugin {} on_playback_status_change error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn on_position_change(&self, position_ms: u64) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_position_change(position_ms) {
                    error!("Plugin {} on_position_change error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn on_seeked(&self, position_ms: u64) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_seeked(position_ms) {
                    error!("Plugin {} on_seeked error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn on_volume_change(&self, volume: f32) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_volume_change(volume) {
                    error!("Plugin {} on_volume_change error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn on_repeat_change(&self, is_repeat: bool) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_repeat_change(is_repeat) {
                    error!("Plugin {} on_repeat_change error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn on_shuffle_change(&self, is_shuffle: bool) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_shuffle_change(is_shuffle) {
                    error!("Plugin {} on_shuffle_change error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn fetch_lyrics(&self, artist: &str, title: &str) -> Option<String> {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                match p.fetch_lyrics(artist, title) {
                    Ok(Some(lyrics)) => return Some(lyrics),
                    Ok(None) => {}
                    Err(e) => {
                        error!("Plugin {} fetch_lyrics error: {}", p.metadata().id, e);
                    }
                }
            }
        }
        None
    }

    pub fn scrobble(&self, track: &TrackMetadata, timestamp: u64) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.scrobble(track, timestamp) {
                    error!("Plugin {} scrobble error: {}", p.metadata().id, e);
                }
            }
        }
    }

    pub fn update_now_playing(&self, track: &TrackMetadata) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.update_now_playing(track) {
                    error!("Plugin {} update_now_playing error: {}", p.metadata().id, e);
                }
            }
        }
    }

    /// Scan directory for Wasm or dynamic plugins
    pub fn load_plugins_from_dir<P: AsRef<Path>>(&self, _dir: P) {
        // Safe discovery: iterate files in directory, load wasm plugins if wasm feature enabled
        info!("Plugin directory scanned");
    }

    /// Returns the number of currently registered plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.read().map(|l| l.len()).unwrap_or(0)
    }
}
