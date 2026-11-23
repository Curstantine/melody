use std::path::PathBuf;

use iced::widget::{column, container, mouse_area, row, Space, Widget};
use iced::{Alignment, Color, Element, Length, Task};
use model::{SortDirection, SortField, TrackFilter, TrackRow};

use crate::message::{Message, PlaylistOpenIntent};
use ::queue::DEFAULT_QUEUE_NAME;
use component::{
    button, icons, styles, ContextMenuAction, ContextMenuState, ContextMenuTarget, TrackMenuContext,
};
use config::{PlaylistSection, ViewTab};
use theme::ThemeConfig;

use super::*;

impl App {
    pub(super) fn handle_context_menu_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenTrackContextMenu { track_id, context } => {
                let position = self.cursor_position();
                if let Some(track) = self.find_track_row(track_id) {
                    self.context_menu = Some(ContextMenuState {
                        position,
                        target: ContextMenuTarget::Track { track, context },
                        submenu: None,
                    });
                    Task::none()
                } else {
                    let db = self.db.clone();
                    Task::perform(
                        async move {
                            db.get_track_rows_by_ids(&[track_id])
                                .await
                                .ok()
                                .and_then(|mut r| r.remove(&track_id))
                        },
                        move |maybe_track| {
                            if let Some(track) = maybe_track {
                                Message::TrackContextMenuReady {
                                    track,
                                    context,
                                    position,
                                }
                            } else {
                                Message::ContextMenu(ContextMenuAction::Close)
                            }
                        },
                    )
                }
            }

            Message::TrackContextMenuReady {
                track,
                context,
                position,
            } => {
                self.context_menu = Some(ContextMenuState {
                    position,
                    target: ContextMenuTarget::Track { track, context },
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenAlbumContextMenu(idx) => {
                if let Some(album) = self.albums.albums.get(idx).cloned() {
                    let position = self.cursor_position();
                    self.context_menu = Some(ContextMenuState {
                        position,
                        target: ContextMenuTarget::Album(album),
                        submenu: None,
                    });
                }
                Task::none()
            }

            Message::OpenArtistContextMenu(idx) => {
                if let Some(artist) = self.artists.artists.get(idx).cloned() {
                    let position = self.cursor_position();
                    self.context_menu = Some(ContextMenuState {
                        position,
                        target: ContextMenuTarget::Artist(artist),
                        submenu: None,
                    });
                }
                Task::none()
            }

            Message::OpenPlaylistContextMenu(playlist) => {
                let position = self.cursor_position();
                self.context_menu = Some(ContextMenuState {
                    position,
                    target: ContextMenuTarget::Playlist(playlist),
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenQueueContextMenu { id, name } => {
                let position = self.cursor_position();
                self.context_menu = Some(ContextMenuState {
                    position,
                    target: ContextMenuTarget::Queue { id, name },
                    submenu: None,
                });
                Task::none()
            }

            Message::OpenPlayerBarContextMenu => {
                if let Some(ref track) = self.current_track {
                    let track_row = TrackRow::from(track);
                    let position = self.cursor_position();
                    self.context_menu = Some(ContextMenuState {
                        position,
                        target: ContextMenuTarget::Track {
                            track: track_row,
                            context: TrackMenuContext::PlayerBar,
                        },
                        submenu: None,
                    });
                }
                Task::none()
            }

            Message::ContextMenu(action) => self.handle_context_menu(action),

            Message::QueuePlayTracksNext(tracks) => {
                if !tracks.is_empty() {
                    if self.queues.active.is_none() {
                        self.queues.create(DEFAULT_QUEUE_NAME);
                    }
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.insert_many_next(tracks);
                    }
                    self.queues.touch_active_summary();
                    return self.persist_active_queue_task();
                }
                Task::none()
            }

            Message::QueueAddTracks(tracks) => {
                if !tracks.is_empty() {
                    if self.queues.active.is_none() {
                        self.queues.create(DEFAULT_QUEUE_NAME);
                    }
                    if let Some(queue) = self.queues.active_queue_mut() {
                        queue.append_many(tracks);
                    }
                    self.queues.touch_active_summary();
                    return self.persist_active_queue_task();
                }
                Task::none()
            }

            Message::NewPlaylistWithTracks(tracks) => {
                self.active_tab = ViewTab::Playlists;
                self.playlists.section = PlaylistSection::Playlists;
                self.playlists.creating = true;
                self.playlists.new_name_input.clear();
                self.pending_playlist_tracks = tracks;
                Task::none()
            }

            Message::ConfirmDuplicatePrompt => {
                if let Some(prompt) = self.duplicate_prompt.take() {
                    match prompt.target {
                        DuplicateTarget::Playlist {
                            track, playlist_id, ..
                        } => self.execute_add_track_to_playlist(track, playlist_id),
                        DuplicateTarget::Queue { track } => self.execute_add_track_to_queue(track),
                    }
                } else {
                    Task::none()
                }
            }

            Message::CancelDuplicatePrompt => {
                self.duplicate_prompt = None;
                Task::none()
            }

            Message::EscapePressed => {
                if self.duplicate_prompt.is_some() {
                    self.duplicate_prompt = None;
                } else if self.context_menu.is_some() {
                    self.context_menu = None;
                }
                Task::none()
            }

            _ => unreachable!("routed in App::update"),
        }
    }

    pub(super) fn handle_context_menu(&mut self, action: ContextMenuAction) -> Task<Message> {
        if let ContextMenuAction::ToggleSubmenu(sub) = action {
            if let Some(ref mut menu) = self.context_menu {
                menu.submenu = sub;
            }
            return Task::none();
        }

        self.context_menu = None;

        match action {
            ContextMenuAction::Close => Task::none(),
            ContextMenuAction::ToggleSubmenu(_) => unreachable!(),

            // Track actions
            ContextMenuAction::PlayTrack(id) => self.start_library_playback(id),
            ContextMenuAction::PlayPlaylistItem { playlist_id, index } => {
                self.update(Message::PlayPlaylistItem(playlist_id, index))
            }
            ContextMenuAction::PlayNextTrack(track) => {
                if self.queues.active.is_none() {
                    self.queues.create(DEFAULT_QUEUE_NAME);
                }
                if let Some(queue) = self.queues.active_queue_mut() {
                    queue.insert_next(track);
                }
                self.queues.touch_active_summary();
                self.persist_active_queue_task()
            }
            ContextMenuAction::AddTrackToQueue(track) => {
                if self.queue_contains_track(&track) {
                    self.duplicate_prompt = Some(DuplicatePromptState {
                        target: DuplicateTarget::Queue { track },
                    });
                    Task::none()
                } else {
                    self.execute_add_track_to_queue(track)
                }
            }
            ContextMenuAction::GoToAlbum(album) => self.update(Message::SelectAlbum(album)),
            ContextMenuAction::GoToArtist(artist) => self.update(Message::SelectArtist(artist)),
            ContextMenuAction::RemoveFromPlaylist { playlist_id, index } => {
                self.update(Message::PlaylistRemoveTrack { playlist_id, index })
            }
            ContextMenuAction::RemoveFromQueue { queue_id, index } => {
                self.update(Message::QueueRemoveTrack { queue_id, index })
            }
            ContextMenuAction::RevealInFileManager(path) => {
                if let Err(err) = path::reveal_in_file_manager(std::path::Path::new(&path)) {
                    tracing::warn!(%path, %err, "Failed to reveal file in file manager");
                }
                Task::none()
            }

            // Album actions
            ContextMenuAction::PlayAlbum { album, shuffle } => self.update(Message::PlayScope {
                filter: TrackFilter::album(album.as_ref()),
                name: album,
                shuffle,
            }),
            ContextMenuAction::PlayNextAlbum(album) => {
                let db = self.db.clone();
                let filter = TrackFilter::album(album.album.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::QueuePlayTracksNext,
                )
            }
            ContextMenuAction::AddAlbumToQueue(album) => {
                let db = self.db.clone();
                let filter = TrackFilter::album(album.album.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::QueueAddTracks,
                )
            }

            // Artist actions
            ContextMenuAction::PlayArtist { artist, shuffle } => self.update(Message::PlayScope {
                filter: TrackFilter::artist(artist.as_ref()),
                name: artist,
                shuffle,
            }),
            ContextMenuAction::PlayNextArtist(artist) => {
                let db = self.db.clone();
                let filter = TrackFilter::artist(artist.artist.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::QueuePlayTracksNext,
                )
            }
            ContextMenuAction::AddArtistToQueue(artist) => {
                let db = self.db.clone();
                let filter = TrackFilter::artist(artist.artist.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::QueueAddTracks,
                )
            }

            // Playlist actions
            ContextMenuAction::PlayPlaylist(id) => self.update(Message::PlayPlaylist(id)),
            ContextMenuAction::PlayNextPlaylist(id) => self.update(Message::PlaylistPlayNext(id)),
            ContextMenuAction::AddPlaylistToQueue(id) => self.update(Message::PlaylistToQueue(id)),
            ContextMenuAction::RenamePlaylist(id) => self.update(Message::RenamePlaylist(id)),
            ContextMenuAction::DeletePlaylist(id) => self.update(Message::DeletePlaylist(id)),

            // Queue actions
            ContextMenuAction::PlayQueue(id) => self.update(Message::QueueSelect(id)),
            ContextMenuAction::DeleteQueue(id) => self.update(Message::QueueDelete(id)),

            // Submenu actions
            ContextMenuAction::NewPlaylistWithTrack(track) => {
                self.update(Message::NewPlaylistWithTracks(vec![track]))
            }
            ContextMenuAction::NewPlaylistWithAlbum(album) => {
                let db = self.db.clone();
                let filter = TrackFilter::album(album.album.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::NewPlaylistWithTracks,
                )
            }
            ContextMenuAction::NewPlaylistWithArtist(artist) => {
                let db = self.db.clone();
                let filter = TrackFilter::artist(artist.artist.as_ref());
                Task::perform(
                    async move {
                        db.get_all_track_rows_filtered(
                            Some(&filter),
                            SortField::TrackNum,
                            SortDirection::Ascending,
                        )
                        .await
                        .unwrap_or_default()
                    },
                    Message::NewPlaylistWithTracks,
                )
            }
            ContextMenuAction::AddTrackToPlaylist { track, playlist_id } => {
                if self.playlist_contains_track(playlist_id, &track) {
                    let playlist_name = self
                        .playlist_by_id(playlist_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Playlist".to_string());
                    self.duplicate_prompt = Some(DuplicatePromptState {
                        target: DuplicateTarget::Playlist {
                            track,
                            playlist_id,
                            playlist_name,
                        },
                    });
                    Task::none()
                } else {
                    self.execute_add_track_to_playlist(track, playlist_id)
                }
            }
            ContextMenuAction::AddAlbumToPlaylist { album, playlist_id } => {
                let playlist = self.playlist_by_id(playlist_id);
                if let Some(p) = playlist {
                    let path = PathBuf::from(&p.file_path);
                    let file_path = p.file_path.clone();
                    let name = p.name.clone();
                    let db = self.db.clone();
                    let filter = TrackFilter::album(album.album.as_ref());
                    let is_open = self.playlists.selected_id == Some(playlist_id);
                    let reload_detail = if is_open {
                        self.resolve_playlist_intent(playlist_id, PlaylistOpenIntent::View)
                    } else {
                        Task::none()
                    };

                    let save_task = Task::perform(
                        async move {
                            let tracks = db
                                .get_all_track_rows_filtered(
                                    Some(&filter),
                                    SortField::TrackNum,
                                    SortDirection::Ascending,
                                )
                                .await
                                .unwrap_or_default();

                            let wrote = tokio::task::spawn_blocking(move || {
                                playlist::append_tracks_to_m3u(&path, &tracks)
                            })
                            .await
                            .ok()
                            .and_then(|r| r.ok());

                            if let Some(count) = wrote {
                                let _ = db.upsert_playlist_file(&file_path, &name, count).await;
                            }
                            db.get_playlists().await.unwrap_or_default()
                        },
                        Message::PlaylistsLoaded,
                    );
                    Task::batch([save_task, reload_detail])
                } else {
                    Task::none()
                }
            }
            ContextMenuAction::AddArtistToPlaylist {
                artist,
                playlist_id,
            } => {
                let playlist = self.playlist_by_id(playlist_id);
                if let Some(p) = playlist {
                    let path = PathBuf::from(&p.file_path);
                    let file_path = p.file_path.clone();
                    let name = p.name.clone();
                    let db = self.db.clone();
                    let filter = TrackFilter::artist(artist.artist.as_ref());
                    let is_open = self.playlists.selected_id == Some(playlist_id);
                    let reload_detail = if is_open {
                        self.resolve_playlist_intent(playlist_id, PlaylistOpenIntent::View)
                    } else {
                        Task::none()
                    };

                    let save_task = Task::perform(
                        async move {
                            let tracks = db
                                .get_all_track_rows_filtered(
                                    Some(&filter),
                                    SortField::TrackNum,
                                    SortDirection::Ascending,
                                )
                                .await
                                .unwrap_or_default();

                            let wrote = tokio::task::spawn_blocking(move || {
                                playlist::append_tracks_to_m3u(&path, &tracks)
                            })
                            .await
                            .ok()
                            .and_then(|r| r.ok());

                            if let Some(count) = wrote {
                                let _ = db.upsert_playlist_file(&file_path, &name, count).await;
                            }
                            db.get_playlists().await.unwrap_or_default()
                        },
                        Message::PlaylistsLoaded,
                    );
                    Task::batch([save_task, reload_detail])
                } else {
                    Task::none()
                }
            }
        }
    }

    pub(super) fn render_duplicate_prompt<'a>(
        &'a self,
        prompt: &'a DuplicatePromptState,
        theme_cfg: &'a ThemeConfig,
    ) -> Element<'a, Message> {
        let (header, message_text) = match &prompt.target {
            DuplicateTarget::Playlist {
                track,
                playlist_name,
                ..
            } => (
                "Track Already in Playlist",
                format!(
                    "\"{}\" by {} is already in playlist \"{}\". Do you want to add it again?",
                    track.title,
                    track.display_artist(),
                    playlist_name
                ),
            ),
            DuplicateTarget::Queue { track } => (
                "Track Already in Queue",
                format!(
                    "\"{}\" by {} is already in the playback queue. Do you want to add it again?",
                    track.title,
                    track.display_artist()
                ),
            ),
        };

        let backdrop = mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                    ..Default::default()
                }),
        )
        .on_press(Message::CancelDuplicatePrompt);

        let font = self.fonts.ui_font();
        let dialog_content = column![
            row![
                icons::icon(icons::DISC_SVG, 16.0, theme_cfg.primary),
                Space::new().width(8),
                component::text(font, header)
                    .size(font.px(14.0))
                    .color(theme_cfg.text),
            ]
            .align_y(Alignment::Center),
            Space::new().height(12),
            component::text(font, message_text)
                .size(font.px(12.0))
                .color(theme_cfg.text_muted),
            Space::new().height(20),
            row![
                Space::new().width(Length::Fill),
                button(
                    component::text(font, "Cancel")
                        .size(font.px(12.0))
                        .color(theme_cfg.text_muted)
                )
                .padding([6, 14])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::CancelDuplicatePrompt),
                Space::new().width(8),
                button(
                    component::text(font, "Add anyway")
                        .size(font.px(12.0))
                        .color(Color::WHITE)
                )
                .padding([6, 14])
                .style(styles::primary_button(theme_cfg))
                .on_press(Message::ConfirmDuplicatePrompt),
            ]
            .align_y(Alignment::Center),
        ]
        .width(Length::Fixed(380.0));

        let dialog_box = container(dialog_content)
            .padding(20)
            .style(styles::dialog_container(theme_cfg));

        let centered_dialog = iced::widget::center(dialog_box)
            .width(Length::Fill)
            .height(Length::Fill);

        iced::widget::stack([backdrop.boxed(), centered_dialog.boxed()]).boxed()
    }
}
