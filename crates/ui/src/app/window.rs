use std::path::PathBuf;

use iced::widget::operation::scrollable::{scroll_to, AbsoluteOffset};
use iced::widget::operation::Animation;
use iced::Task;

use crate::message::Message;
use config::{PlaylistSection, SettingsTab, ViewTab};

use super::*;

impl App {
    pub(super) fn handle_window_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectTab(tab) => {
                self.active_tab = tab;
                if tab != ViewTab::Lyrics {
                    self.current_now_playing_cover = None;
                    self.current_blurred_cover = None;
                }
                if tab != ViewTab::Albums {
                    self.albums.clear_covers();
                }
                if tab != ViewTab::Playlists {
                    self.playlists.clear_covers();
                }
                let save_task = self.save_state_task();
                let scroll_task = self.restore_tab_scroll_task(tab);
                match tab {
                    ViewTab::Lyrics => {
                        let cover_task = if self.current_now_playing_cover.is_none()
                            || self.current_blurred_cover.is_none()
                        {
                            if let Some(track) = &self.current_track {
                                self.load_now_playing_cover_task(
                                    track.id,
                                    PathBuf::from(&track.file_path),
                                )
                            } else {
                                Task::none()
                            }
                        } else {
                            Task::none()
                        };
                        Task::batch([save_task, cover_task, scroll_task])
                    }
                    ViewTab::Albums => {
                        let load_task = if self.albums.is_empty() {
                            let db = self.db.clone();
                            Task::perform(
                                async move { db.get_albums().await.unwrap_or_default() },
                                Message::AlbumsLoaded,
                            )
                        } else {
                            self.albums.refresh_window();
                            self.albums.recompute_rendered_window(
                                &self.layout_manager.current_template.view_formats.albums,
                            );
                            self.load_visible_album_covers_task()
                        };
                        Task::batch([save_task, load_task, scroll_task])
                    }
                    ViewTab::Artists => {
                        let load_task = if self.artists.is_empty() {
                            let db = self.db.clone();
                            let load_task = Task::perform(
                                async move { db.get_artists().await.unwrap_or_default() },
                                Message::ArtistsLoaded,
                            );
                            Task::batch([save_task, load_task, scroll_task])
                        } else {
                            self.artists.refresh_window();
                            self.artists.recompute_rendered_window(
                                &self.layout_manager.current_template.view_formats.artists,
                            );
                            Task::batch([save_task, scroll_task])
                        };
                        load_task
                    }
                    ViewTab::Settings => {
                        let mut tasks = vec![save_task, self.load_storage_stats_task()];
                        if self.settings.active_tab == SettingsTab::Appearance {
                            if let Some(font_task) = self.ensure_fonts_task() {
                                tasks.push(font_task);
                            }
                        }
                        Task::batch(tasks)
                    }
                    ViewTab::Playlists => {
                        let db = self.db.clone();
                        let refresh = Task::perform(
                            async move { db.get_playlists().await.unwrap_or_default() },
                            Message::PlaylistsLoaded,
                        );
                        Task::batch([save_task, refresh, scroll_task])
                    }
                    _ => Task::batch([save_task, scroll_task]),
                }
            }

            Message::TabDragStarted(tab) => {
                self.dragging_tab = Some(tab);
                self.drag_order_changed = false;
                if self.active_tab != tab {
                    self.update(Message::SelectTab(tab))
                } else {
                    Task::none()
                }
            }

            Message::TabDraggedOver(target_tab) => {
                if let Some(dragged) = self.dragging_tab {
                    if dragged != target_tab {
                        if let (Some(drag_idx), Some(target_idx)) = (
                            self.tab_order.iter().position(|&t| t == dragged),
                            self.tab_order.iter().position(|&t| t == target_tab),
                        ) {
                            self.tab_order.remove(drag_idx);
                            self.tab_order.insert(target_idx, dragged);
                            self.drag_order_changed = true;
                        }
                    }
                }
                Task::none()
            }

            Message::TabDragReleased => {
                let queue_task = self.finish_queue_drag_task();
                let detail_task = self.finish_detail_drag_task();
                let had_drag = self.dragging_tab.is_some();
                let order_changed = self.drag_order_changed;
                self.dragging_tab = None;
                self.drag_order_changed = false;
                if had_drag && order_changed {
                    Task::batch([self.save_config_task(), queue_task, detail_task])
                } else {
                    Task::batch([queue_task, detail_task])
                }
            }

            Message::TabHovered(hovered) => {
                self.hovered_tab = hovered;
                Task::none()
            }

            Message::WindowResized(size) => {
                let width_diff = size.width - self.window_size.width;
                self.window_size = size;
                let save_task = self.save_state_debounced();
                let max_task = iced::window::oldest()
                    .and_then(iced::window::is_maximized)
                    .map(Message::WindowMaximizedChanged);
                let refresh_task = if width_diff.abs() > 0.5 {
                    match self.active_tab {
                        ViewTab::Albums => {
                            if self.albums.on_window_resized(width_diff) {
                                self.albums.recompute_rendered_window(
                                    &self.layout_manager.current_template.view_formats.albums,
                                );
                                self.load_visible_album_covers_task()
                            } else {
                                Task::none()
                            }
                        }
                        ViewTab::Artists => {
                            if self.artists.on_window_resized(width_diff) {
                                self.artists.recompute_rendered_window(
                                    &self.layout_manager.current_template.view_formats.artists,
                                );
                            }
                            Task::none()
                        }
                        _ => {
                            self.albums.on_window_resized(width_diff);
                            self.artists.on_window_resized(width_diff);
                            Task::none()
                        }
                    }
                } else {
                    Task::none()
                };
                Task::batch([save_task, max_task, refresh_task])
            }

            Message::TitlebarPressed => {
                let now = std::time::Instant::now();
                let is_double_click = self
                    .last_titlebar_press
                    .is_some_and(|prev| now.duration_since(prev).as_millis() < 350);
                if is_double_click {
                    self.last_titlebar_press = None;
                    iced::window::oldest().and_then(|id| {
                        Task::batch([
                            iced::window::toggle_maximize(id),
                            iced::window::is_maximized(id).map(Message::WindowMaximizedChanged),
                        ])
                    })
                } else {
                    self.last_titlebar_press = Some(now);
                    iced::window::oldest().and_then(iced::window::drag)
                }
            }

            Message::WindowDragResize(direction) => {
                if self.is_maximized {
                    Task::none()
                } else {
                    iced::window::oldest()
                        .and_then(move |id| iced::window::drag_resize(id, direction))
                }
            }

            Message::WindowMinimize => {
                iced::window::oldest().and_then(|id| iced::window::minimize(id, true))
            }

            Message::WindowToggleMaximize => iced::window::oldest().and_then(|id| {
                Task::batch([
                    iced::window::toggle_maximize(id),
                    iced::window::is_maximized(id).map(Message::WindowMaximizedChanged),
                ])
            }),

            Message::WindowMaximizedChanged(maximized) => {
                self.is_maximized = maximized;
                Task::none()
            }

            Message::WindowClose => {
                if self.minimize_to_tray {
                    if self.tray.is_none() {
                        self.tray = crate::tray::TrayController::new();
                    }
                    #[cfg(target_os = "windows")]
                    if let Some(ref plugin) = self.windows_media_plugin {
                        plugin.detach_window();
                    }
                    let id = self.main_window_id.take();
                    let save_state = self.save_state_task();
                    if let Some(id) = id {
                        Task::batch([save_state, iced::window::close(id)])
                    } else {
                        save_state
                    }
                } else {
                    iced::exit()
                }
            }

            Message::WindowOpened(id) => {
                self.main_window_id = Some(id);
                let mut tasks = vec![crate::window_decorations::sync_titlebar_theme(
                    Some(id),
                    &self.theme_manager.current_config,
                )];
                if self.is_maximized {
                    tasks.push(iced::window::maximize(id, true));
                } else {
                    tasks.push(iced::window::is_maximized(id).map(Message::WindowMaximizedChanged));
                }

                #[cfg(target_os = "windows")]
                if let Some(ref plugin) = self.windows_media_plugin {
                    use iced::window::raw_window_handle::RawWindowHandle;
                    let plugin = plugin.clone();
                    tasks.push(
                        iced::window::run(id, move |window| {
                            if let Ok(handle) = window.window_handle() {
                                if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                                    let hwnd = win32.hwnd.get();
                                    plugin.attach_window(hwnd);
                                }
                            }
                        })
                        .discard(),
                    );
                }

                Task::batch(tasks)
            }

            Message::WindowClosed(id) => {
                if self.main_window_id == Some(id) {
                    self.main_window_id = None;
                }
                if !self.minimize_to_tray {
                    return iced::exit();
                }
                Task::none()
            }

            Message::WindowShowSystemMenu => {
                iced::window::oldest().and_then(iced::window::show_system_menu)
            }

            Message::ToggleCustomTitlebar(enabled) => {
                if self.custom_titlebar == enabled {
                    return Task::none();
                }
                self.custom_titlebar = enabled;
                let theme_cfg = self.theme_manager.current_config.clone();
                Task::batch([
                    self.save_config_task(),
                    iced::window::oldest().and_then(iced::window::toggle_decorations),
                    crate::window_decorations::sync_titlebar_theme(None, &theme_cfg),
                ])
            }

            Message::ToggleMinimizeToTray(enabled) => {
                if self.minimize_to_tray == enabled {
                    return Task::none();
                }
                self.minimize_to_tray = enabled;
                if enabled {
                    if self.tray.is_none() {
                        self.tray = crate::tray::TrayController::new();
                    }
                } else {
                    self.tray = None;
                }
                self.save_config_task()
            }

            Message::TrayAction(action) => match action {
                crate::tray::TrayAction::ShowWindow => self.open_or_focus_window_task(),
                crate::tray::TrayAction::PlayPause => self.update(Message::TogglePlayPause),
                crate::tray::TrayAction::Next => self.update(Message::NextTrack),
                crate::tray::TrayAction::Previous => self.update(Message::PreviousTrack),
                crate::tray::TrayAction::Quit => iced::exit(),
            },

            _ => unreachable!("routed in App::update"),
        }
    }

    pub(super) fn restore_tab_scroll_task(&self, tab: ViewTab) -> Task<Message> {
        match tab {
            ViewTab::Tracks => {
                let y = self.library.scroll_offset.get();
                if y > 0.0 {
                    scroll_to(
                        iced::widget::Id::new("library_tracks"),
                        AbsoluteOffset { x: 0.0, y },
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                }
            }
            ViewTab::Albums => {
                let y = self.albums.scroll_offset.get();
                if y > 0.0 {
                    scroll_to(
                        iced::widget::Id::new("albums_scroll"),
                        AbsoluteOffset { x: 0.0, y },
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                }
            }
            ViewTab::Artists => {
                let y = self.artists.scroll_offset.get();
                if y > 0.0 {
                    scroll_to(
                        iced::widget::Id::new("artists_scroll"),
                        AbsoluteOffset { x: 0.0, y },
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                }
            }
            ViewTab::Playlists => match self.playlists.section {
                PlaylistSection::Playlists => {
                    if self.playlists.selected_id.is_some() {
                        let y = self.playlists.last_detail_scroll.get();
                        if y > 0.0 {
                            scroll_to(
                                iced::widget::Id::new("playlist_detail_scroll"),
                                AbsoluteOffset { x: 0.0, y },
                                Animation::Instant,
                            )
                        } else {
                            Task::none()
                        }
                    } else {
                        let y = self.playlists.last_playlists_scroll.get();
                        if y > 0.0 {
                            scroll_to(
                                iced::widget::Id::new("playlists_scroll"),
                                AbsoluteOffset { x: 0.0, y },
                                Animation::Instant,
                            )
                        } else {
                            Task::none()
                        }
                    }
                }
                PlaylistSection::Queues => {
                    let y = self.playlists.last_queue_scroll.get();
                    if y > 0.0 {
                        scroll_to(
                            iced::widget::Id::new("queue_scroll"),
                            AbsoluteOffset { x: 0.0, y },
                            Animation::Instant,
                        )
                    } else {
                        Task::none()
                    }
                }
            },
            ViewTab::Lyrics => {
                let y = self.now_playing.scroll_offset.get();
                if y > 0.0 {
                    scroll_to(
                        iced::widget::Id::new("lyrics_scroll"),
                        AbsoluteOffset { x: 0.0, y },
                        Animation::Instant,
                    )
                } else {
                    Task::none()
                }
            }
            ViewTab::Settings => Task::none(),
        }
    }

    pub(crate) fn use_custom_titlebar(&self) -> bool {
        self.custom_titlebar
    }

    pub(crate) fn open_or_focus_window_task(&mut self) -> Task<Message> {
        if let Some(id) = self.main_window_id {
            Task::batch([
                iced::window::minimize(id, false),
                iced::window::gain_focus(id),
            ])
        } else {
            let cfg = self.to_config();
            let (id, task) = iced::window::open(crate::build_window_settings(
                &cfg,
                self.window_size.width,
                self.window_size.height,
            ));
            self.main_window_id = Some(id);
            task.map(Message::WindowOpened)
        }
    }
}
