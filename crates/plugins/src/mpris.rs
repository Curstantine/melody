#[cfg(target_os = "linux")]
mod linux {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, RwLock};
    use std::time::Duration;

    use crossbeam_channel::Sender;
    use mpris_server::zbus::{fdo, Result as ZbusResult};
    use mpris_server::{
        LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property,
        RootInterface, Server, Signal, Time, TrackId, Volume,
    };
    use tracing::{error, info};
    use url::Url;

    use crate::manager::Plugin;
    use crate::types::{PlayerAction, PluginMetadata, TrackMetadata};

    #[derive(Clone)]
    struct MprisInner {
        action_tx: Sender<PlayerAction>,
        current_track: Arc<RwLock<Option<TrackMetadata>>>,
        is_playing: Arc<AtomicBool>,
        position_ms: Arc<AtomicU64>,
        volume: Arc<AtomicU32>, // stored as (vol * 10000.0) as u32
        is_shuffle: Arc<AtomicBool>,
        is_repeat: Arc<AtomicBool>,
    }

    impl MprisInner {
        fn new(action_tx: Sender<PlayerAction>) -> Self {
            Self {
                action_tx,
                current_track: Arc::new(RwLock::new(None)),
                is_playing: Arc::new(AtomicBool::new(false)),
                position_ms: Arc::new(AtomicU64::new(0)),
                volume: Arc::new(AtomicU32::new(10000)), // 1.0 default
                is_shuffle: Arc::new(AtomicBool::new(false)),
                is_repeat: Arc::new(AtomicBool::new(false)),
            }
        }

        fn get_volume(&self) -> f64 {
            self.volume.load(Ordering::Relaxed) as f64 / 10000.0
        }

        fn set_volume_ratio(&self, vol: f64) {
            let clamped = vol.clamp(0.0, 1.0);
            self.volume
                .store((clamped * 10000.0) as u32, Ordering::Relaxed);
        }
    }

    impl RootInterface for MprisInner {
        async fn raise(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Raise);
            Ok(())
        }

        async fn quit(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Quit);
            Ok(())
        }

        async fn can_quit(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn fullscreen(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn set_fullscreen(&self, _fullscreen: bool) -> ZbusResult<()> {
            Ok(())
        }

        async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn can_raise(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn has_track_list(&self) -> fdo::Result<bool> {
            Ok(false)
        }

        async fn identity(&self) -> fdo::Result<String> {
            Ok("Vibecore".to_string())
        }

        async fn desktop_entry(&self) -> fdo::Result<String> {
            Ok("vibecore".to_string())
        }

        async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
            Ok(vec!["file".to_string()])
        }

        async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
            Ok(vec![
                "audio/mpeg".to_string(),
                "audio/flac".to_string(),
                "audio/x-vorbis+ogg".to_string(),
                "audio/ogg".to_string(),
                "audio/x-wav".to_string(),
                "audio/wav".to_string(),
                "audio/aac".to_string(),
                "audio/mp4".to_string(),
            ])
        }
    }

    impl PlayerInterface for MprisInner {
        async fn next(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Next);
            Ok(())
        }

        async fn previous(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Previous);
            Ok(())
        }

        async fn pause(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Pause);
            Ok(())
        }

        async fn play_pause(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::TogglePlayPause);
            Ok(())
        }

        async fn stop(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Stop);
            Ok(())
        }

        async fn play(&self) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::Play);
            Ok(())
        }

        async fn seek(&self, offset: Time) -> fdo::Result<()> {
            let micros = offset.as_micros();
            let duration = Duration::from_micros(micros.unsigned_abs());
            let _ = self.action_tx.send(PlayerAction::Seek(duration));
            Ok(())
        }

        async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
            let micros = position.as_micros();
            if micros >= 0 {
                let duration = Duration::from_micros(micros as u64);
                let _ = self.action_tx.send(PlayerAction::SetPosition(duration));
            }
            Ok(())
        }

        async fn open_uri(&self, uri: String) -> fdo::Result<()> {
            let _ = self.action_tx.send(PlayerAction::OpenUri(uri));
            Ok(())
        }

        async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
            if self.is_playing.load(Ordering::Relaxed) {
                Ok(PlaybackStatus::Playing)
            } else if self
                .current_track
                .read()
                .map(|g| g.is_some())
                .unwrap_or(false)
            {
                Ok(PlaybackStatus::Paused)
            } else {
                Ok(PlaybackStatus::Stopped)
            }
        }

        async fn loop_status(&self) -> fdo::Result<LoopStatus> {
            if self.is_repeat.load(Ordering::Relaxed) {
                Ok(LoopStatus::Track)
            } else {
                Ok(LoopStatus::None)
            }
        }

        async fn set_loop_status(&self, loop_status: LoopStatus) -> ZbusResult<()> {
            let repeat = match loop_status {
                LoopStatus::None => false,
                LoopStatus::Track | LoopStatus::Playlist => true,
            };
            self.is_repeat.store(repeat, Ordering::Relaxed);
            let _ = self.action_tx.send(PlayerAction::SetRepeat(repeat));
            Ok(())
        }

        async fn rate(&self) -> fdo::Result<PlaybackRate> {
            Ok(1.0)
        }

        async fn set_rate(&self, _rate: PlaybackRate) -> ZbusResult<()> {
            Ok(())
        }

        async fn shuffle(&self) -> fdo::Result<bool> {
            Ok(self.is_shuffle.load(Ordering::Relaxed))
        }

        async fn set_shuffle(&self, shuffle: bool) -> ZbusResult<()> {
            self.is_shuffle.store(shuffle, Ordering::Relaxed);
            let _ = self.action_tx.send(PlayerAction::SetShuffle(shuffle));
            Ok(())
        }

        async fn metadata(&self) -> fdo::Result<Metadata> {
            let track_guard = self.current_track.read().ok();
            if let Some(Some(ref track)) = track_guard.as_deref() {
                let mut builder = Metadata::builder()
                    .title(&track.title)
                    .artist(vec![track.artist.clone()])
                    .album(&track.album)
                    .length(Time::from_millis(track.duration_ms as i64));

                let track_id_path = format!(
                    "/org/mpris/MediaPlayer2/Track/{}",
                    track
                        .file_path
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric())
                        .collect::<String>()
                );
                if let Ok(tid) = TrackId::try_from(track_id_path) {
                    builder = builder.trackid(tid);
                } else {
                    builder = builder.trackid(TrackId::NO_TRACK);
                }

                if let Ok(file_url) = Url::from_file_path(PathBuf::from(&track.file_path)) {
                    builder = builder.url(file_url.as_str());
                }

                // If cover thumbnail exists in cache, expose artUrl
                let thumb_path = core_engine::thumbnail_cache_dir().join(format!(
                    "{}.jpg",
                    core_engine::compute_thumbnail_cache_key(&PathBuf::from(&track.file_path), 500)
                ));
                if thumb_path.exists() {
                    if let Ok(art_url) = Url::from_file_path(&thumb_path) {
                        builder = builder.art_url(art_url.as_str());
                    }
                }

                Ok(builder.build())
            } else {
                Ok(Metadata::builder().trackid(TrackId::NO_TRACK).build())
            }
        }

        async fn volume(&self) -> fdo::Result<Volume> {
            Ok(self.get_volume())
        }

        async fn set_volume(&self, volume: Volume) -> ZbusResult<()> {
            self.set_volume_ratio(volume);
            let _ = self.action_tx.send(PlayerAction::SetVolume(volume as f32));
            Ok(())
        }

        async fn position(&self) -> fdo::Result<Time> {
            let ms = self.position_ms.load(Ordering::Relaxed);
            Ok(Time::from_millis(ms as i64))
        }

        async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
            Ok(1.0)
        }

        async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
            Ok(1.0)
        }

        async fn can_go_next(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_go_previous(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_play(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_pause(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_seek(&self) -> fdo::Result<bool> {
            Ok(true)
        }

        async fn can_control(&self) -> fdo::Result<bool> {
            Ok(true)
        }
    }

    enum MprisServerMsg {
        PropertiesChanged(Vec<Property>),
        Seeked(Time),
    }

    pub struct MprisPlugin {
        meta: PluginMetadata,
        inner: MprisInner,
        server_tx: Mutex<Option<Sender<MprisServerMsg>>>,
    }

    impl MprisPlugin {
        pub fn new(action_tx: Sender<PlayerAction>) -> Self {
            let meta = PluginMetadata {
                id: "builtin.mpris".to_string(),
                name: "MPRIS D-Bus Interface".to_string(),
                version: "1.0.0".to_string(),
                author: "vibecore".to_string(),
                description: "Exposes org.mpris.MediaPlayer2 player interface on Linux D-Bus"
                    .to_string(),
            };

            let inner = MprisInner::new(action_tx);
            let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<MprisServerMsg>();

            let inner_clone = inner.clone();
            std::thread::Builder::new()
                .name("mpris-worker".to_string())
                .spawn(move || {
                    async_io::block_on(async move {
                        let server = match Server::new("vibecore", inner_clone).await {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Failed to initialize MPRIS D-Bus server: {}", e);
                                return;
                            }
                        };
                        info!("MPRIS D-Bus server successfully registered on org.mpris.MediaPlayer2.vibecore");

                        // Worker loop handling signals and properties changed
                        loop {
                            match msg_rx.recv() {
                                Ok(MprisServerMsg::PropertiesChanged(props)) => {
                                    if let Err(e) = server.properties_changed(props).await {
                                        error!("MPRIS properties_changed error: {}", e);
                                    }
                                }
                                Ok(MprisServerMsg::Seeked(pos)) => {
                                    if let Err(e) = server.emit(Signal::Seeked { position: pos }).await {
                                        error!("MPRIS seeked signal error: {}", e);
                                    }
                                }
                                Err(_) => {
                                    info!("MPRIS worker channel closed, shutting down MPRIS server");
                                    break;
                                }
                            }
                        }
                    });
                })
                .ok();

            Self {
                meta,
                inner,
                server_tx: Mutex::new(Some(msg_tx)),
            }
        }

        fn send_msg(&self, msg: MprisServerMsg) {
            if let Ok(guard) = self.server_tx.lock() {
                if let Some(ref tx) = *guard {
                    let _ = tx.send(msg);
                }
            }
        }
    }

    impl Plugin for MprisPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }

        fn on_track_change(&self, track: &TrackMetadata) -> Result<(), String> {
            if let Ok(mut g) = self.inner.current_track.write() {
                *g = Some(track.clone());
            }
            self.inner.position_ms.store(0, Ordering::Relaxed);

            // Construct new metadata
            let mut builder = Metadata::builder()
                .title(&track.title)
                .artist(vec![track.artist.clone()])
                .album(&track.album)
                .length(Time::from_millis(track.duration_ms as i64));

            let track_id_path = format!(
                "/org/mpris/MediaPlayer2/Track/{}",
                track
                    .file_path
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
            );
            if let Ok(tid) = TrackId::try_from(track_id_path) {
                builder = builder.trackid(tid);
            } else {
                builder = builder.trackid(TrackId::NO_TRACK);
            }

            if let Ok(file_url) = Url::from_file_path(PathBuf::from(&track.file_path)) {
                builder = builder.url(file_url.as_str());
            }

            let thumb_path = core_engine::thumbnail_cache_dir().join(format!(
                "{}.jpg",
                core_engine::compute_thumbnail_cache_key(&PathBuf::from(&track.file_path), 500)
            ));
            if thumb_path.exists() {
                if let Ok(art_url) = Url::from_file_path(&thumb_path) {
                    builder = builder.art_url(art_url.as_str());
                }
            }

            self.send_msg(MprisServerMsg::PropertiesChanged(vec![
                Property::Metadata(builder.build()),
                Property::PlaybackStatus(PlaybackStatus::Playing),
            ]));
            Ok(())
        }

        fn on_playback_end(&self) -> Result<(), String> {
            self.inner.is_playing.store(false, Ordering::Relaxed);
            self.send_msg(MprisServerMsg::PropertiesChanged(vec![
                Property::PlaybackStatus(PlaybackStatus::Stopped),
            ]));
            Ok(())
        }

        fn on_playback_status_change(&self, is_playing: bool) -> Result<(), String> {
            self.inner.is_playing.store(is_playing, Ordering::Relaxed);
            let status = if is_playing {
                PlaybackStatus::Playing
            } else if self
                .inner
                .current_track
                .read()
                .map(|g| g.is_some())
                .unwrap_or(false)
            {
                PlaybackStatus::Paused
            } else {
                PlaybackStatus::Stopped
            };
            self.send_msg(MprisServerMsg::PropertiesChanged(vec![
                Property::PlaybackStatus(status),
            ]));
            Ok(())
        }

        fn on_position_change(&self, position_ms: u64) -> Result<(), String> {
            self.inner.position_ms.store(position_ms, Ordering::Relaxed);
            Ok(())
        }

        fn on_seeked(&self, position_ms: u64) -> Result<(), String> {
            self.inner.position_ms.store(position_ms, Ordering::Relaxed);
            self.send_msg(MprisServerMsg::Seeked(Time::from_millis(
                position_ms as i64,
            )));
            Ok(())
        }

        fn on_volume_change(&self, volume: f32) -> Result<(), String> {
            self.inner.set_volume_ratio(volume as f64);
            self.send_msg(MprisServerMsg::PropertiesChanged(vec![Property::Volume(
                volume as f64,
            )]));
            Ok(())
        }

        fn on_repeat_change(&self, is_repeat: bool) -> Result<(), String> {
            self.inner.is_repeat.store(is_repeat, Ordering::Relaxed);
            let loop_status = if is_repeat {
                LoopStatus::Track
            } else {
                LoopStatus::None
            };
            self.send_msg(MprisServerMsg::PropertiesChanged(vec![
                Property::LoopStatus(loop_status),
            ]));
            Ok(())
        }

        fn on_shuffle_change(&self, is_shuffle: bool) -> Result<(), String> {
            self.inner.is_shuffle.store(is_shuffle, Ordering::Relaxed);
            self.send_msg(MprisServerMsg::PropertiesChanged(vec![Property::Shuffle(
                is_shuffle,
            )]));
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::MprisPlugin;
