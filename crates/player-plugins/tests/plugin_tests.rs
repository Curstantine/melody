use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use player_plugins::{Plugin, PluginManager, PluginMetadata, TrackMetadata};

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

    fn fetch_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>, String> {
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
            version: "0.1.0".to_string(),
            author: "Tester".to_string(),
            description: "Test".to_string(),
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

    let lyrics = manager.fetch_lyrics("TestArtist", "TestSong");
    assert_eq!(lyrics, Some("La la la test lyrics".to_string()));

    let no_lyrics = manager.fetch_lyrics("Unknown", "Song");
    assert_eq!(no_lyrics, None);
}
