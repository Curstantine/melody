use std::{path::PathBuf, time::Duration};

use iced::widget::operation::scrollable::{scroll_to, AbsoluteOffset};
use iced::widget::operation::Animation;
use iced::Task;
use model::TrackRow;
use plugin::{PlayerAction, TrackMetadata};
use tracing::info;

use crate::message::Message;
use config::ViewTab;

use super::*;

impl App {
    pub(super) fn handle_playback_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AudioTick => {
                let status = self.audio_controller.status();
                let was_playing = self.is_playing;
                self.is_playing = status.is_playing();

                if was_playing != self.is_playing {
                    self.plugin_manager
                        .on_playback_status_change(self.is_playing);
                }

                if status.duration_ms() > 0 {
                    self.duration_ms = status.duration_ms();
                }

                if !self.player_bar.is_seeking {
                    let old_pos = self.position_ms;
                    let raw_pos = status.position_ms();
                    self.position_ms = if self.duration_ms > 0 {
                        raw_pos.min(self.duration_ms)
                    } else {
                        raw_pos
                    };
                    if self.is_playing && self.position_ms != old_pos {
                        self.plugin_manager.on_position_change(self.position_ms);
                    }
                }
                self.player_bar
                    .update_position(self.position_ms, self.duration_ms);

                // Check for gapless track transition
                if status.take_gapless_transition() {
                    info!("Gapless transition occurred");
                    self.position_ms = 0;
                    if status.duration_ms() > 0 {
                        self.duration_ms = status.duration_ms();
                    }
                    self.plugin_manager.on_playback_end();

                    if self.repeat_mode == model::RepeatMode::One {
                        if let Some(track) = &self.current_track {
                            self.plugin_manager.on_track_change(&TrackMetadata {
                                title: track.title.clone(),
                                artist: track.artist.clone(),
                                album: track.album.clone(),
                                duration_ms: track.duration_ms,
                                file_path: track.file_path.clone(),
                            });
                        }
                        self.queue_next_gapless_track();
                        return Task::none();
                    }

                    if let Some(next_track) = self.pending_gapless_next.take() {
                        if let Some(queue) = self.queues.active_queue_mut() {
                            if let Some(pos) =
                                queue.tracks.iter().position(|t| t.id == next_track.id)
                            {
                                queue.set_current(pos);
                            }
                            self.queues.touch_active_summary();
                        }

                        let track_id = next_track.id;
                        let duration_ms = next_track.duration_ms;
                        let file_path = next_track.file_path;
                        let title = next_track.title;
                        let artist = next_track.artist.to_string();
                        let album = next_track.album.to_string();

                        self.current_cover = None;
                        self.current_now_playing_cover = None;
                        self.current_blurred_cover = None;
                        self.current_tech_info = None;
                        self.now_playing.set_lyrics(None);

                        self.plugin_manager.on_track_change(&TrackMetadata {
                            title: title.clone(),
                            artist: artist.clone(),
                            album: album.clone(),
                            duration_ms,
                            file_path: file_path.clone(),
                        });

                        let path = PathBuf::from(&file_path);
                        let load_assets = self.load_track_assets_task(
                            track_id,
                            path,
                            artist.clone(),
                            title.clone(),
                            duration_ms,
                        );

                        let track = Track {
                            id: track_id,
                            file_path,
                            title,
                            artist,
                            album_artist: None,
                            album,
                            genre: None,
                            duration_ms,
                            track_num: None,
                            disc_num: None,
                            year: None,
                            composer: None,
                            label: None,
                            musicbrainz_track_id: None,
                            musicbrainz_artist_id: None,
                            musicbrainz_album_id: None,
                            added_timestamp: 0,
                        };

                        self.current_track = Some(track);
                        self.player_bar.update_track(self.current_track.as_ref());
                        self.sync_now_playing();
                        self.current_track_id = Some(track_id);
                        self.duration_ms = duration_ms;

                        let persist = self.persist_active_current_task();
                        let snap = self.snap_queue_to_current();
                        let db = self.db.clone();
                        let fetch_details = Task::perform(
                            async move { db.get_track_by_id(track_id).await.unwrap_or(None) },
                            |res| Message::TrackDetailsLoaded(Box::new(res)),
                        );

                        self.queue_next_gapless_track();
                        let save_state = self.save_state_debounced();

                        return Task::batch([
                            load_assets,
                            persist,
                            snap,
                            fetch_details,
                            save_state,
                        ]);
                    } else {
                        return self.update(Message::NextTrack);
                    }
                }

                // Check if track ended
                if status.take_ended() {
                    info!("Playback ended, triggering next track");
                    if self.duration_ms > 0 {
                        self.position_ms = self.duration_ms;
                    }
                    self.plugin_manager.on_playback_end();
                    if self.repeat_mode == model::RepeatMode::One {
                        self.audio_controller.seek(Duration::from_millis(0));
                        self.position_ms = 0;
                        self.audio_controller.play();
                        self.queue_next_gapless_track();
                        return Task::none();
                    }
                    return self.update(Message::NextTrack);
                }

                // Drain any incoming media control actions (MPRIS on Linux, SMTC on Windows)
                let mut media_action = None;
                if let Some(ref rx) = self.media_controls_rx {
                    while let Ok(action) = rx.try_recv() {
                        media_action = Some(action);
                    }
                }
                if let Some(action) = media_action {
                    return self.update(Message::MediaControlAction(action));
                }

                // Drain any incoming library watcher events
                let mut watcher_paths = Vec::new();
                if let Some(ref rx) = self.watcher_rx {
                    while let Ok(paths) = rx.try_recv() {
                        watcher_paths.extend(paths);
                    }
                }
                if !watcher_paths.is_empty() {
                    if self.is_scanning {
                        self.pending_watcher_paths.extend(watcher_paths);
                    } else {
                        self.is_scanning = true;
                        self.scan_progress = None;
                        let db = self.db.clone();
                        let paths = watcher_paths;
                        return Task::stream(iced::stream::channel(
                            100,
                            async move |mut sender| {
                                let mut cb_sender = sender.clone();
                                let summary = db
                                    .scan_new_files(
                                        &paths,
                                        Some(move |current, total| {
                                            let _ = cb_sender
                                                .try_send(Message::ScanProgress { current, total });
                                        }),
                                    )
                                    .await
                                    .unwrap_or_default();
                                let _ = sender.send(Message::ScanFinished(summary)).await;
                            },
                        ));
                    }
                }

                if self.tray.is_some() {
                    let tray_actions = crate::tray::TrayController::poll_actions();
                    if !tray_actions.is_empty() {
                        let tasks: Vec<Task<Message>> = tray_actions
                            .into_iter()
                            .map(|action| Task::done(Message::TrayAction(action)))
                            .collect();
                        return Task::batch(tasks);
                    }
                }

                if self.main_window_id.is_some() && self.active_tab == ViewTab::Lyrics {
                    if let Some(active_idx) =
                        self.now_playing.update_active_lyric_line(self.position_ms)
                    {
                        let target_y = (active_idx as f32
                            * NowPlayingView::line_stride(self.fonts.lyrics_size)
                            - 90.0)
                            .max(0.0);
                        return scroll_to(
                            iced::widget::Id::new("lyrics_scroll"),
                            AbsoluteOffset {
                                x: 0.0,
                                y: target_y,
                            },
                            Animation::Instant,
                        );
                    }
                }

                Task::none()
            }

            Message::PlayTrack(track_id) => {
                let db = self.db.clone();
                Task::perform(
                    async move { db.get_track_by_id(track_id).await.unwrap_or(None) },
                    |res| Message::TrackResolvedForPlayback(Box::new(res)),
                )
            }

            Message::TrackResolvedForPlayback(maybe_track) => {
                if let Some(track) = *maybe_track {
                    info!("Playing track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    // Start playback immediately; the expensive visual/metadata
                    // work is dispatched to a blocking worker below so the UI
                    // thread never stalls on disk I/O, image decoding or FFmpeg.
                    self.audio_controller.load(path.clone(), true);
                    let track_id = track.id;
                    let duration_ms = track.duration_ms;
                    let title = track.title.clone();
                    let artist = track.artist.clone();
                    let album = track.album.clone();
                    let file_path = track.file_path.clone();

                    self.current_track = Some(track);
                    self.player_bar.update_track(self.current_track.as_ref());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = duration_ms;

                    // Clear stale assets from the previous track right away so we
                    // never render a mismatched cover while the new one loads.
                    self.current_cover = None;
                    self.current_now_playing_cover = None;
                    self.current_blurred_cover = None;
                    self.current_tech_info = None;
                    self.now_playing.set_lyrics(None);
                    self.sync_now_playing();

                    // Notify plugins synchronously (cheap state updates).
                    self.plugin_manager.on_track_change(&TrackMetadata {
                        title: title.clone(),
                        artist: artist.clone(),
                        album,
                        duration_ms,
                        file_path,
                    });

                    self.queue_next_gapless_track();
                    let load_assets =
                        self.load_track_assets_task(track_id, path, artist, title, duration_ms);
                    let save_state = self.save_state_debounced();
                    Task::batch([load_assets, save_state])
                } else {
                    Task::none()
                }
            }

            Message::TrackResolvedForRestore(maybe_track) => {
                if let Some(track) = *maybe_track {
                    info!("Restoring player track: {} (ID {})", track.title, track.id);
                    let path = PathBuf::from(&track.file_path);

                    self.audio_controller.load(path.clone(), false);
                    let track_id = track.id;
                    let duration_ms = track.duration_ms;
                    let title = track.title.clone();
                    let artist = track.artist.clone();

                    self.current_track = Some(track);
                    self.player_bar.update_track(self.current_track.as_ref());
                    self.current_track_id = Some(track_id);
                    self.position_ms = 0;
                    self.duration_ms = duration_ms;

                    self.current_cover = None;
                    self.current_now_playing_cover = None;
                    self.current_blurred_cover = None;
                    self.current_tech_info = None;
                    self.now_playing.set_lyrics(None);
                    self.sync_now_playing();

                    self.queue_next_gapless_track();
                    self.load_track_assets_task(track_id, path, artist, title, duration_ms)
                } else {
                    Task::none()
                }
            }

            Message::TrackDetailsLoaded(maybe_track) => {
                if let Some(track) = *maybe_track {
                    if self.current_track_id == Some(track.id) {
                        self.current_track = Some(track);
                        self.player_bar.update_track(self.current_track.as_ref());
                        self.sync_now_playing();
                    }
                }
                Task::none()
            }

            Message::PlaybackAssetsLoaded {
                track_id,
                cover,
                now_playing_cover,
                blurred,
                tech_info,
                lyrics,
            } => {
                // Drop assets that belong to a track the user has since skipped.
                if self.current_track_id != Some(track_id) {
                    return Task::none();
                }

                self.current_cover = cover.map(image::Handle::from_bytes);
                self.current_now_playing_cover = if self.active_tab == ViewTab::Lyrics {
                    now_playing_cover.map(image::Handle::from_bytes)
                } else {
                    None
                };
                self.current_blurred_cover = blurred.map(image::Handle::from_bytes);
                self.current_tech_info = tech_info;
                self.now_playing.set_lyrics(lyrics);
                self.sync_now_playing();
                Task::none()
            }

            Message::NowPlayingCoverLoaded {
                track_id,
                cover,
                blurred,
            } => {
                if self.current_track_id == Some(track_id) && self.active_tab == ViewTab::Lyrics {
                    self.current_now_playing_cover = cover.map(image::Handle::from_bytes);
                    self.current_blurred_cover = blurred.map(image::Handle::from_bytes);
                }
                Task::none()
            }

            Message::TogglePlayPause => {
                self.audio_controller.toggle_play_pause();
                Task::none()
            }

            Message::Stop => {
                self.pending_gapless_next = None;
                self.audio_controller.stop();
                self.position_ms = 0;
                Task::none()
            }

            Message::NextTrack => {
                if self.queues.active.is_some() {
                    if let Some(id) = self.step_active_queue(true) {
                        self.queues.touch_active_summary();
                        let play = self.update(Message::PlayTrack(id));
                        return Task::batch([
                            play,
                            self.persist_active_current_task(),
                            self.snap_queue_to_current(),
                        ]);
                    }
                }

                if let Some(next) = self.get_next_track() {
                    return self.update(Message::PlayTrack(next.id));
                }
                Task::none()
            }

            Message::ToggleShuffle => {
                self.is_shuffle = !self.is_shuffle;
                info!("Shuffle toggled: {}", self.is_shuffle);
                self.plugin_manager.on_shuffle_change(self.is_shuffle);
                self.queue_next_gapless_track();
                self.save_state_task()
            }

            Message::ToggleRepeat => {
                let next_mode = self.repeat_mode.next();
                self.update(Message::SetRepeatMode(next_mode))
            }

            Message::SetRepeatMode(mode) => {
                if self.repeat_mode != mode {
                    self.repeat_mode = mode;
                    info!("Repeat mode set: {:?}", self.repeat_mode);
                    self.plugin_manager.on_repeat_change(self.repeat_mode);
                    self.queue_next_gapless_track();
                    return self.save_state_task();
                }
                Task::none()
            }

            Message::SetOutputDevice(device) => {
                if self.output_device != device {
                    self.output_device = device.clone();
                    info!("Audio output device changed: {:?}", self.output_device);
                    self.audio_controller.set_output_device(device);
                    return self.save_config_task();
                }
                Task::none()
            }

            Message::RefreshOutputDevices => {
                self.available_output_devices = audio::list_output_devices();
                Task::none()
            }

            Message::ToggleGapless(enabled) => {
                self.gapless = enabled;
                info!("Gapless playback toggled: {}", self.gapless);
                self.queue_next_gapless_track();
                self.save_config_task()
            }

            Message::SetResamplingFilter(filter) => {
                self.resampling_config.filter = filter;
                info!("Resampling filter changed: {:?}", filter);
                self.audio_controller
                    .set_resampling(self.resampling_config.clone());
                self.save_config_task()
            }

            Message::SetResamplingQuality(quality) => {
                self.resampling_config.quality = quality;
                info!("Resampling quality changed: {:?}", quality);
                self.audio_controller
                    .set_resampling(self.resampling_config.clone());
                self.save_config_task()
            }

            Message::ToggleResamplingLinearInterp(enabled) => {
                self.resampling_config.linear_interp = enabled;
                info!("Resampling linear interpolation toggled: {}", enabled);
                self.audio_controller
                    .set_resampling(self.resampling_config.clone());
                self.save_config_task()
            }

            Message::PreviousTrack => {
                // If more than 3 seconds in, restart track
                if self.position_ms > 3000 {
                    self.audio_controller.seek(Duration::from_millis(0));
                    self.position_ms = 0;
                    return Task::none();
                }

                if self.queues.active.is_some() {
                    if let Some(id) = self.step_active_queue(false) {
                        self.queues.touch_active_summary();
                        let play = self.update(Message::PlayTrack(id));
                        return Task::batch([
                            play,
                            self.persist_active_current_task(),
                            self.snap_queue_to_current(),
                        ]);
                    }
                }

                if let Some(cur_id) = self.current_track_id {
                    if let Some(prev) = self.library.get_previous_row(cur_id) {
                        return self.update(Message::PlayTrack(prev.id));
                    }
                }
                Task::none()
            }

            Message::Seek(ms) => {
                let clamped = if self.duration_ms > 0 {
                    ms.min(self.duration_ms as f32)
                } else {
                    ms
                };
                self.player_bar.on_seek(clamped);
                Task::none()
            }

            Message::SeekFinished => {
                let seek_pos = self.player_bar.on_seek_finished();
                let clamped_pos = if self.duration_ms > 0 {
                    seek_pos.min(self.duration_ms)
                } else {
                    seek_pos
                };
                self.audio_controller
                    .seek(Duration::from_millis(clamped_pos));
                self.position_ms = clamped_pos;
                self.plugin_manager.on_seeked(self.position_ms);
                Task::none()
            }

            Message::SeekToMs(ms) => {
                let clamped = if self.duration_ms > 0 {
                    ms.min(self.duration_ms)
                } else {
                    ms
                };
                self.audio_controller.seek(Duration::from_millis(clamped));
                self.position_ms = clamped;
                self.player_bar
                    .update_position(self.position_ms, self.duration_ms);
                self.plugin_manager.on_seeked(self.position_ms);
                Task::none()
            }

            Message::VolumeChanged(vol) => {
                if vol > 0.01 {
                    self.pre_mute_volume = vol;
                }
                self.apply_volume(vol)
            }

            Message::ToggleMute => {
                let target = if self.volume > 0.01 {
                    self.pre_mute_volume = self.volume;
                    0.0
                } else if self.pre_mute_volume > 0.01 {
                    self.pre_mute_volume
                } else {
                    1.0
                };
                self.apply_volume(target)
            }

            Message::MediaControlAction(action) => match action {
                PlayerAction::Play => {
                    self.audio_controller.play();
                    Task::none()
                }
                PlayerAction::Pause => {
                    self.audio_controller.pause();
                    Task::none()
                }
                PlayerAction::TogglePlayPause => self.update(Message::TogglePlayPause),
                PlayerAction::Stop => self.update(Message::Stop),
                PlayerAction::Next => self.update(Message::NextTrack),
                PlayerAction::Previous => self.update(Message::PreviousTrack),
                PlayerAction::Seek(offset) => {
                    let current = Duration::from_millis(self.position_ms);
                    let target = current.saturating_add(offset);
                    let target_ms = target.as_millis().min(self.duration_ms as u128) as u64;
                    self.audio_controller.seek(Duration::from_millis(target_ms));
                    self.position_ms = target_ms;
                    self.plugin_manager.on_seeked(target_ms);
                    Task::none()
                }
                PlayerAction::SetPosition(pos) => {
                    let target_ms = pos.as_millis().min(self.duration_ms as u128) as u64;
                    self.audio_controller.seek(Duration::from_millis(target_ms));
                    self.position_ms = target_ms;
                    self.plugin_manager.on_seeked(target_ms);
                    Task::none()
                }
                PlayerAction::SetVolume(vol) => self.update(Message::VolumeChanged(vol)),
                PlayerAction::SetShuffle(shuffle) => {
                    if self.is_shuffle != shuffle {
                        self.update(Message::ToggleShuffle)
                    } else {
                        Task::none()
                    }
                }
                PlayerAction::SetRepeat(mode) => self.update(Message::SetRepeatMode(mode)),
                PlayerAction::OpenUri(uri) => {
                    if let Ok(url) = url::Url::parse(&uri) {
                        if url.scheme() == "file" {
                            if let Ok(path) = url.to_file_path() {
                                let path_str = path.to_string_lossy().to_string();
                                let db = self.db.clone();
                                return Task::perform(
                                    async move { db.get_track_by_path(&path_str).await.unwrap_or(None) },
                                    |res| Message::TrackResolvedForPlayback(Box::new(res)),
                                );
                            }
                        }
                    }
                    Task::none()
                }
                PlayerAction::Raise => {
                    info!("MPRIS Raise requested");
                    self.open_or_focus_window_task()
                }
                PlayerAction::Quit => {
                    info!("MPRIS Quit requested");
                    iced::exit()
                }
            },

            _ => unreachable!("routed in App::update"),
        }
    }

    pub fn get_next_track(&self) -> Option<&TrackRow> {
        if let Some(queue) = self.queues.active_queue() {
            if self.is_shuffle && !queue.is_empty() {
                let len = queue.len();
                if len <= 1 {
                    return queue.tracks.get(queue.current_index().unwrap_or(0));
                }
                let current = queue.current_index().unwrap_or(0);
                let mut pick = (self.position_ms as usize ^ 0x9e3779b9) % len;
                if pick == current {
                    pick = (pick + 1) % len;
                }
                return queue.tracks.get(pick);
            }
            if let Some(idx) = queue.next_index() {
                return queue.tracks.get(idx);
            }
            if self.repeat_mode == model::RepeatMode::All {
                return queue.tracks.first();
            }
            return None;
        }
        self.library
            .get_next_row(self.current_track_id, self.is_shuffle, self.position_ms)
    }

    pub(crate) fn sync_now_playing(&mut self) {
        self.now_playing.update_track(self.current_track.as_ref());
        let next = self.get_next_track().cloned();
        self.now_playing.update_next_track(next.as_ref());
        self.now_playing
            .update_tech_info(self.current_tech_info.as_ref());
        self.sync_status_strings();
    }

    pub(crate) fn sync_status_strings(&mut self) {
        if let Some(ref track) = self.current_track {
            self.status_now_playing_title =
                format!("Now playing: {} • {}", track.title, track.artist);
            self.status_now_playing_duration = track.formatted_duration();
        } else {
            self.status_now_playing_title.clear();
            self.status_now_playing_duration.clear();
        }
        if let Some(q) = self.queues.active_queue() {
            self.status_queue_name = format!("Queue: {}", q.name);
            self.status_queue_tracks = format!("{} tracks", q.len());
        } else {
            self.status_queue_name.clear();
            self.status_queue_tracks.clear();
        }
        self.status_playlist_count = format!("{} playlists", self.playlists.len());
        self.status_queue_count = format!("{} queues open", self.queues.summaries.len());
    }

    pub(super) fn load_now_playing_cover_task(
        &self,
        track_id: i64,
        path: PathBuf,
    ) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let cover = thumbnail::get_or_create(&path, thumbnail::Resolution::High);
                    let blurred = thumbnail::get_or_create(&path, thumbnail::Resolution::Low)
                        .as_deref()
                        .and_then(thumbnail::generate_blurred);
                    (cover, blurred)
                })
                .await
                .unwrap_or_default()
            },
            move |(cover, blurred)| Message::NowPlayingCoverLoaded {
                track_id,
                cover,
                blurred,
            },
        )
    }

    pub(super) fn load_track_assets_task(
        &self,
        track_id: i64,
        path: PathBuf,
        artist: String,
        title: String,
        duration_ms: u64,
    ) -> Task<Message> {
        let plugin_manager = self.plugin_manager.clone();
        let load_now_playing_cover = self.active_tab == ViewTab::Lyrics;

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let cover = thumbnail::get_or_create(&path, thumbnail::Resolution::Low);

                    let now_playing_cover = if load_now_playing_cover {
                        thumbnail::get_or_create(&path, thumbnail::Resolution::High)
                    } else {
                        None
                    };

                    let blurred = cover.as_deref().and_then(thumbnail::generate_blurred);

                    let tech_info = metadata::extract_technical_info(&path, duration_ms);

                    let lyrics = plugin_manager.fetch_lyrics(Some(&path), &artist, &title);

                    (cover, now_playing_cover, blurred, tech_info, lyrics)
                })
                .await
                .unwrap_or((None, None, None, None, None))
            },
            move |(cover, now_playing_cover, blurred, tech_info, lyrics)| {
                Message::PlaybackAssetsLoaded {
                    track_id,
                    cover,
                    now_playing_cover,
                    blurred,
                    tech_info,
                    lyrics,
                }
            },
        )
    }

    pub fn queue_next_gapless_track(&mut self) {
        if !self.gapless {
            self.pending_gapless_next = None;
            self.audio_controller.set_next_track(None);
            return;
        }

        if self.repeat_mode == model::RepeatMode::One {
            if let Some(track) = &self.current_track {
                let path = PathBuf::from(&track.file_path);
                self.audio_controller.set_next_track(Some(path));
                let track_row = TrackRow::from(track);
                self.pending_gapless_next = Some(track_row);
            } else {
                self.audio_controller.set_next_track(None);
                self.pending_gapless_next = None;
            }
            return;
        }

        if let Some(next_track) = self.get_next_track() {
            let path = PathBuf::from(&next_track.file_path);
            self.audio_controller.set_next_track(Some(path));
            self.pending_gapless_next = Some(next_track.clone());
        } else {
            self.audio_controller.set_next_track(None);
            self.pending_gapless_next = None;
        }
    }

    /// Updates the active volume across UI, audio engine, and plugins, then schedules a state save.
    fn apply_volume(&mut self, vol: f32) -> Task<Message> {
        let clamped = vol.clamp(0.0, 1.0);
        self.volume = clamped;
        self.player_bar.update_volume(clamped);
        self.audio_controller.set_volume(clamped);
        self.plugin_manager.on_volume_change(clamped);
        self.save_state_debounced()
    }
}
