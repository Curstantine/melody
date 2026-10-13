use core_engine::{PlaylistSummary, SortDirection};
use iced::border::Radius;
use iced::widget::{button, column, container, row, scrollable, text_input, Space};
use iced::{Alignment, Border, Element, Length};

use crate::components::{self, text};
use crate::icons;
use crate::message::{Message, PlaylistItem, PlaylistSection, PlaylistSortField, ViewMode};
use crate::queue::QueueStore;
use crate::theme::{styles, ThemeConfig};

/// Self-contained state and view component for the Playlists tab.
///
/// Playlists are loaded from `playlist_files` references and their contents are
/// read from the M3U file on demand. Queues are rendered from the shared
/// [`QueueStore`], so this view only owns selection/editor UI state.
pub struct PlaylistsView {
    pub section: PlaylistSection,
    pub view_mode: ViewMode,
    pub sort_field: PlaylistSortField,
    pub sort_dir: SortDirection,
    pub playlists: Vec<PlaylistSummary>,
    pub selected_id: Option<i64>,
    pub detail_name: String,
    pub detail_path: String,
    pub detail_items: Vec<PlaylistItem>,

    pub creating: bool,
    pub new_name_input: String,
    pub renaming: bool,
    pub rename_input: String,
    pub saving_queue: bool,
    pub queue_save_input: String,
}

impl PlaylistsView {
    pub fn new(view_mode: ViewMode, sort_field: PlaylistSortField) -> Self {
        Self {
            section: PlaylistSection::Playlists,
            view_mode,
            sort_field,
            sort_dir: SortDirection::Ascending,
            playlists: Vec::new(),
            selected_id: None,
            detail_name: String::new(),
            detail_path: String::new(),
            detail_items: Vec::new(),
            creating: false,
            new_name_input: String::new(),
            renaming: false,
            rename_input: String::new(),
            saving_queue: false,
            queue_save_input: String::new(),
        }
    }

    pub fn set_section(&mut self, section: PlaylistSection) {
        self.section = section;
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    pub fn set_sort_field(&mut self, field: PlaylistSortField) {
        if self.sort_field == field {
            self.sort_dir = match self.sort_dir {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_field = field;
            self.sort_dir = SortDirection::Ascending;
        }
    }

    pub fn set_playlists(&mut self, playlists: Vec<PlaylistSummary>) {
        self.playlists = playlists;
        if self
            .selected_id
            .is_some_and(|id| !self.playlists.iter().any(|p| p.id == id))
        {
            self.selected_id = None;
            self.detail_items.clear();
            self.detail_name.clear();
            self.detail_path.clear();
        }
    }

    pub fn set_detail(
        &mut self,
        playlist_id: i64,
        name: String,
        file_path: String,
        items: Vec<PlaylistItem>,
    ) {
        self.selected_id = Some(playlist_id);
        self.detail_name = name;
        self.detail_path = file_path;
        self.detail_items = items;
        self.renaming = false;
        self.rename_input.clear();
    }

    pub fn clear_detail(&mut self) {
        self.selected_id = None;
        self.detail_name.clear();
        self.detail_path.clear();
        self.detail_items.clear();
        self.renaming = false;
    }

    pub fn len(&self) -> usize {
        self.playlists.len()
    }

    /// Indices into `self.playlists` in the current sort order.
    fn sorted_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.playlists.len()).collect();
        let field = self.sort_field;
        indices.sort_by(|&a, &b| {
            let pa = &self.playlists[a];
            let pb = &self.playlists[b];
            let ord = match field {
                PlaylistSortField::Name => case_insensitive(&pa.name, &pb.name),
                PlaylistSortField::TrackCount => pa
                    .track_count
                    .cmp(&pb.track_count)
                    .then_with(|| case_insensitive(&pa.name, &pb.name)),
                PlaylistSortField::DateAdded => pa
                    .created_timestamp
                    .cmp(&pb.created_timestamp)
                    .then_with(|| case_insensitive(&pa.name, &pb.name)),
            };
            match self.sort_dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
        indices
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        store: &'a QueueStore,
    ) -> Element<'a, Message> {
        let toolbar = self.view_toolbar(theme_cfg);
        let content: Element<'a, Message> = match self.section {
            PlaylistSection::Playlists => {
                if self.selected_id.is_some() {
                    self.view_detail(theme_cfg)
                } else {
                    self.view_catalog(theme_cfg)
                }
            }
            PlaylistSection::Queues => self.view_queues(theme_cfg, store),
        };

        column![toolbar, content]
            .spacing(4)
            .padding(iced::Padding {
                top: 8.0,
                right: 1.0,
                bottom: 0.0,
                left: 0.0,
            })
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_toolbar<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let section_chip = |label: &'static str, section: PlaylistSection, icon: &'static str| {
            components::chip_with_icon(
                theme_cfg,
                icon,
                label,
                self.section == section,
                Message::PlaylistSectionChanged(section),
            )
        };

        let mut bar = row![
            section_chip("Playlists", PlaylistSection::Playlists, icons::PLAYLIST_SVG),
            section_chip("Queues", PlaylistSection::Queues, icons::QUEUE_SVG),
            Space::with_width(Length::Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        match self.section {
            PlaylistSection::Playlists => {
                if self.selected_id.is_some() {
                    bar = bar.push(components::chip_with_icon(
                        theme_cfg,
                        icons::ARROW_LEFT_SVG,
                        "Back",
                        false,
                        Message::SelectPlaylist(-1),
                    ));
                } else {
                    bar = bar
                        .push(components::chip_with_icon(
                            theme_cfg,
                            icons::FOLDER_SVG,
                            "Import",
                            false,
                            Message::ImportPlaylistsClicked,
                        ))
                        .push(components::chip_with_icon(
                            theme_cfg,
                            icons::PLUS_SVG,
                            "New",
                            false,
                            Message::NewPlaylistClicked,
                        ));
                }
            }
            PlaylistSection::Queues => {
                bar = bar.push(components::chip_with_icon(
                    theme_cfg,
                    icons::PLUS_SVG,
                    "New Queue",
                    false,
                    Message::QueueNew,
                ));
            }
        }

        container(bar)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 6.0,
                left: 16.0,
            })
            .into()
    }

    // ── Playlist catalog ────────────────────────────────────────────────────

    fn view_catalog<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        if self.creating {
            return self.view_new_playlist_form(theme_cfg);
        }

        if self.playlists.is_empty() {
            return components::empty_state(
                theme_cfg,
                "No playlists yet",
                "Import an existing .m3u / .m3u8 file or create a new playlist",
            );
        }

        let header = row![
            components::sort_chip_list(
                theme_cfg,
                &[
                    ("Name", PlaylistSortField::Name),
                    ("Tracks", PlaylistSortField::TrackCount),
                    ("Date Added", PlaylistSortField::DateAdded),
                ],
                self.sort_field,
                self.sort_dir,
                Message::PlaylistSortChanged,
            ),
            Space::with_width(Length::Fill),
            components::view_mode_toggle(theme_cfg, self.view_mode, Message::SetPlaylistsViewMode),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        });

        let content = match self.view_mode {
            ViewMode::List => self.view_catalog_list(theme_cfg),
            ViewMode::Tile => self.view_catalog_tiles(theme_cfg),
        };

        column![header, content]
            .spacing(2)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_catalog_list<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let mut list = column![].spacing(2).width(Length::Fill);

        for idx in self.sorted_indices() {
            let playlist = &self.playlists[idx];
            let is_selected = self.selected_id == Some(playlist.id);

            let thumbnail = container(icons::icon(
                icons::PLAYLIST_SVG,
                20.0,
                theme_cfg.primary_color(),
            ))
            .width(42)
            .height(42)
            .center_x(42)
            .center_y(42)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                border: Border {
                    radius: Radius::from(4.0),
                    color: theme_cfg.border_color(),
                    width: 1.0,
                },
                ..Default::default()
            });

            let subtitle = text(theme_cfg, playlist.file_path.as_str())
                .size(11)
                .color(theme_cfg.text_muted_color());

            let title_block = column![
                text(theme_cfg, playlist.name.as_str())
                    .size(13)
                    .color(theme_cfg.text_color()),
                subtitle,
            ]
            .spacing(2)
            .width(Length::Fill);

            let details = column![
                text(theme_cfg, playlist.tracks_label.as_str())
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(
                    theme_cfg,
                    if playlist.managed {
                        "Managed"
                    } else {
                        "External"
                    },
                )
                .size(10)
                .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .align_x(Alignment::End)
            .width(90);

            let row_content = row![thumbnail, Space::with_width(12), title_block, details]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([4, 8]);

            let row_container = container(row_content)
                .style(styles::table_row_container(theme_cfg, is_selected))
                .width(Length::Fill);

            let play_btn = button(icons::icon(
                icons::PLAY_SVG,
                12.0,
                theme_cfg.primary_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding([4, 6])
            .on_press(Message::PlayPlaylist(playlist.id));

            let open_btn = button(icons::icon(
                icons::ARROW_RIGHT_SVG,
                12.0,
                theme_cfg.text_muted_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding([4, 6])
            .on_press(Message::SelectPlaylist(playlist.id));

            list = list.push(
                row![
                    button(row_container)
                        .style(styles::tile(theme_cfg))
                        .width(Length::Fill)
                        .padding(0)
                        .on_press(Message::SelectPlaylist(playlist.id)),
                    play_btn,
                    open_btn,
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 12.0,
                    bottom: 0.0,
                    left: 4.0,
                }),
            );
        }

        scrollable(list)
            .id(scrollable::Id::new("playlists_scroll"))
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .into()
    }

    fn view_catalog_tiles<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        const TILE: f32 = 150.0;
        let mut grid = column![].spacing(12).width(Length::Fill);
        let mut tile_row = row![].spacing(12).align_y(Alignment::Start);
        let mut columns = 0;

        for idx in self.sorted_indices() {
            let playlist = &self.playlists[idx];

            let icon = container(icons::icon(
                icons::PLAYLIST_SVG,
                TILE * 0.34,
                theme_cfg.primary_color(),
            ))
            .width(TILE)
            .height(TILE)
            .center_x(TILE)
            .center_y(TILE)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                border: Border {
                    radius: Radius::from(6.0),
                    color: theme_cfg.border_color(),
                    width: 1.0,
                },
                ..Default::default()
            });

            let meta = container(
                column![
                    text(theme_cfg, playlist.name.as_str())
                        .size(12)
                        .color(theme_cfg.text_color()),
                    text(theme_cfg, playlist.tracks_label.as_str())
                        .size(10)
                        .color(theme_cfg.text_muted_color()),
                ]
                .spacing(3)
                .padding(6),
            )
            .width(TILE)
            .height(48);

            let card = button(container(column![icon, meta]).width(TILE).clip(true))
                .style(styles::tile(theme_cfg))
                .padding(0)
                .on_press(Message::SelectPlaylist(playlist.id));

            tile_row = tile_row.push(card);
            columns += 1;
            if columns == 5 {
                grid = grid.push(tile_row);
                tile_row = row![].spacing(12);
                columns = 0;
            }
        }
        if columns > 0 {
            grid = grid.push(tile_row);
        }

        scrollable(container(grid).width(Length::Fill).padding(iced::Padding {
            left: 16.0,
            right: 16.0,
            ..Default::default()
        }))
        .id(scrollable::Id::new("playlists_scroll"))
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .into()
    }

    fn view_new_playlist_form<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let input = text_input("New playlist name...", &self.new_name_input)
            .size(13)
            .padding([7, 10])
            .style(styles::search_input_style(theme_cfg))
            .on_input(Message::NewPlaylistNameChanged)
            .on_submit(Message::NewPlaylistNameSubmitted(
                self.new_name_input.clone(),
            ))
            .width(Length::Fill);

        let create = button(
            text(theme_cfg, "Create")
                .size(12)
                .color(theme_cfg.text_color()),
        )
        .padding([7, 16])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::NewPlaylistNameSubmitted(
            self.new_name_input.clone(),
        ));

        let cancel = button(
            text(theme_cfg, "Cancel")
                .size(12)
                .color(theme_cfg.text_muted_color()),
        )
        .padding([7, 16])
        .style(styles::ghost_button(theme_cfg, false))
        .on_press(Message::NewPlaylistCancelled);

        container(
            column![
                text(theme_cfg, "Create a new playlist")
                    .size(16)
                    .color(theme_cfg.text_color()),
                text(
                    theme_cfg,
                    "A new .m3u8 file will be created in the Melody playlists directory."
                )
                .size(11)
                .color(theme_cfg.text_muted_color()),
                Space::with_height(8),
                row![
                    input,
                    Space::with_width(8),
                    create,
                    Space::with_width(4),
                    cancel
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(6),
        )
        .padding(24)
        .width(Length::Fill)
        .into()
    }

    // ── Playlist detail ─────────────────────────────────────────────────────

    fn view_detail<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
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
                Space::with_width(8),
                button(
                    text(theme_cfg, "Save")
                        .size(12)
                        .color(theme_cfg.text_color())
                )
                .padding([6, 12])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::RenamePlaylistSubmitted(
                    playlist_id,
                    self.rename_input.clone()
                )),
                Space::with_width(4),
                button(
                    text(theme_cfg, "Cancel")
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                )
                .padding([6, 12])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::RenamePlaylistCancelled),
            ]
            .align_y(Alignment::Center)
            .into()
        } else {
            column![
                text(theme_cfg, self.detail_name.as_str())
                    .size(18)
                    .color(theme_cfg.text_color()),
                text(theme_cfg, self.detail_path.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .into()
        };

        let action = |label: &'static str, icon: &'static str, msg: Message| {
            button(
                row![
                    icons::icon(icon, 12.0, theme_cfg.text_color()),
                    Space::with_width(5),
                    text(theme_cfg, label)
                        .size(11)
                        .color(theme_cfg.text_color()),
                ]
                .align_y(Alignment::Center),
            )
            .padding([5, 10])
            .style(styles::ghost_button(theme_cfg, false))
            .on_press(msg)
        };

        let actions = row![
            action("Play", icons::PLAY_SVG, Message::PlayPlaylist(playlist_id)),
            action(
                "Add to Queue",
                icons::QUEUE_SVG,
                Message::PlaylistToQueue(playlist_id)
            ),
            action(
                "Rename",
                icons::REFRESH_SVG,
                Message::RenamePlaylist(playlist_id)
            ),
            action(
                "Delete",
                icons::TRASH_SVG,
                Message::DeletePlaylist(playlist_id)
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let header_block = container(column![header, Space::with_height(8), actions].spacing(4))
            .padding(iced::Padding {
                top: 8.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
            })
            .width(Length::Fill);

        let mut list = column![].spacing(1).width(Length::Fill);
        if self.detail_items.is_empty() {
            list = list.push(components::empty_state(
                theme_cfg,
                "This playlist is empty",
                "Add tracks from the Library or save a queue as a playlist",
            ));
        } else {
            for (index, item) in self.detail_items.iter().enumerate() {
                let available = item.is_available();
                let title_color = if available {
                    theme_cfg.text_color()
                } else {
                    theme_cfg.text_muted_color()
                };

                let index_label = container(
                    text(theme_cfg, format!("{}", index + 1))
                        .size(12)
                        .color(theme_cfg.text_muted_color()),
                )
                .width(36)
                .center_x(36);

                let title = text(theme_cfg, item.title.as_str())
                    .size(13)
                    .color(title_color);
                let artist = text(theme_cfg, item.artist.as_str())
                    .size(11)
                    .color(theme_cfg.text_muted_color());
                let duration = text(theme_cfg, item.duration_label.as_str())
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(80);

                let row_content = row![
                    index_label,
                    title.width(Length::FillPortion(5)),
                    artist.width(Length::FillPortion(4)),
                    duration,
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .height(26)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 12.0,
                    bottom: 0.0,
                    left: 0.0,
                });

                let row_container = container(row_content)
                    .style(styles::table_row_container(theme_cfg, false))
                    .width(Length::Fill)
                    .height(26);

                let clickable: Element<'a, Message> = if available {
                    iced::widget::mouse_area(row_container)
                        .on_press(Message::PlayPlaylistItem(playlist_id, index))
                        .into()
                } else {
                    row_container.into()
                };

                let add_btn: Element<'a, Message> = if let Some(track_id) = item.track_id {
                    button(icons::icon(
                        icons::PLUS_SVG,
                        12.0,
                        theme_cfg.text_muted_color(),
                    ))
                    .style(styles::icon_control_button(theme_cfg))
                    .padding([3, 5])
                    .on_press(Message::QueueAddTrack(track_id))
                    .into()
                } else {
                    container(icons::icon(
                        icons::ARROW_RIGHT_SVG,
                        12.0,
                        theme_cfg.text_muted_color(),
                    ))
                    .padding([3, 5])
                    .into()
                };

                list = list.push(
                    row![clickable, add_btn]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .padding(iced::Padding {
                            top: 0.0,
                            right: 12.0,
                            bottom: 0.0,
                            left: 4.0,
                        }),
                );
            }
        }

        column![
            header_block,
            scrollable(list)
                .id(scrollable::Id::new("playlist_detail_scroll"))
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg)),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    // ── Queues ──────────────────────────────────────────────────────────────

    fn view_queues<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        store: &'a QueueStore,
    ) -> Element<'a, Message> {
        if store.is_empty() {
            return components::empty_state(
                theme_cfg,
                "No queues open",
                "Play a playlist or add tracks from the library to start a queue",
            );
        }

        // Queue selector chips.
        let mut chips = row![].spacing(6).align_y(Alignment::Center);
        for queue in &store.queues {
            let is_active = store.active == Some(queue.id);
            chips = chips.push(components::chip(
                theme_cfg,
                format!("{} ({})", queue.name, queue.len()),
                is_active,
                Message::QueueSelect(queue.id),
            ));
        }
        let chips = chips.wrap();

        let Some(active) = store.active_queue() else {
            return components::empty_state(
                theme_cfg,
                "No active queue",
                "Select a queue to view it",
            );
        };

        let active_id = active.id;

        let header = row![
            column![
                text(theme_cfg, active.name.as_str())
                    .size(15)
                    .color(theme_cfg.text_color()),
                text(theme_cfg, format!("{} tracks in queue", active.len()))
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .width(Length::Fill),
            components::chip_with_icon(
                theme_cfg,
                icons::PLAYLIST_SVG,
                "Save as Playlist",
                false,
                Message::QueueSaveClicked,
            ),
            components::chip_with_icon(
                theme_cfg,
                icons::TRASH_SVG,
                "Delete Queue",
                false,
                Message::QueueDelete(active_id),
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 4.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        });

        let save_form: Element<'a, Message> = if self.saving_queue {
            row![
                text_input("Save queue as playlist...", &self.queue_save_input)
                    .size(12)
                    .padding([6, 10])
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::QueueSaveNameChanged)
                    .on_submit(Message::QueueSaveSubmitted(self.queue_save_input.clone()))
                    .width(Length::Fixed(280.0)),
                Space::with_width(8),
                button(
                    text(theme_cfg, "Save")
                        .size(12)
                        .color(theme_cfg.text_color())
                )
                .padding([6, 12])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::QueueSaveSubmitted(self.queue_save_input.clone())),
                Space::with_width(4),
                button(
                    text(theme_cfg, "Cancel")
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                )
                .padding([6, 12])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::QueueSaveCancelled),
            ]
            .align_y(Alignment::Center)
            .padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
            })
            .into()
        } else {
            Space::with_height(0).into()
        };

        let mut list = column![].spacing(1).width(Length::Fill);
        if active.is_empty() {
            list = list.push(components::empty_state(
                theme_cfg,
                "Queue is empty",
                "Add tracks from the library or a playlist",
            ));
        } else {
            let current = active.current_index();
            for (index, track) in active.tracks.iter().enumerate() {
                let is_current = current == Some(index);
                let index_label =
                    container(text(theme_cfg, format!("{}", index + 1)).size(12).color(
                        if is_current {
                            theme_cfg.primary_color()
                        } else {
                            theme_cfg.text_muted_color()
                        },
                    ))
                    .width(36)
                    .center_x(36);

                let title = text(theme_cfg, track.title.as_str())
                    .size(13)
                    .color(if is_current {
                        theme_cfg.primary_color()
                    } else {
                        theme_cfg.text_color()
                    });
                let artist = text(theme_cfg, track.display_artist())
                    .size(11)
                    .color(theme_cfg.text_muted_color());

                let row_content = row![
                    index_label,
                    title.width(Length::FillPortion(5)),
                    artist.width(Length::FillPortion(4)),
                    text(theme_cfg, track.formatted_duration())
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                        .width(64),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .height(26);

                let row_container = container(row_content)
                    .style(styles::table_row_container(theme_cfg, is_current))
                    .width(Length::Fill)
                    .height(26);

                let clickable = iced::widget::mouse_area(row_container)
                    .on_press(Message::QueuePlayIndex(index));

                let icon_btn = |icon: &'static str, msg: Message| {
                    button(icons::icon(icon, 12.0, theme_cfg.text_muted_color()))
                        .style(styles::icon_control_button(theme_cfg))
                        .padding([3, 5])
                        .on_press(msg)
                };

                list = list.push(
                    row![
                        clickable,
                        icon_btn(
                            icons::CHEVRON_UP_SVG,
                            Message::QueueMoveTrack {
                                queue_id: active_id,
                                index,
                                up: true
                            }
                        ),
                        icon_btn(
                            icons::CHEVRON_DOWN_SVG,
                            Message::QueueMoveTrack {
                                queue_id: active_id,
                                index,
                                up: false
                            }
                        ),
                        icon_btn(
                            icons::TRASH_SVG,
                            Message::QueueRemoveTrack {
                                queue_id: active_id,
                                index
                            }
                        ),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 0.0,
                        left: 4.0,
                    }),
                );
            }
        }

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
                .id(scrollable::Id::new("queue_scroll"))
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg)),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}

fn case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
    let a_chars = a.chars().flat_map(|c| c.to_lowercase());
    let b_chars = b.chars().flat_map(|c| c.to_lowercase());
    a_chars.cmp(b_chars)
}
