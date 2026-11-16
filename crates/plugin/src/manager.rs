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
    fn on_repeat_change(&self, _mode: model::RepeatMode) -> Result<(), String> {
        Ok(())
    }
    fn on_shuffle_change(&self, _is_shuffle: bool) -> Result<(), String> {
        Ok(())
    }
    fn fetch_lyrics(
        &self,
        _path: Option<&std::path::Path>,
        _artist: &str,
        _title: &str,
    ) -> Result<Option<String>, String> {
        Ok(None)
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
        manager.register_plugin(Arc::new(crate::lyrics::LyricsPlugin::new()));
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
                    error!(
                        "Plugin {} on_playback_status_change error: {}",
                        p.metadata().id,
                        e
                    );
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

    pub fn on_repeat_change(&self, mode: model::RepeatMode) {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                if let Err(e) = p.on_repeat_change(mode) {
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

    pub fn fetch_lyrics(
        &self,
        path: Option<&std::path::Path>,
        artist: &str,
        title: &str,
    ) -> Option<String> {
        if let Ok(list) = self.plugins.read() {
            for p in list.iter() {
                match p.fetch_lyrics(path, artist, title) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct TestPlugin {
        meta: PluginMetadata,
        track_changed: Arc<AtomicBool>,
        playback_ended: Arc<AtomicBool>,
    }

    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }

        fn on_track_change(&self, _track: &TrackMetadata) -> Result<(), String> {
            self.track_changed.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn on_playback_end(&self) -> Result<(), String> {
            self.playback_ended.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn fetch_lyrics(
            &self,
            _path: Option<&std::path::Path>,
            artist: &str,
            title: &str,
        ) -> Result<Option<String>, String> {
            if artist == "TestArtist" && title == "TestSong" {
                Ok(Some("La la la test lyrics".to_string()))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn test_plugin_manager_lifecycle() {
        let manager = PluginManager::new();

        let track_flag = Arc::new(AtomicBool::new(false));
        let end_flag = Arc::new(AtomicBool::new(false));

        let plugin = Arc::new(TestPlugin {
            meta: PluginMetadata {
                id: "test.plugin".to_string(),
                name: "Test Plugin".to_string(),
            },
            track_changed: track_flag.clone(),
            playback_ended: end_flag.clone(),
        });

        manager.register_plugin(plugin);

        let meta = TrackMetadata {
            title: "TestSong".to_string(),
            artist: "TestArtist".to_string(),
            album: "TestAlbum".to_string(),
            duration_ms: 180000,
            file_path: "/test/path.mp3".to_string(),
        };

        manager.on_track_change(&meta);
        assert!(track_flag.load(Ordering::SeqCst));

        manager.on_playback_end();
        assert!(end_flag.load(Ordering::SeqCst));

        let lyrics = manager.fetch_lyrics(
            Some(std::path::Path::new("/test/path.mp3")),
            "TestArtist",
            "TestSong",
        );
        assert_eq!(lyrics, Some("La la la test lyrics".to_string()));

        let no_lyrics = manager.fetch_lyrics(None, "Unknown", "Song");
        assert_eq!(no_lyrics, None);
    }

    #[test]
    fn test_plugin_hooks_dispatch() {
        use std::sync::atomic::{AtomicU64, AtomicU8};

        struct FullPlugin {
            meta: PluginMetadata,
            playing: Arc<AtomicBool>,
            position: Arc<AtomicU64>,
            repeat: Arc<AtomicU8>,
            shuffle: Arc<AtomicBool>,
        }

        impl Plugin for FullPlugin {
            fn metadata(&self) -> &PluginMetadata {
                &self.meta
            }
            fn on_playback_status_change(&self, is_playing: bool) -> Result<(), String> {
                self.playing.store(is_playing, Ordering::SeqCst);
                Ok(())
            }
            fn on_position_change(&self, pos: u64) -> Result<(), String> {
                self.position.store(pos, Ordering::SeqCst);
                Ok(())
            }
            fn on_repeat_change(&self, mode: model::RepeatMode) -> Result<(), String> {
                let code = match mode {
                    model::RepeatMode::Off => 0,
                    model::RepeatMode::All => 1,
                    model::RepeatMode::One => 2,
                };
                self.repeat.store(code, Ordering::SeqCst);
                Ok(())
            }
            fn on_shuffle_change(&self, is_shuffle: bool) -> Result<(), String> {
                self.shuffle.store(is_shuffle, Ordering::SeqCst);
                Ok(())
            }
        }

        let manager = PluginManager::new();
        let playing = Arc::new(AtomicBool::new(false));
        let position = Arc::new(AtomicU64::new(0));
        let repeat = Arc::new(AtomicU8::new(0));
        let shuffle = Arc::new(AtomicBool::new(false));

        let plugin = Arc::new(FullPlugin {
            meta: PluginMetadata {
                id: "test.full".to_string(),
                name: "Full".to_string(),
            },
            playing: playing.clone(),
            position: position.clone(),
            repeat: repeat.clone(),
            shuffle: shuffle.clone(),
        });

        manager.register_plugin(plugin);

        manager.on_playback_status_change(true);
        assert!(playing.load(Ordering::SeqCst));

        manager.on_position_change(42000);
        assert_eq!(position.load(Ordering::SeqCst), 42000);

        manager.on_repeat_change(model::RepeatMode::One);
        assert_eq!(repeat.load(Ordering::SeqCst), 2);

        manager.on_shuffle_change(true);
        assert!(shuffle.load(Ordering::SeqCst));
    }
}
