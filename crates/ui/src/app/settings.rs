use iced::widget::operation::scrollable::{snap_to, RelativeOffset};
use iced::widget::operation::Animation;
use iced::Task;
use plugin::scrobbler::ScrobbleService;
use tracing::info;

use crate::message::Message;
use config::SettingsTab;

use super::*;

impl App {
    pub(super) fn handle_settings_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectLyricsFont(font_name) => {
                let font_opt = if font_name == "Default" || font_name.trim().is_empty() {
                    None
                } else {
                    Some(font_name.trim().to_string())
                };
                self.fonts.set_lyrics(font_opt);
                self.save_config_task()
            }

            Message::SetLyricsFontSize(size) => {
                self.fonts.set_lyrics_size(size);
                self.save_config_debounced()
            }

            Message::SelectSettingsTab(tab) => {
                self.settings.active_tab = tab;
                self.settings.show_debug_subpage = false;
                let mut tasks = vec![self.save_state_task(), self.load_storage_stats_task()];
                if tab == SettingsTab::Appearance {
                    if let Some(font_task) = self.ensure_fonts_task() {
                        tasks.push(font_task);
                    }
                } else if tab == SettingsTab::Playback {
                    self.available_output_devices = audio::list_output_devices();
                }
                Task::batch(tasks)
            }

            Message::SetDebugSubpage(open) => {
                self.settings.show_debug_subpage = open;
                if open {
                    snap_to(
                        iced::widget::Id::new("settings_scroll"),
                        RelativeOffset::START,
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                }
            }

            Message::DebugNoop => Task::none(),
            Message::DebugToggle(v) => {
                self.settings.debug_state.toggler = v;
                Task::none()
            }
            Message::DebugSliderChanged(v) => {
                self.settings.debug_state.slider_val = v;
                Task::none()
            }
            Message::DebugInputChanged(s) => {
                self.settings.debug_state.input_val = s;
                Task::none()
            }
            Message::DebugPickSelected(s) => {
                self.settings.debug_state.selected_option = s;
                Task::none()
            }
            Message::DebugChipSelected(i) => {
                self.settings.debug_state.active_chip = i;
                Task::none()
            }
            Message::DebugViewModeChanged(mode) => {
                self.settings.debug_state.view_mode = mode;
                Task::none()
            }
            Message::DebugSortChanged(field) => {
                let st = &mut self.settings.debug_state;
                if st.sort_field == field {
                    st.sort_dir = match st.sort_dir {
                        model::SortDirection::Ascending => model::SortDirection::Descending,
                        model::SortDirection::Descending => model::SortDirection::Ascending,
                    };
                } else {
                    st.sort_field = field;
                    st.sort_dir = model::SortDirection::Ascending;
                }
                Task::none()
            }

            Message::SelectTheme(name) => {
                self.theme_manager.set_theme(&name);
                Task::batch([
                    self.save_config_task(),
                    crate::window_decorations::sync_titlebar_theme(
                        None,
                        &self.theme_manager.current_config,
                    ),
                ])
            }

            Message::SelectLayout(name) => {
                if self.layout_manager.select_layout(&name) {
                    let formats = &self.layout_manager.current_template.view_formats;
                    self.library.recompute_rendered_rows(&formats.library);
                    self.albums.recompute_rendered_window(&formats.albums);
                    self.artists.recompute_rendered_window(&formats.artists);
                    self.playlists
                        .recompute_rendered_playlists(&formats.playlists);
                    self.playlists
                        .recompute_rendered_details(&formats.playlists);
                    self.playlists
                        .recompute_rendered_queue(&formats.queue, &self.queues);
                    #[cfg(target_os = "windows")]
                    crate::window_decorations::snap_layouts::sync_state(
                        self.use_custom_titlebar(),
                        self.current_titlebar_height(),
                    );
                    self.save_config_task()
                } else {
                    Task::none()
                }
            }

            Message::ReloadThemesAndLayouts => {
                self.theme_manager.load_all_themes();
                self.layout_manager.load_all_layouts();
                let formats = &self.layout_manager.current_template.view_formats;
                self.library.recompute_rendered_rows(&formats.library);
                self.albums.recompute_rendered_window(&formats.albums);
                self.artists.recompute_rendered_window(&formats.artists);
                self.playlists
                    .recompute_rendered_playlists(&formats.playlists);
                self.playlists
                    .recompute_rendered_details(&formats.playlists);
                self.playlists
                    .recompute_rendered_queue(&formats.queue, &self.queues);
                #[cfg(target_os = "windows")]
                crate::window_decorations::snap_layouts::sync_state(
                    self.use_custom_titlebar(),
                    self.current_titlebar_height(),
                );
                crate::window_decorations::sync_titlebar_theme(
                    None,
                    &self.theme_manager.current_config,
                )
            }

            Message::SelectFont(font_name) => {
                info!("Selected font: {}", font_name);
                let font_opt = if font_name == "Default" || font_name.trim().is_empty() {
                    None
                } else {
                    Some(font_name.trim().to_string())
                };
                self.fonts.set_primary(font_opt);
                self.save_config_task()
            }

            Message::FontsLoaded(fonts) => {
                self.settings.set_available_fonts(fonts);
                Task::none()
            }

            Message::ToggleListenBrainz(enabled) => {
                self.scrobble_config.listenbrainz_enabled = enabled;
                self.listenbrainz_plugin.set_enabled(enabled);
                self.save_config_task()
            }

            Message::ListenBrainzTokenChanged(token) => {
                self.scrobble_config.listenbrainz_token = token.clone();
                self.listenbrainz_plugin.set_user_token(token);
                self.save_config_task()
            }

            Message::ListenBrainzUrlChanged(url) => {
                self.scrobble_config.listenbrainz_api_url = url.clone();
                self.listenbrainz_plugin.set_api_url(url);
                self.save_config_task()
            }

            Message::ToggleLastFm(enabled) => {
                self.scrobble_config.lastfm_enabled = enabled;
                self.lastfm_plugin.set_enabled(enabled);
                self.save_config_task()
            }

            Message::LastFmApiKeyChanged(key) => {
                self.scrobble_config.lastfm_api_key = key.clone();
                self.lastfm_plugin.set_api_key(key);
                self.save_config_task()
            }

            Message::LastFmApiSecretChanged(secret) => {
                self.scrobble_config.lastfm_api_secret = secret.clone();
                self.lastfm_plugin.set_api_secret(secret);
                self.save_config_task()
            }

            Message::LastFmSessionKeyChanged(sk) => {
                self.scrobble_config.lastfm_session_key = sk.clone();
                self.lastfm_plugin.set_session_key(sk);
                self.save_config_task()
            }

            Message::LastFmUrlChanged(url) => {
                self.scrobble_config.lastfm_api_url = url.clone();
                self.lastfm_plugin.set_api_url(url);
                self.save_config_task()
            }

            Message::ToggleSubmitNowPlaying(submit) => {
                self.scrobble_config.submit_now_playing = submit;
                self.listenbrainz_plugin.set_submit_now_playing(submit);
                self.lastfm_plugin.set_submit_now_playing(submit);
                self.save_config_task()
            }

            Message::SetScrobblePercent(pct) => {
                self.scrobble_config.scrobble_percent = pct;
                self.listenbrainz_plugin.set_scrobble_percent(pct);
                self.lastfm_plugin.set_scrobble_percent(pct);
                Task::none()
            }

            Message::TestScrobble(service) => {
                let plugin_lb = self.listenbrainz_plugin.clone();
                let plugin_lfm = self.lastfm_plugin.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || match service {
                            ScrobbleService::ListenBrainz => (service, plugin_lb.test_connection()),
                            ScrobbleService::LastFm => (service, plugin_lfm.test_connection()),
                        })
                        .await
                        .unwrap_or_else(|e| (service, Err(format!("Task execution error: {}", e))))
                    },
                    |(service, res)| Message::ScrobbleTestFinished(service, res),
                )
            }

            Message::ScrobbleTestFinished(service, res) => {
                self.settings.scrobble_test_status = Some((service, res));
                Task::none()
            }

            Message::CheckForUpdates => {
                self.settings.update_status = updater::UpdateStatus::Checking;
                Task::perform(updater::check_for_updates(), Message::UpdateCheckFinished)
            }

            Message::UpdateCheckFinished(result) => {
                match result {
                    Ok(Some(info)) => {
                        self.settings.update_status = updater::UpdateStatus::Available(info);
                    }
                    Ok(None) => {
                        self.settings.update_status = updater::UpdateStatus::UpToDate;
                    }
                    Err(err) => {
                        self.settings.update_status = updater::UpdateStatus::Failed(err);
                    }
                }
                Task::none()
            }

            Message::StartUpdateDownload(info) => {
                self.settings.update_status = updater::UpdateStatus::Downloading {
                    info: info.clone(),
                    percent: 0,
                };
                Task::stream(iced::stream::channel(100, async move |mut sender| {
                    let mut cb_sender = sender.clone();
                    let info_clone = info.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        updater::download_update(move |pct| {
                            let _ = cb_sender.try_send(Message::UpdateDownloadProgress(pct));
                        })
                    })
                    .await;

                    match res {
                        Ok(Ok(())) => {
                            let _ = sender
                                .send(Message::UpdateDownloadFinished(Ok(info_clone)))
                                .await;
                        }
                        Ok(Err(err)) => {
                            let _ = sender.send(Message::UpdateDownloadFinished(Err(err))).await;
                        }
                        Err(join_err) => {
                            let _ = sender
                                .send(Message::UpdateDownloadFinished(Err(join_err.to_string())))
                                .await;
                        }
                    }
                }))
            }

            Message::UpdateDownloadProgress(percent) => {
                if let updater::UpdateStatus::Downloading { ref info, .. } =
                    self.settings.update_status
                {
                    self.settings.update_status = updater::UpdateStatus::Downloading {
                        info: info.clone(),
                        percent,
                    };
                }
                Task::none()
            }

            Message::UpdateDownloadFinished(result) => {
                match result {
                    Ok(info) => {
                        self.settings.update_status =
                            updater::UpdateStatus::ReadyToRestart { info };
                    }
                    Err(err) => {
                        self.settings.update_status = updater::UpdateStatus::Failed(err);
                    }
                }
                Task::none()
            }

            Message::ApplyUpdateAndRestart => {
                let state = self.to_state();
                let state_path = self.state_path;
                let config = self.to_config();
                let config_path = self.config_path;

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            state.save_to_path(state_path);
                            config.save_to_path(config_path);
                            updater::apply_update_and_restart()
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |res| match res {
                        Ok(()) => Message::StateSaved,
                        Err(err) => Message::UpdateDownloadFinished(Err(err)),
                    },
                )
            }

            Message::OpenUrl(url) => {
                updater::open_in_browser(&url);
                Task::none()
            }
            _ => unreachable!("routed in App::update"),
        }
    }
}
