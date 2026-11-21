use iced::widget::operation::scrollable::{snap_to, RelativeOffset};
use iced::widget::operation::Animation;
use iced::Task;
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
                    #[cfg(target_os = "windows")]
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
                {
                    crate::window_decorations::snap_layouts::sync_state(
                        self.use_custom_titlebar(),
                        self.current_titlebar_height(),
                    );
                    crate::window_decorations::sync_titlebar_theme(
                        None,
                        &self.theme_manager.current_config,
                    )
                }
                #[cfg(not(target_os = "windows"))]
                Task::none()
            }

            Message::OpenConfigFolder => {
                if let Err(err) = path::open_directory(path::config_dir()) {
                    tracing::warn!("Failed to open config folder: {err}");
                }
                Task::none()
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

            Message::ToggleExtension(id, enabled) => {
                let saved = {
                    let entry = self.extensions_config.entry(id.clone()).or_default();
                    entry.enabled = enabled;
                    entry.settings.clone()
                };
                self.settings.extension_action_status.remove(&id);
                let pm = self.plugin_manager.clone();
                let sync_task = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            pm.set_extension_enabled(&id, enabled, &saved)
                                .unwrap_or_else(|_| pm.snapshots())
                        })
                        .await
                        .unwrap_or_default()
                    },
                    Message::ExtensionsSynced,
                );
                Task::batch([sync_task, self.save_config_task()])
            }

            Message::ExtensionSettingChanged {
                extension_id,
                key,
                value,
                save_immediately,
            } => {
                let entry = self
                    .extensions_config
                    .entry(extension_id.clone())
                    .or_default();
                entry.settings.insert(key, value);
                self.plugin_manager
                    .update_extension_settings(&extension_id, &entry.settings);
                if save_immediately {
                    self.save_config_task()
                } else {
                    self.save_config_debounced()
                }
            }

            Message::TriggerExtensionAction {
                extension_id,
                action_id,
            } => {
                self.settings
                    .extension_action_busy
                    .insert((extension_id.clone(), action_id.clone()));
                let saved = self
                    .extensions_config
                    .get(&extension_id)
                    .map(|e| e.settings.clone())
                    .unwrap_or_default();
                let pm = self.plugin_manager.clone();
                Task::perform(
                    async move {
                        let ext_id = extension_id.clone();
                        let act_id = action_id.clone();
                        let result = tokio::task::spawn_blocking(move || {
                            pm.trigger_extension_action(&ext_id, &act_id, &saved)
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("Task execution error: {e}")));
                        (extension_id, action_id, result)
                    },
                    |(extension_id, action_id, result)| Message::ExtensionActionFinished {
                        extension_id,
                        action_id,
                        result,
                    },
                )
            }

            Message::ExtensionActionFinished {
                extension_id,
                action_id,
                result,
            } => {
                self.settings
                    .extension_action_busy
                    .remove(&(extension_id.clone(), action_id));
                match result {
                    Ok(outcome) => {
                        let has_updates = !outcome.updated_settings.is_empty();
                        if has_updates {
                            let entry = self
                                .extensions_config
                                .entry(extension_id.clone())
                                .or_default();
                            for (key, value) in outcome.updated_settings {
                                entry.settings.insert(key, value);
                            }
                            self.plugin_manager
                                .update_extension_settings(&extension_id, &entry.settings);
                        }
                        self.settings
                            .extension_action_status
                            .insert(extension_id, Ok(outcome.message));
                        if has_updates {
                            return self.save_config_task();
                        }
                        Task::none()
                    }
                    Err(err) => {
                        self.settings
                            .extension_action_status
                            .insert(extension_id, Err(err));
                        Task::none()
                    }
                }
            }

            Message::InstallExtensionArchiveClicked => Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Install Melody extension archive")
                        .add_filter("Melody extension", &["zip", "mext"])
                        .pick_file()
                        .await;
                    file.map(|f| f.path().to_path_buf())
                },
                Message::ExtensionInstallPathSelected,
            ),

            Message::InstallExtensionFolderClicked => Task::perform(
                async {
                    let folder = rfd::AsyncFileDialog::new()
                        .set_title("Install Melody extension folder")
                        .pick_folder()
                        .await;
                    folder.map(|f| f.path().to_path_buf())
                },
                Message::ExtensionInstallPathSelected,
            ),

            Message::ExtensionInstallPathSelected(Some(source)) => {
                let pm = self.plugin_manager.clone();
                let ext_cfg = self.extensions_config.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            pm.install_from_path(&source, &ext_cfg)
                                .map(|(manifest, snaps)| (manifest.name, snaps))
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("Install task failed: {e}")))
                    },
                    Message::ExtensionInstalled,
                )
            }

            Message::ExtensionInstallPathSelected(None) => Task::none(),

            Message::ExtensionInstalled(Ok((_name, snapshots))) => {
                self.extensions_snapshots = snapshots;
                self.settings.extension_banner = None;
                Task::none()
            }

            Message::ExtensionInstalled(Err(err)) => {
                self.settings.extension_banner =
                    Some((false, format!("Extension installation failed: {err}")));
                Task::none()
            }

            Message::UninstallExtension(id) => {
                let pm = self.plugin_manager.clone();
                Task::perform(
                    async move {
                        let id_clone = id.clone();
                        tokio::task::spawn_blocking(move || {
                            pm.uninstall_extension(&id_clone)
                                .map(|snaps| (id_clone, snaps))
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("Uninstall task failed: {e}")))
                    },
                    Message::ExtensionUninstalled,
                )
            }

            Message::ExtensionUninstalled(Ok((id, snapshots))) => {
                self.extensions_snapshots = snapshots;
                self.extensions_config.remove(&id);
                self.settings.extension_action_status.remove(&id);
                self.settings.extension_banner = None;
                self.save_config_task()
            }

            Message::ExtensionUninstalled(Err(err)) => {
                self.settings.extension_banner =
                    Some((false, format!("Failed to uninstall extension: {err}")));
                Task::none()
            }

            Message::ReloadExtensions => {
                let pm = self.plugin_manager.clone();
                let ext_cfg = self.extensions_config.clone();
                self.settings.extension_banner = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || pm.discover_and_sync(&ext_cfg))
                            .await
                            .unwrap_or_default()
                    },
                    Message::ExtensionsSynced,
                )
            }

            Message::ExtensionsSynced(snapshots) => {
                self.extensions_snapshots = snapshots;
                Task::none()
            }

            Message::OpenExtensionsFolder => {
                if let Err(err) = path::open_directory(path::user_extensions_dir()) {
                    self.settings.extension_banner =
                        Some((false, format!("Failed to open extensions folder: {err}")));
                }
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
