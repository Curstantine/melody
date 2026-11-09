#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, RwLock,
    };
    use std::time::Duration;

    use crossbeam_channel::{Receiver, Sender};
    use tracing::{error, info};
    use windows::{
        core::HSTRING,
        Foundation::{TimeSpan, TypedEventHandler},
        Media::*,
        Storage::{StorageFile, Streams::RandomAccessStreamReference},
        Win32::{
            Foundation::HWND,
            System::{
                Com::{CoInitializeEx, COINIT_MULTITHREADED},
                WinRT::ISystemMediaTransportControlsInterop,
            },
        },
    };

    use crate::{
        manager::Plugin,
        types::{PlayerAction, PluginMetadata, TrackMetadata},
    };

    #[derive(Debug)]
    enum WindowsMediaMsg {
        AttachWindow(isize),
        TrackChanged(TrackMetadata),
        PlaybackStatusChanged(bool),
        PlaybackEnded,
        Seeked(u64),
        RepeatChanged(bool),
        ShuffleChanged(bool),
    }

    struct WindowsMediaInner {
        current_track: Arc<RwLock<Option<TrackMetadata>>>,
        is_playing: Arc<AtomicBool>,
        position_ms: Arc<AtomicU64>,
        is_shuffle: Arc<AtomicBool>,
        is_repeat: Arc<AtomicBool>,
    }

    impl WindowsMediaInner {
        fn new() -> Self {
            Self {
                current_track: Arc::new(RwLock::new(None)),
                is_playing: Arc::new(AtomicBool::new(false)),
                position_ms: Arc::new(AtomicU64::new(0)),
                is_shuffle: Arc::new(AtomicBool::new(false)),
                is_repeat: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    pub struct WindowsMediaPlugin {
        meta: PluginMetadata,
        inner: Arc<WindowsMediaInner>,
        worker_tx: Sender<WindowsMediaMsg>,
    }

    impl WindowsMediaPlugin {
        pub fn new(action_tx: Sender<PlayerAction>) -> Self {
            let meta = PluginMetadata {
                id: "builtin.windows_media".to_string(),
                name: "Windows Media Controls (SMTC)".to_string(),
                version: "1.0.0".to_string(),
                author: "melody".to_string(),
                description:
                    "Integrates with Windows System Media Transport Controls (SMTC) and media keys"
                        .to_string(),
            };

            let inner = Arc::new(WindowsMediaInner::new());
            let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<WindowsMediaMsg>();

            let inner_worker = inner.clone();
            std::thread::Builder::new()
                .name("windows-media-worker".to_string())
                .spawn(move || {
                    run_worker_loop(inner_worker, msg_rx, action_tx);
                })
                .ok();

            Self {
                meta,
                inner,
                worker_tx: msg_tx,
            }
        }

        pub fn attach_window(&self, hwnd: isize) {
            let _ = self.worker_tx.send(WindowsMediaMsg::AttachWindow(hwnd));
        }
    }

    struct SmtcState {
        controls: SystemMediaTransportControls,
        updater: SystemMediaTransportControlsDisplayUpdater,
        timeline: SystemMediaTransportControlsTimelineProperties,
        button_token: i64,
        position_token: i64,
        repeat_token: i64,
        shuffle_token: i64,
        attached_hwnd: isize,
    }

    impl Drop for SmtcState {
        fn drop(&mut self) {
            let _ = self.controls.SetIsEnabled(false);
            let _ = self.controls.RemoveButtonPressed(self.button_token);
            let _ = self
                .controls
                .RemovePlaybackPositionChangeRequested(self.position_token);
            let _ = self
                .controls
                .RemoveAutoRepeatModeChangeRequested(self.repeat_token);
            let _ = self
                .controls
                .RemoveShuffleEnabledChangeRequested(self.shuffle_token);
        }
    }

    fn run_worker_loop(
        inner: Arc<WindowsMediaInner>,
        rx: Receiver<WindowsMediaMsg>,
        action_tx: Sender<PlayerAction>,
    ) {
        unsafe {
            if let Err(e) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
                error!(
                    "Failed to initialize COM MTA on windows-media-worker: {}",
                    e
                );
                return;
            }
        }

        let mut smtc_state: Option<SmtcState> = None;
        let mut cached_track: Option<TrackMetadata> = None;
        let mut cached_playing = false;
        let mut cached_repeat = false;
        let mut cached_shuffle = false;
        let mut cached_position_ms: u64 = 0;

        while let Ok(msg) = rx.recv() {
            match msg {
                WindowsMediaMsg::AttachWindow(hwnd) => {
                    if let Some(ref state) = smtc_state {
                        if state.attached_hwnd == hwnd {
                            continue;
                        }
                    }

                    // Drop previous state if any
                    smtc_state = None;

                    match setup_smtc(hwnd, &action_tx, &inner) {
                        Ok(mut state) => {
                            info!(
                                "Windows SMTC successfully initialized and bound to HWND: {:#x}",
                                hwnd
                            );

                            // Apply current states
                            let _ = state.controls.SetAutoRepeatMode(if cached_repeat {
                                MediaPlaybackAutoRepeatMode::Track
                            } else {
                                MediaPlaybackAutoRepeatMode::None
                            });
                            let _ = state.controls.SetShuffleEnabled(cached_shuffle);

                            if let Some(ref track) = cached_track {
                                update_metadata_and_timeline(
                                    &mut state,
                                    track,
                                    cached_position_ms,
                                    cached_playing,
                                );
                            } else {
                                let _ = state
                                    .controls
                                    .SetPlaybackStatus(MediaPlaybackStatus::Stopped);
                            }

                            smtc_state = Some(state);
                        }
                        Err(e) => {
                            error!(
                                "Failed to initialize Windows SMTC for HWND {:#x}: {}",
                                hwnd, e
                            );
                        }
                    }
                }

                WindowsMediaMsg::TrackChanged(track) => {
                    cached_position_ms = 0;
                    cached_playing = true;
                    cached_track = Some(track.clone());

                    if let Some(ref mut state) = smtc_state {
                        update_metadata_and_timeline(state, &track, 0, true);
                    }
                }

                WindowsMediaMsg::PlaybackStatusChanged(is_playing) => {
                    cached_playing = is_playing;
                    if let Some(ref mut state) = smtc_state {
                        let status = if is_playing {
                            MediaPlaybackStatus::Playing
                        } else if cached_track.is_some() {
                            MediaPlaybackStatus::Paused
                        } else {
                            MediaPlaybackStatus::Stopped
                        };
                        let _ = state.controls.SetPlaybackStatus(status);

                        // Refresh timeline position
                        let cur_pos = inner.position_ms.load(Ordering::Relaxed);
                        cached_position_ms = cur_pos;
                        let span = TimeSpan {
                            Duration: (cur_pos as i64) * 10_000,
                        };
                        let _ = state.timeline.SetPosition(span);
                        let _ = state.controls.UpdateTimelineProperties(&state.timeline);
                    }
                }

                WindowsMediaMsg::PlaybackEnded => {
                    cached_playing = false;
                    if let Some(ref mut state) = smtc_state {
                        let _ = state
                            .controls
                            .SetPlaybackStatus(MediaPlaybackStatus::Stopped);
                    }
                }

                WindowsMediaMsg::Seeked(position_ms) => {
                    cached_position_ms = position_ms;
                    if let Some(ref mut state) = smtc_state {
                        let span = TimeSpan {
                            Duration: (position_ms as i64) * 10_000,
                        };
                        let _ = state.timeline.SetPosition(span);
                        let _ = state.controls.UpdateTimelineProperties(&state.timeline);
                    }
                }

                WindowsMediaMsg::RepeatChanged(is_repeat) => {
                    cached_repeat = is_repeat;
                    if let Some(ref mut state) = smtc_state {
                        let mode = if is_repeat {
                            MediaPlaybackAutoRepeatMode::Track
                        } else {
                            MediaPlaybackAutoRepeatMode::None
                        };
                        let _ = state.controls.SetAutoRepeatMode(mode);
                    }
                }

                WindowsMediaMsg::ShuffleChanged(is_shuffle) => {
                    cached_shuffle = is_shuffle;
                    if let Some(ref mut state) = smtc_state {
                        let _ = state.controls.SetShuffleEnabled(is_shuffle);
                    }
                }
            }
        }

        info!("Windows media controls worker thread exiting");
    }

    fn setup_smtc(
        hwnd: isize,
        action_tx: &Sender<PlayerAction>,
        inner: &Arc<WindowsMediaInner>,
    ) -> windows::core::Result<SmtcState> {
        let interop: ISystemMediaTransportControlsInterop = windows::core::factory::<
            SystemMediaTransportControls,
            ISystemMediaTransportControlsInterop,
        >()?;

        let controls: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(HWND(hwnd as *mut c_void)) }?;

        controls.SetIsEnabled(true)?;
        controls.SetIsPlayEnabled(true)?;
        controls.SetIsPauseEnabled(true)?;
        controls.SetIsStopEnabled(true)?;
        controls.SetIsNextEnabled(true)?;
        controls.SetIsPreviousEnabled(true)?;
        controls.SetIsFastForwardEnabled(true)?;
        controls.SetIsRewindEnabled(true)?;

        let updater = controls.DisplayUpdater()?;
        updater.SetType(MediaPlaybackType::Music)?;

        let timeline = SystemMediaTransportControlsTimelineProperties::new()?;

        // Attach Button Handler
        let button_handler = TypedEventHandler::new({
            let action_tx = action_tx.clone();
            let inner = inner.clone();
            move |_,
                  args: windows::core::Ref<
                '_,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >| {
                let args = args.ok()?;
                let button = args.Button()?;
                match button {
                    SystemMediaTransportControlsButton::Play => {
                        let _ = action_tx.send(PlayerAction::Play);
                    }
                    SystemMediaTransportControlsButton::Pause => {
                        let _ = action_tx.send(PlayerAction::Pause);
                    }
                    SystemMediaTransportControlsButton::Stop => {
                        let _ = action_tx.send(PlayerAction::Stop);
                    }
                    SystemMediaTransportControlsButton::Next => {
                        let _ = action_tx.send(PlayerAction::Next);
                    }
                    SystemMediaTransportControlsButton::Previous => {
                        let _ = action_tx.send(PlayerAction::Previous);
                    }
                    SystemMediaTransportControlsButton::FastForward => {
                        let _ = action_tx.send(PlayerAction::Seek(Duration::from_secs(5)));
                    }
                    SystemMediaTransportControlsButton::Rewind => {
                        let cur_ms = inner.position_ms.load(Ordering::Relaxed);
                        let target_ms = cur_ms.saturating_sub(5000);
                        let _ = action_tx
                            .send(PlayerAction::SetPosition(Duration::from_millis(target_ms)));
                    }
                    _ => {}
                }
                Ok(())
            }
        });
        let button_token = controls.ButtonPressed(&button_handler)?;

        // Attach Seek / Timeline Position Change Handler
        let position_handler = TypedEventHandler::new({
            let action_tx = action_tx.clone();
            move |_, args: windows::core::Ref<'_, PlaybackPositionChangeRequestedEventArgs>| {
                let args = args.ok()?;
                let pos = args.RequestedPlaybackPosition()?;
                let nanos = (pos.Duration.max(0) as u64) * 100;
                let _ = action_tx.send(PlayerAction::SetPosition(Duration::from_nanos(nanos)));
                Ok(())
            }
        });
        let position_token = controls.PlaybackPositionChangeRequested(&position_handler)?;

        // Attach Repeat Handler
        let repeat_handler = TypedEventHandler::new({
            let action_tx = action_tx.clone();
            move |_, args: windows::core::Ref<'_, AutoRepeatModeChangeRequestedEventArgs>| {
                let args = args.ok()?;
                let mode = args.RequestedAutoRepeatMode()?;
                let repeat = mode != MediaPlaybackAutoRepeatMode::None;
                let _ = action_tx.send(PlayerAction::SetRepeat(repeat));
                Ok(())
            }
        });
        let repeat_token = controls.AutoRepeatModeChangeRequested(&repeat_handler)?;

        // Attach Shuffle Handler
        let shuffle_handler = TypedEventHandler::new({
            let action_tx = action_tx.clone();
            move |_, args: windows::core::Ref<'_, ShuffleEnabledChangeRequestedEventArgs>| {
                let args = args.ok()?;
                let shuffle = args.RequestedShuffleEnabled()?;
                let _ = action_tx.send(PlayerAction::SetShuffle(shuffle));
                Ok(())
            }
        });
        let shuffle_token = controls.ShuffleEnabledChangeRequested(&shuffle_handler)?;

        Ok(SmtcState {
            controls,
            updater,
            timeline,
            button_token,
            position_token,
            repeat_token,
            shuffle_token,
            attached_hwnd: hwnd,
        })
    }

    fn update_metadata_and_timeline(
        state: &mut SmtcState,
        track: &TrackMetadata,
        position_ms: u64,
        is_playing: bool,
    ) {
        if let Ok(props) = state.updater.MusicProperties() {
            let _ = props.SetTitle(&HSTRING::from(&track.title));
            let _ = props.SetArtist(&HSTRING::from(&track.artist));
            let _ = props.SetAlbumTitle(&HSTRING::from(&track.album));
        }

        // Thumbnail resolution: probe thumbnail cache
        let path = PathBuf::from(&track.file_path);
        // Ensure thumbnail is available in cache
        let _ = thumbnail::get_or_create_thumbnail(&path, 500);
        let key = thumbnail::compute_thumbnail_cache_key(&path, 500);
        let thumb_file = paths::thumbnail_cache_dir().join(format!("{}.jpg", key));
        if thumb_file.is_file() {
            let clean_path = thumb_file.to_string_lossy().to_string();
            if let Ok(file_async) = StorageFile::GetFileFromPathAsync(&HSTRING::from(&clean_path)) {
                if let Ok(file) = file_async.join() {
                    if let Ok(stream) = RandomAccessStreamReference::CreateFromFile(&file) {
                        let _ = state.updater.SetThumbnail(&stream);
                    }
                }
            }
        }

        let _ = state.updater.Update();

        // Update Timeline
        let duration_span = TimeSpan {
            Duration: (track.duration_ms as i64) * 10_000,
        };
        let position_span = TimeSpan {
            Duration: (position_ms as i64) * 10_000,
        };
        let _ = state.timeline.SetStartTime(TimeSpan::default());
        let _ = state.timeline.SetMinSeekTime(TimeSpan::default());
        let _ = state.timeline.SetEndTime(duration_span);
        let _ = state.timeline.SetMaxSeekTime(duration_span);
        let _ = state.timeline.SetPosition(position_span);
        let _ = state.controls.UpdateTimelineProperties(&state.timeline);

        let status = if is_playing {
            MediaPlaybackStatus::Playing
        } else {
            MediaPlaybackStatus::Paused
        };
        let _ = state.controls.SetPlaybackStatus(status);
    }

    impl Plugin for WindowsMediaPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.meta
        }

        fn on_track_change(&self, track: &TrackMetadata) -> Result<(), String> {
            if let Ok(mut g) = self.inner.current_track.write() {
                *g = Some(track.clone());
            }
            self.inner.position_ms.store(0, Ordering::Relaxed);
            let _ = self
                .worker_tx
                .send(WindowsMediaMsg::TrackChanged(track.clone()));
            Ok(())
        }

        fn on_playback_end(&self) -> Result<(), String> {
            self.inner.is_playing.store(false, Ordering::Relaxed);
            let _ = self.worker_tx.send(WindowsMediaMsg::PlaybackEnded);
            Ok(())
        }

        fn on_playback_status_change(&self, is_playing: bool) -> Result<(), String> {
            self.inner.is_playing.store(is_playing, Ordering::Relaxed);
            let _ = self
                .worker_tx
                .send(WindowsMediaMsg::PlaybackStatusChanged(is_playing));
            Ok(())
        }

        fn on_position_change(&self, position_ms: u64) -> Result<(), String> {
            self.inner.position_ms.store(position_ms, Ordering::Relaxed);
            Ok(())
        }

        fn on_seeked(&self, position_ms: u64) -> Result<(), String> {
            self.inner.position_ms.store(position_ms, Ordering::Relaxed);
            let _ = self.worker_tx.send(WindowsMediaMsg::Seeked(position_ms));
            Ok(())
        }

        fn on_repeat_change(&self, is_repeat: bool) -> Result<(), String> {
            self.inner.is_repeat.store(is_repeat, Ordering::Relaxed);
            let _ = self
                .worker_tx
                .send(WindowsMediaMsg::RepeatChanged(is_repeat));
            Ok(())
        }

        fn on_shuffle_change(&self, is_shuffle: bool) -> Result<(), String> {
            self.inner.is_shuffle.store(is_shuffle, Ordering::Relaxed);
            let _ = self
                .worker_tx
                .send(WindowsMediaMsg::ShuffleChanged(is_shuffle));
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub use win::WindowsMediaPlugin;

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::manager::Plugin;
    use crate::types::TrackMetadata;

    #[test]
    fn test_windows_media_plugin_instantiation_and_metadata() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let plugin = WindowsMediaPlugin::new(tx);
        assert_eq!(plugin.metadata().id, "builtin.windows_media");

        let meta = TrackMetadata {
            title: "Test Track".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            duration_ms: 240000,
            file_path: "C:\\music\\test.mp3".to_string(),
        };

        assert!(plugin.on_track_change(&meta).is_ok());
        assert!(plugin.on_playback_status_change(true).is_ok());
        assert!(plugin.on_position_change(5000).is_ok());
        assert!(plugin.on_seeked(6000).is_ok());
        assert!(plugin.on_volume_change(0.8).is_ok());
        assert!(plugin.on_repeat_change(true).is_ok());
        assert!(plugin.on_shuffle_change(true).is_ok());
        assert!(plugin.on_playback_end().is_ok());
    }
}
