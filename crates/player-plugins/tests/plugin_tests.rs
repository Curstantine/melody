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

#[test]
fn test_plugin_hooks_dispatch() {
    use std::sync::atomic::AtomicU64;

    struct FullPlugin {
        meta: PluginMetadata,
        playing: Arc<AtomicBool>,
        position: Arc<AtomicU64>,
        repeat: Arc<AtomicBool>,
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
        fn on_repeat_change(&self, is_repeat: bool) -> Result<(), String> {
            self.repeat.store(is_repeat, Ordering::SeqCst);
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
    let repeat = Arc::new(AtomicBool::new(false));
    let shuffle = Arc::new(AtomicBool::new(false));

    let plugin = Arc::new(FullPlugin {
        meta: PluginMetadata {
            id: "test.full".to_string(),
            name: "Full".to_string(),
            version: "1.0.0".to_string(),
            author: "Tester".to_string(),
            description: "".to_string(),
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

    manager.on_repeat_change(true);
    assert!(repeat.load(Ordering::SeqCst));

    manager.on_shuffle_change(true);
    assert!(shuffle.load(Ordering::SeqCst));
}

#[cfg(target_os = "linux")]
#[test]
fn test_mpris_plugin_instantiation_and_metadata() {
    use player_plugins::MprisPlugin;

    let (tx, _rx) = crossbeam_channel::unbounded();
    let mpris = MprisPlugin::new(tx);
    assert_eq!(mpris.metadata().id, "builtin.mpris");

    let meta = TrackMetadata {
        title: "Test Track".to_string(),
        artist: "Test Artist".to_string(),
        album: "Test Album".to_string(),
        duration_ms: 240000,
        file_path: "/music/test.mp3".to_string(),
    };

    assert!(mpris.on_track_change(&meta).is_ok());
    assert!(mpris.on_playback_status_change(true).is_ok());
    assert!(mpris.on_position_change(5000).is_ok());
    assert!(mpris.on_seeked(6000).is_ok());
    assert!(mpris.on_volume_change(0.8).is_ok());
    assert!(mpris.on_repeat_change(true).is_ok());
    assert!(mpris.on_shuffle_change(true).is_ok());
    assert!(mpris.on_playback_end().is_ok());
}
