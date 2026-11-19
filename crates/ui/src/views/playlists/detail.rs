use iced::widget::{column, container, mouse_area, row, scrollable, text_input, Space, Widget};
use iced::{Alignment, Element, Length};

use ::queue::{queue_window_for_start, queue_window_start, QueueStore, QUEUE_ROW_HEIGHT};
use component::{self, button, icons, styles, text, TrackMenuContext};
use model::{format_template, FieldLookup};
use theme::ThemeConfig;

use crate::message::Message;

use super::*;

impl PlaylistsView {
    pub fn on_detail_click(&mut self, playlist_id: i64, index: usize) -> bool {
        let now = Instant::now();
        let is_double_click = if let Some((last_id, last_index, last_time)) = self.last_detail_click
        {
            last_id == playlist_id
                && last_index == index
                && now.duration_since(last_time).as_millis() <= 400
        } else {
            false
        };

        self.selected_detail_track = Some((playlist_id, index));
        if is_double_click {
            self.last_detail_click = None;
            true
        } else {
            self.last_detail_click = Some((playlist_id, index, now));
            false
        }
    }

    /// Moves an open-playlist row, permuting display items and raw file
    /// entries identically so the file rewrite preserves every line.
    pub fn move_detail_entry(&mut self, from: usize, to: usize) -> bool {
        let len = self.detail_items.len();
        if from >= len || to >= len || from == to || self.detail_entries.len() != len {
            return false;
        }
        let item = self.detail_items.remove(from);
        self.detail_items.insert(to, item);
        let entry = self.detail_entries.remove(from);
        self.detail_entries.insert(to, entry);
        if !self.rendered_detail_items.is_empty()
            && from < self.rendered_detail_items.len()
            && to < self.rendered_detail_items.len()
        {
            let item = self.rendered_detail_items.remove(from);
            self.rendered_detail_items.insert(to, item);
        }
        true
    }

    /// Removes an open-playlist row, keeping display items and raw file
    /// entries 1:1 so the file rewrite preserves every remaining line.
    /// Selection/hover/click state is shifted or cleared to match.
    pub fn remove_detail_entry(&mut self, playlist_id: i64, index: usize) -> bool {
        if self.selected_id != Some(playlist_id) {
            return false;
        }
        let len = self.detail_items.len();
        if index >= len || self.detail_entries.len() != len {
            return false;
        }
        self.detail_items.remove(index);
        self.detail_entries.remove(index);
        if index < self.rendered_detail_items.len() {
            self.rendered_detail_items.remove(index);
        }
        self.detail_tracks_str = format!("{} tracks", self.detail_items.len());
        self.selected_detail_track = match self.selected_detail_track {
            Some((id, i)) if id == playlist_id => {
                if i == index {
                    None
                } else if i > index {
                    Some((id, i - 1))
                } else {
                    Some((id, i))
                }
            }
            other => other,
        };
        self.hovered_detail_row = match self.hovered_detail_row {
            Some((id, i)) if id == playlist_id && i == index => None,
            Some((id, i)) if id == playlist_id && i > index => Some((id, i - 1)),
            other => other,
        };
        if self
            .last_detail_click
            .is_some_and(|(id, i, _)| id == playlist_id && i == index)
        {
            self.last_detail_click = None;
        } else if let Some((id, i, t)) = self.last_detail_click {
            if id == playlist_id && i > index {
                self.last_detail_click = Some((id, i - 1, t));
            }
        }
        true
    }

    pub(super) fn view_detail<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        dragging_detail: Option<(i64, usize)>,
        playlists_fmt: &'a layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
        let playlist_id = self.selected_id.unwrap_or(-1);

        let header: Element<'a, Message> = if self.renaming {
            row![
                text_input("Playlist name...", &self.rename_input)
                    .size(13)
                    .padding([6, 10])
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::RenamePlaylistNameChanged)
                    .on_submit(Message::RenamePlaylistSubmitted(
                        playlist_id,
                        self.rename_input.clone()
                    ))
                    .width(Length::Fixed(280.0)),
                Space::new().width(8),
                button(text(font, "Save").size(12).color(theme_cfg.text))
                    .padding([6, 12])
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press(Message::RenamePlaylistSubmitted(
                        playlist_id,
                        self.rename_input.clone()
                    )),
                Space::new().width(4),
                button(text(font, "Cancel").size(12).color(theme_cfg.text_muted))
                    .padding([6, 12])
                    .style(styles::ghost_button(theme_cfg, false))
                    .on_press(Message::RenamePlaylistCancelled),
            ]
            .align_y(Alignment::Center)
            .boxed()
        } else {
            column![
                text(font, self.detail_name.as_str())
                    .size(15)
                    .color(theme_cfg.text),
                text(font, self.detail_tracks_str.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted),
            ]
            .spacing(2)
            .boxed()
        };

        let actions = row![
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::PLAY_SVG,
                "Play",
                false,
                Message::PlayPlaylist(playlist_id),
            ),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::NEXT_SVG,
                "Play next",
                false,
                Message::PlaylistPlayNext(playlist_id),
            ),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::QUEUE_SVG,
                "Add to queue",
                false,
                Message::PlaylistToQueue(playlist_id),
            ),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::REFRESH_SVG,
                "Rename",
                false,
                Message::RenamePlaylist(playlist_id),
            ),
            component::chip_with_icon(
                theme_cfg,
                font,
                icons::TRASH_SVG,
                "Delete",
                false,
                Message::DeletePlaylist(playlist_id),
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        // Title on the left, actions on the right — mirroring `queue_header`.
        let header_block = container(
            row![container(header).width(Length::Fill), actions,]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding(iced::Padding {
            top: 6.0,
            right: 16.0,
            bottom: 8.0,
            left: 16.0,
        })
        .width(Length::Fill);

        let mut list = column![].spacing(0).width(Length::Fill);
        let total = self.detail_items.len();
        let first_visible =
            (self.last_detail_scroll.get().max(0.0) / QUEUE_ROW_HEIGHT).floor() as usize;
        let cur_win_start = queue_window_start(total, first_visible);

        if self.detail_items.is_empty() {
            list = list.push(component::empty_state(
                theme_cfg,
                font,
                "This playlist is empty",
                "Add tracks from the Library or save a queue as a playlist",
            ));
        } else {
            let (start, end) = queue_window_for_start(total, cur_win_start);
            let top_spacer = start as f32 * QUEUE_ROW_HEIGHT;
            if top_spacer > 0.0 {
                list = list.push(Space::new().height(top_spacer).boxed());
            }

            // While a handle-drag is held, hovering a row live-reorders;
            // otherwise hovering just reveals that row's drag handle in place
            // of the track number.
            let row_hover_enter = |index: usize| {
                if dragging_detail.is_some() {
                    Message::PlaylistDetailDraggedOver { playlist_id, index }
                } else {
                    Message::PlaylistDetailHovered(Some((playlist_id, index)))
                }
            };
            for (offset, item) in self.detail_items[start..end].iter().enumerate() {
                let index = start + offset;
                let available = item.track_id.is_some();
                let on_right_press = item.track_id.map(|track_id| Message::OpenTrackContextMenu {
                    track_id,
                    context: TrackMenuContext::Playlist { playlist_id, index },
                });

                let (primary, secondary, details) =
                    if let Some(cached) = self.rendered_detail_items.get(index) {
                        (
                            std::borrow::Cow::Borrowed(cached.primary.as_str()),
                            std::borrow::Cow::Borrowed(cached.secondary.as_str()),
                            std::borrow::Cow::Borrowed(cached.details.as_str()),
                        )
                    } else {
                        let track_row = item
                            .track_id
                            .and_then(|tid| self.detail_tracks.iter().find(|t| t.id == tid));
                        let lookup = |key: &str| -> Option<String> {
                            if let Some(t) = track_row {
                                if let Some(v) = t.lookup(key) {
                                    return Some(v.into_owned());
                                }
                            }
                            match key {
                                "title" => Some(item.title.clone()),
                                "artist" => Some(item.artist.clone()),
                                "duration_label" | "duration" => Some(item.duration_label.clone()),
                                "id" => item.track_id.map(|id| id.to_string()),
                                _ => None,
                            }
                        };
                        (
                            std::borrow::Cow::Owned(format_template(
                                &playlists_fmt.detail_primary,
                                &lookup,
                            )),
                            std::borrow::Cow::Owned(format_template(
                                &playlists_fmt.detail_secondary,
                                &lookup,
                            )),
                            std::borrow::Cow::Owned(format_template(
                                &playlists_fmt.detail_details,
                                &lookup,
                            )),
                        )
                    };

                list = list.push(
                    component::TrackRow {
                        theme: theme_cfg,
                        font,
                        index,
                        title: primary,
                        artist: secondary,
                        duration: details,
                        highlighted: self.selected_detail_track == Some((playlist_id, index)),
                        dragging: dragging_detail == Some((playlist_id, index)),
                        show_handle: dragging_detail == Some((playlist_id, index))
                            || self.hovered_detail_row == Some((playlist_id, index)),
                        on_press: available
                            .then_some(Message::PlaylistDetailClicked { playlist_id, index }),
                        on_right_press,
                        drag_start: Message::PlaylistDetailDragStarted { playlist_id, index },
                        drag_release: Message::PlaylistDetailDragReleased,
                        hover_enter: row_hover_enter(index),
                        hover_exit: Message::PlaylistDetailHovered(None),
                        on_remove: Message::PlaylistRemoveTrack { playlist_id, index },
                    }
                    .view(),
                );
            }
            let bottom_spacer = total.saturating_sub(end) as f32 * QUEUE_ROW_HEIGHT;
            if bottom_spacer > 0.0 {
                list = list.push(Space::new().height(bottom_spacer).boxed());
            }
        }

        let scroll_offset = &self.last_detail_scroll;
        let hovered_row = self.hovered_detail_row;
        column![
            header_block,
            scrollable(list)
                .id(iced::widget::Id::new("playlist_detail_scroll"))
                .on_scroll(move |scroll| {
                    let y = scroll.viewport.absolute_offset().y;
                    scroll_offset.set(y);
                    if total > 0 && QUEUE_ROW_HEIGHT > 0.0 {
                        let fv = (y.max(0.0) / QUEUE_ROW_HEIGHT).floor() as usize;
                        if queue_window_start(total, fv) != cur_win_start {
                            return Some(Message::PlaylistDetailHovered(hovered_row));
                        }
                    }
                    None::<Message>
                })
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg)),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .boxed()
    }

    // ── Queues ──────────────────────────────────────────────────────────────

    pub(crate) fn view_queues<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        store: &'a QueueStore,
        dragging_track: Option<(u64, usize)>,
        queue_fmt: &'a layout::QueueFormatConfig,
    ) -> Element<'a, Message> {
        if store.is_empty() {
            return component::empty_state(
                theme_cfg,
                font,
                "No queues open",
                "Play a playlist or add tracks from the library to start a queue",
            );
        }

        // Queue selector chips.
        let mut chips = row![].spacing(6).align_y(Alignment::Center);
        for (chip_idx, queue) in store.summaries.iter().enumerate() {
            let is_active = store.active == Some(queue.id);
            let chip_label = self
                .rendered_queue_chips
                .get(chip_idx)
                .filter(|(id, _)| *id == queue.id)
                .map(|(_, label)| label.as_str())
                .unwrap_or(queue.name.as_str());
            let chip_btn = component::chip(
                theme_cfg,
                font,
                chip_label,
                is_active,
                Message::QueueSelect(queue.id),
            );
            chips = chips.push(mouse_area(chip_btn).on_right_press(
                Message::OpenQueueContextMenu {
                    id: queue.id,
                    name: queue.name.clone(),
                },
            ));
        }
        let chips = chips.wrap();

        let Some(active) = store.active_queue() else {
            let chips = container(chips).padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 4.0,
                left: 16.0,
            });
            let Some(id) = store.active else {
                return column![
                    chips,
                    component::empty_state(
                        theme_cfg,
                        font,
                        "No active queue",
                        "Select a queue to view it",
                    )
                ]
                .spacing(8)
                .boxed();
            };
            // A selection exists but its tracks are still loading (queue
            // switch or delete fallback). The summary already knows the name
            // and track count, so render the real header and only the track
            // list area resolves once loading finishes.
            let name = store
                .summary(id)
                .map(|s| s.name.as_str())
                .unwrap_or("Queue");
            return column![
                chips,
                Self::queue_header(theme_cfg, font, name, &self.queue_header_tracks_str, id),
                self.queue_save_form(theme_cfg, font),
                component::empty_state(theme_cfg, font, "Loading queue…", "Fetching tracks"),
            ]
            .height(Length::Fill)
            .width(Length::Fill)
            .boxed();
        };

        let active_id = active.id;

        let header = Self::queue_header(
            theme_cfg,
            font,
            &active.name,
            &self.queue_header_tracks_str,
            active_id,
        );

        let save_form: Element<'a, Message> = self.queue_save_form(theme_cfg, font);

        // Virtualized track list: only the window around the visible scroll
        // range is built each frame; spacers keep the scrollbar proportional
        // to the whole queue (same approach as the library view).
        let mut list = column![].spacing(0).width(Length::Fill);
        if active.is_empty() {
            list = list.push(component::empty_state(
                theme_cfg,
                font,
                "Queue is empty",
                "Add tracks from the library or a playlist",
            ));
        } else {
            let total = active.len();
            let (start, end) = queue_window_for_start(total, self.queue_window_start);
            let top_spacer = start as f32 * QUEUE_ROW_HEIGHT;
            if top_spacer > 0.0 {
                list = list.push(Space::new().height(top_spacer).boxed());
            }
            let current = active.current_index();
            // While a handle-drag is held, hovering a row live-reorders (like
            // the tab bar); otherwise hovering just reveals that row's drag
            // handle in place of the track number.
            let row_hover_enter = |index: usize| {
                if dragging_track.is_some() {
                    Message::QueueDraggedOver {
                        queue_id: active_id,
                        index,
                    }
                } else {
                    Message::QueueRowHovered(Some((active_id, index)))
                }
            };
            for (offset, track) in active.tracks[start..end].iter().enumerate() {
                let index = start + offset;
                let is_current = current == Some(index);
                let (primary, secondary, details) = if let Some(cached) =
                    self.rendered_queue_window.get(offset)
                {
                    (
                        std::borrow::Cow::Borrowed(cached.primary.as_str()),
                        std::borrow::Cow::Borrowed(cached.secondary.as_str()),
                        std::borrow::Cow::Borrowed(cached.details.as_str()),
                    )
                } else {
                    (
                        std::borrow::Cow::Owned(format_template(&queue_fmt.track_primary, track)),
                        std::borrow::Cow::Owned(format_template(&queue_fmt.track_secondary, track)),
                        std::borrow::Cow::Owned(format_template(&queue_fmt.track_details, track)),
                    )
                };
                list = list.push(
                    component::TrackRow {
                        theme: theme_cfg,
                        font,
                        index,
                        title: primary,
                        artist: secondary,
                        duration: details,
                        highlighted: is_current
                            || self.selected_queue_track == Some((active_id, index)),
                        dragging: dragging_track == Some((active_id, index)),
                        show_handle: dragging_track == Some((active_id, index))
                            || self.hovered_queue_row == Some((active_id, index)),
                        on_press: Some(Message::QueueClicked {
                            queue_id: active_id,
                            index,
                        }),
                        on_right_press: Some(Message::OpenTrackContextMenu {
                            track_id: track.id,
                            context: TrackMenuContext::Queue {
                                queue_id: active_id,
                                index,
                            },
                        }),
                        drag_start: Message::QueueDragStarted {
                            queue_id: active_id,
                            index,
                        },
                        drag_release: Message::QueueDragReleased,
                        hover_enter: row_hover_enter(index),
                        hover_exit: Message::QueueRowHovered(None),
                        on_remove: Message::QueueRemoveTrack {
                            queue_id: active_id,
                            index,
                        },
                    }
                    .view(),
                );
            }
            let bottom_spacer = total.saturating_sub(end) as f32 * QUEUE_ROW_HEIGHT;
            if bottom_spacer > 0.0 {
                list = list.push(Space::new().height(bottom_spacer).boxed());
            }
        }

        let scroll_offset = &self.last_queue_scroll;
        let total_tracks = active.len();
        let cur_window_start = self.queue_window_start;

        column![
            container(chips).padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 4.0,
                left: 16.0,
            }),
            header,
            save_form,
            scrollable(list)
                .id(iced::widget::Id::new("queue_scroll"))
                .on_scroll(move |scroll| {
                    let y = scroll.viewport.absolute_offset().y;
                    scroll_offset.set(y);
                    if total_tracks > 0 && QUEUE_ROW_HEIGHT > 0.0 {
                        let first_visible = (y.max(0.0) / QUEUE_ROW_HEIGHT).floor() as usize;
                        let desired = queue_window_start(total_tracks, first_visible);
                        if desired != cur_window_start {
                            return Some(Message::QueueScrolled(scroll.viewport));
                        }
                    }
                    None
                })
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg)),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .boxed()
    }
}
