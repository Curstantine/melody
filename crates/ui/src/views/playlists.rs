use core_engine::{
    format_template, FieldLookup, PlaylistEntry, PlaylistSummary, SortDirection, TrackRow,
};
use iced::widget::{
    button, column, container, image, mouse_area, row, scrollable, text_input, Space, Widget,
};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length};
use std::collections::HashMap;
use std::time::Instant;

use crate::components::{self, text, TrackMenuContext};
use crate::icons;
use crate::message::{Message, PlaylistItem, PlaylistSection, PlaylistSortField, ViewMode};
use crate::queue::{queue_window_for_start, queue_window_start, QueueStore, QUEUE_ROW_HEIGHT};
use crate::theme::{styles, ThemeConfig};

/// Square playlist tile size in the Playlists catalog grid.
pub const PLAYLIST_TILE_SIZE: f32 = 150.0;

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
    /// Matched indexed tracks for the open playlist.
    pub detail_tracks: Vec<TrackRow>,
    /// Raw M3U entries 1:1 with `detail_items` (including unmatched lines),
    /// kept so a drag-reorder can rewrite the file without losing data.
    pub detail_entries: Vec<PlaylistEntry>,
    /// Single-clicked detail row awaiting a potential double-click.
    pub last_detail_click: Option<(i64, usize, Instant)>,
    /// Single-click selection highlight in the open playlist.
    pub selected_detail_track: Option<(i64, usize)>,
    /// Open-playlist row under the cursor (`#` swaps to handle on hover).
    pub hovered_detail_row: Option<(i64, usize)>,

    pub creating: bool,
    pub new_name_input: String,
    pub renaming: bool,
    pub rename_input: String,
    pub saving_queue: bool,
    pub queue_save_input: String,
    /// Single-clicked queue track awaiting a potential double-click (mirrors
    /// [`crate::views::library::LibraryView::last_track_click`], but keyed by
    /// `(queue_id, index)` since a queue may hold the same track twice).
    pub last_queue_click: Option<(u64, usize, Instant)>,
    /// Single-click selection highlight, distinct from the playing cursor.
    pub selected_queue_track: Option<(u64, usize)>,
    /// Queue row under the cursor. The row's `#` slot swaps to the drag
    /// handle while hovered (or dragged), keeping rows clean otherwise.
    pub hovered_queue_row: Option<(u64, usize)>,
    /// First rendered track of the virtualized queue detail window. Only the
    /// window around the visible scroll range is built each frame, mirroring
    /// the library view; spacers keep the scrollbar proportional.
    pub queue_window_start: usize,
    /// Cached cover handles for playlist preview tiles (up to 4 tracks per playlist).
    pub playlist_covers: HashMap<i64, Vec<image::Handle>>,
}

impl PlaylistsView {
    pub fn new(view_mode: ViewMode, sort_field: PlaylistSortField) -> Self {
        Self {
            section: PlaylistSection::Playlists,
            view_mode,
            sort_field,
            sort_dir: SortDirection::Ascending,
            playlists: Vec::new(),
            playlist_covers: HashMap::new(),
            selected_id: None,
            detail_name: String::new(),
            detail_path: String::new(),
            detail_items: Vec::new(),
            detail_tracks: Vec::new(),
            detail_entries: Vec::new(),
            last_detail_click: None,
            selected_detail_track: None,
            hovered_detail_row: None,
            creating: false,
            new_name_input: String::new(),
            renaming: false,
            rename_input: String::new(),
            saving_queue: false,
            queue_save_input: String::new(),
            last_queue_click: None,
            selected_queue_track: None,
            hovered_queue_row: None,
            queue_window_start: 0,
        }
    }

    pub fn set_section(&mut self, section: PlaylistSection) {
        self.section = section;
    }

    /// Resets the queue detail window to the top (queue switch, delete
    /// fallback, rebuild). Callers also snap the scrollable to the top since
    /// iced preserves scroll offsets across content swaps.
    pub fn reset_queue_window(&mut self) {
        self.queue_window_start = 0;
        self.last_queue_click = None;
        self.selected_queue_track = None;
        self.hovered_queue_row = None;
    }

    /// Moves the queue detail window from an absolute scroll offset in pixels.
    /// Returns `true` when the window moved. Mirrors
    /// [`crate::views::library::LibraryView::on_scroll_offset`], but the data
    /// is already in memory so no follow-up load is needed.
    pub fn on_queue_scroll_offset(&mut self, scroll_y: f32, total: usize) -> bool {
        if total == 0 || QUEUE_ROW_HEIGHT <= 0.0 {
            return false;
        }
        let first_visible = (scroll_y.max(0.0) / QUEUE_ROW_HEIGHT).floor() as usize;
        let desired = queue_window_start(total, first_visible);
        if desired == self.queue_window_start {
            return false;
        }
        self.queue_window_start = desired;
        true
    }

    /// Handles a queue track click. Returns `true` on double-click (playback
    /// should start), mirroring
    /// [`crate::views::library::LibraryView::on_track_click`].
    pub fn on_queue_click(&mut self, queue_id: u64, index: usize) -> bool {
        let now = Instant::now();
        let is_double_click = if let Some((last_id, last_index, last_time)) = self.last_queue_click
        {
            last_id == queue_id
                && last_index == index
                && now.duration_since(last_time).as_millis() <= 400
        } else {
            false
        };

        self.selected_queue_track = Some((queue_id, index));
        if is_double_click {
            self.last_queue_click = None;
            true
        } else {
            self.last_queue_click = Some((queue_id, index, now));
            false
        }
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
        let valid_ids: std::collections::HashSet<i64> =
            self.playlists.iter().map(|p| p.id).collect();
        self.playlist_covers.retain(|id, _| valid_ids.contains(id));
        if self.selected_id.is_some_and(|id| !valid_ids.contains(&id)) {
            self.selected_id = None;
            self.detail_items.clear();
            self.detail_entries.clear();
            self.detail_name.clear();
            self.detail_path.clear();
        }
    }

    pub fn set_playlist_covers(&mut self, covers: Vec<(i64, Vec<Vec<u8>>)>) {
        for (id, track_bytes) in covers {
            let handles: Vec<image::Handle> = track_bytes
                .into_iter()
                .filter(|b| !b.is_empty())
                .map(image::Handle::from_bytes)
                .collect();
            if !handles.is_empty() {
                self.playlist_covers.insert(id, handles);
            } else {
                self.playlist_covers.remove(&id);
            }
        }
    }

    pub fn set_detail(
        &mut self,
        playlist_id: i64,
        name: String,
        file_path: String,
        items: Vec<PlaylistItem>,
        tracks: Vec<TrackRow>,
        entries: Vec<PlaylistEntry>,
    ) {
        self.selected_id = Some(playlist_id);
        self.detail_name = name;
        self.detail_path = file_path;
        self.detail_items = items;
        self.detail_tracks = tracks;
        self.detail_entries = entries;
        self.renaming = false;
        self.rename_input.clear();
        self.last_detail_click = None;
        self.selected_detail_track = None;
        self.hovered_detail_row = None;
    }

    pub fn clear_detail(&mut self) {
        self.selected_id = None;
        self.detail_name.clear();
        self.detail_path.clear();
        self.detail_items.clear();
        self.detail_tracks.clear();
        self.detail_entries.clear();
        self.last_detail_click = None;
        self.selected_detail_track = None;
        self.hovered_detail_row = None;
        self.renaming = false;
    }

    /// Handles an open-playlist row click. Returns `true` on double-click
    /// (playback should start).
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
        dragging_track: Option<(u64, usize)>,
        dragging_detail: Option<(i64, usize)>,
        playlists_fmt: &'a crate::layout::PlaylistsFormatConfig,
        queue_fmt: &'a crate::layout::QueueFormatConfig,
    ) -> Element<'a, Message> {
        let toolbar = self.view_toolbar(theme_cfg);
        let content: Element<'a, Message> = match self.section {
            PlaylistSection::Playlists => {
                if self.selected_id.is_some() {
                    self.view_detail(theme_cfg, dragging_detail, playlists_fmt)
                } else {
                    self.view_catalog(theme_cfg, playlists_fmt)
                }
            }
            PlaylistSection::Queues => {
                self.view_queues(theme_cfg, store, dragging_track, queue_fmt)
            }
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
            .boxed()
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
            Space::new().width(Length::Fill),
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
                    "New queue",
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
            .boxed()
    }

    // ── Playlist catalog ────────────────────────────────────────────────────

    fn view_catalog<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        playlists_fmt: &'a crate::layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
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
                    ("Date added", PlaylistSortField::DateAdded),
                ],
                self.sort_field,
                self.sort_dir,
                Message::PlaylistSortChanged,
            ),
            Space::new().width(Length::Fill),
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
            ViewMode::List => self.view_catalog_list(theme_cfg, playlists_fmt),
            ViewMode::Tile => self.view_catalog_tiles(theme_cfg, playlists_fmt),
        };

        column![header, content]
            .spacing(2)
            .height(Length::Fill)
            .width(Length::Fill)
            .boxed()
    }

    fn view_catalog_list<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        playlists_fmt: &'a crate::layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
        let mut list = column![].spacing(2).width(Length::Fill);

        for idx in self.sorted_indices() {
            let playlist = &self.playlists[idx];
            let is_selected = self.selected_id == Some(playlist.id);

            let line1 = format_template(&playlists_fmt.list_line1, playlist);
            let line2 = format_template(&playlists_fmt.list_line2, playlist);
            let details_label = format_template(&playlists_fmt.list_details, playlist);

            let subtitle = text(theme_cfg, line2)
                .size(11)
                .color(theme_cfg.text_muted_color());

            let title_block = column![
                text(theme_cfg, line1)
                    .size(13)
                    .color(theme_cfg.text_color()),
                subtitle,
            ]
            .spacing(2)
            .width(Length::Fill);

            let details = column![
                text(theme_cfg, details_label)
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

            let row_content = row![title_block, details]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding(iced::Padding {
                    top: 4.0,
                    right: 8.0,
                    bottom: 4.0,
                    left: 16.0,
                });

            let open_row = mouse_area(
                button(row_content)
                    .style(styles::tile(theme_cfg))
                    .width(Length::Fill)
                    .padding(0)
                    .on_press(Message::SelectPlaylist(playlist.id)),
            )
            .on_right_press(Message::OpenPlaylistContextMenu(playlist.clone()));

            let play_btn = button(icons::icon(
                icons::PLAY_SVG,
                12.0,
                theme_cfg.primary_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding([4, 6])
            .on_press(Message::PlayPlaylist(playlist.id));

            let delete_btn = button(icons::icon(
                icons::TRASH_SVG,
                12.0,
                theme_cfg.text_muted_color(),
            ))
            .style(styles::icon_control_button(theme_cfg))
            .padding([4, 6])
            .on_press(Message::DeletePlaylist(playlist.id));

            // The highlight wraps the whole row so the background spans the
            // content and the action buttons.
            list = list.push(
                container(
                    row![open_row, play_btn, delete_btn]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .padding(iced::Padding {
                            top: 0.0,
                            right: 12.0,
                            bottom: 0.0,
                            left: 0.0,
                        }),
                )
                .style(styles::table_row_container(theme_cfg, is_selected))
                .width(Length::Fill)
                .boxed(),
            );
        }

        scrollable(list)
            .id(iced::widget::Id::new("playlists_scroll"))
            .height(Length::Fill)
            .style(styles::scrollable_style(theme_cfg))
            .boxed()
    }

    fn view_catalog_tiles<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        playlists_fmt: &'a crate::layout::PlaylistsFormatConfig,
    ) -> Element<'a, Message> {
        let mut grid = column![].spacing(12).width(Length::Fill);
        let mut tile_row = row![].spacing(12).align_y(Alignment::Start);
        let mut columns = 0;

        for idx in self.sorted_indices() {
            let playlist = &self.playlists[idx];
            let covers = self.playlist_covers.get(&playlist.id);

            let icon = render_playlist_tile_cover(
                theme_cfg,
                covers,
                PLAYLIST_TILE_SIZE,
                Message::SelectPlaylist(playlist.id),
            );

            let line1 = format_template(&playlists_fmt.tile_line1, playlist);
            let line2 = format_template(&playlists_fmt.tile_line2, playlist);

            let meta = container(
                column![
                    text(theme_cfg, line1).size(12),
                    text(theme_cfg, line2)
                        .size(10)
                        .color(theme_cfg.text_muted_color()),
                ]
                .spacing(3)
                .padding(6),
            )
            .width(PLAYLIST_TILE_SIZE)
            .height(48);

            let meta_btn = button(meta)
                .style(styles::tile(theme_cfg))
                .padding(0)
                .on_press(Message::SelectPlaylist(playlist.id));

            let play_btn = button(icons::icon(icons::PLAY_SVG, 14.0, theme_cfg.text_color()))
                .style(styles::icon_control_button(theme_cfg))
                .padding(4)
                .on_press(Message::PlayPlaylist(playlist.id));

            let delete_btn = button(icons::icon(icons::TRASH_SVG, 14.0, theme_cfg.text_color()))
                .style(styles::icon_control_button(theme_cfg))
                .padding(4)
                .on_press(Message::DeletePlaylist(playlist.id));

            // Action buttons overlay the cover's bottom-right corner, just
            // like the album tile: the actions container is sized to the
            // cover and stacked above the select button.
            let actions = container(row![play_btn, delete_btn].spacing(4))
                .width(PLAYLIST_TILE_SIZE)
                .height(PLAYLIST_TILE_SIZE)
                .align_x(Alignment::End)
                .align_y(Alignment::End)
                .padding(6);

            let cover_block = iced::widget::stack([icon, actions.boxed()]);

            let card_content = column![cover_block, meta_btn].width(PLAYLIST_TILE_SIZE);

            let tile = mouse_area(card_content)
                .on_right_press(Message::OpenPlaylistContextMenu(playlist.clone()));

            tile_row = tile_row.push(tile);
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
        .id(iced::widget::Id::new("playlists_scroll"))
        .height(Length::Fill)
        .style(styles::scrollable_style(theme_cfg))
        .boxed()
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
                Space::new().height(8),
                row![
                    input,
                    Space::new().width(8),
                    create,
                    Space::new().width(4),
                    cancel
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(6),
        )
        .padding(24)
        .width(Length::Fill)
        .boxed()
    }

    // ── Playlist detail ─────────────────────────────────────────────────────

    fn view_detail<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        dragging_detail: Option<(i64, usize)>,
        playlists_fmt: &'a crate::layout::PlaylistsFormatConfig,
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
                Space::new().width(4),
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
            .boxed()
        } else {
            column![
                text(theme_cfg, self.detail_name.as_str())
                    .size(15)
                    .color(theme_cfg.text_color()),
                text(theme_cfg, format!("{} tracks", self.detail_items.len()))
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .boxed()
        };

        let actions = row![
            components::chip_with_icon(
                theme_cfg,
                icons::PLAY_SVG,
                "Play",
                false,
                Message::PlayPlaylist(playlist_id),
            ),
            components::chip_with_icon(
                theme_cfg,
                icons::NEXT_SVG,
                "Play next",
                false,
                Message::PlaylistPlayNext(playlist_id),
            ),
            components::chip_with_icon(
                theme_cfg,
                icons::QUEUE_SVG,
                "Add to queue",
                false,
                Message::PlaylistToQueue(playlist_id),
            ),
            components::chip_with_icon(
                theme_cfg,
                icons::REFRESH_SVG,
                "Rename",
                false,
                Message::RenamePlaylist(playlist_id),
            ),
            components::chip_with_icon(
                theme_cfg,
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

        let mut list = column![].spacing(1).width(Length::Fill);
        if self.detail_items.is_empty() {
            list = list.push(components::empty_state(
                theme_cfg,
                "This playlist is empty",
                "Add tracks from the Library or save a queue as a playlist",
            ));
        } else {
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
            for (index, item) in self.detail_items.iter().enumerate() {
                let available = item.is_available();
                let on_right_press = item.track_id.map(|track_id| Message::OpenTrackContextMenu {
                    track_id,
                    context: TrackMenuContext::Playlist { playlist_id, index },
                });

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

                let primary = format_template(&playlists_fmt.detail_primary, &lookup);
                let secondary = format_template(&playlists_fmt.detail_secondary, &lookup);
                let details = format_template(&playlists_fmt.detail_details, &lookup);

                list = list.push(
                    components::TrackRow {
                        theme: theme_cfg,
                        index,
                        title: primary.into(),
                        artist: secondary.into(),
                        duration: details.into(),
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
        }

        column![
            header_block,
            scrollable(list)
                .id(iced::widget::Id::new("playlist_detail_scroll"))
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
        store: &'a QueueStore,
        dragging_track: Option<(u64, usize)>,
        queue_fmt: &'a crate::layout::QueueFormatConfig,
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
        for queue in &store.summaries {
            let is_active = store.active == Some(queue.id);
            let chip_label = format_template(&queue_fmt.chip_format, queue);
            let chip_btn = components::chip(
                theme_cfg,
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
                    components::empty_state(
                        theme_cfg,
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
            let (name, track_count) = store
                .summary(id)
                .map(|s| (s.name.as_str(), s.track_count))
                .unwrap_or(("Queue", 0));
            return column![
                chips,
                Self::queue_header(theme_cfg, name, track_count, id),
                self.queue_save_form(theme_cfg),
                components::empty_state(theme_cfg, "Loading queue…", "Fetching tracks"),
            ]
            .height(Length::Fill)
            .width(Length::Fill)
            .boxed();
        };

        let active_id = active.id;

        let header = Self::queue_header(theme_cfg, &active.name, active.len(), active_id);

        let save_form: Element<'a, Message> = self.queue_save_form(theme_cfg);

        // Virtualized track list: only the window around the visible scroll
        // range is built each frame; spacers keep the scrollbar proportional
        // to the whole queue (same approach as the library view).
        let mut list = column![].spacing(0).width(Length::Fill);
        if active.is_empty() {
            list = list.push(components::empty_state(
                theme_cfg,
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
                let primary = format_template(&queue_fmt.track_primary, track);
                let secondary = format_template(&queue_fmt.track_secondary, track);
                let details = format_template(&queue_fmt.track_details, track);
                list = list.push(
                    components::TrackRow {
                        theme: theme_cfg,
                        index,
                        title: primary.into(),
                        artist: secondary.into(),
                        duration: details.into(),
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
                .on_scroll(|scroll| Message::QueueScrolled(scroll.viewport))
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg)),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .boxed()
    }

    /// Queue title header (name, track count, actions). Rendered from either
    /// the materialized queue or, while tracks are loading, its summary so
    /// the header stays stable across the load.
    fn queue_header<'a>(
        theme_cfg: &'a ThemeConfig,
        name: &'a str,
        track_count: usize,
        active_id: u64,
    ) -> Element<'a, Message> {
        row![
            column![
                text(theme_cfg, name).size(15).color(theme_cfg.text_color()),
                text(theme_cfg, format!("{} tracks in queue", track_count))
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(2)
            .width(Length::Fill),
            components::chip_with_icon(
                theme_cfg,
                icons::PLAYLIST_SVG,
                "Save as playlist",
                false,
                Message::QueueSaveClicked,
            ),
            components::chip_with_icon(
                theme_cfg,
                icons::TRASH_SVG,
                "Delete queue",
                false,
                Message::QueueDelete(active_id),
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 6.0,
            right: 16.0,
            bottom: 8.0,
            left: 16.0,
        })
        .boxed()
    }

    /// "Save queue as playlist" inline form, shown when `saving_queue` is set.
    fn queue_save_form<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        if self.saving_queue {
            row![
                text_input("Save queue as playlist...", &self.queue_save_input)
                    .size(12)
                    .padding([6, 10])
                    .style(styles::search_input_style(theme_cfg))
                    .on_input(Message::QueueSaveNameChanged)
                    .on_submit(Message::QueueSaveSubmitted(self.queue_save_input.clone()))
                    .width(Length::Fixed(280.0)),
                Space::new().width(8),
                button(
                    text(theme_cfg, "Save")
                        .size(12)
                        .color(theme_cfg.text_color())
                )
                .padding([6, 12])
                .style(styles::ghost_button(theme_cfg, false))
                .on_press(Message::QueueSaveSubmitted(self.queue_save_input.clone())),
                Space::new().width(4),
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
            .boxed()
        } else {
            Space::new().height(0).boxed()
        }
    }
}

fn case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
    let a_chars = a.chars().flat_map(|c| c.to_lowercase());
    let b_chars = b.chars().flat_map(|c| c.to_lowercase());
    a_chars.cmp(b_chars)
}

/// Renders either a 2x2 grid of track album covers, a single full-size cover,
/// or a fallback icon inside the playlist tile.
fn render_playlist_tile_cover<'a>(
    theme_cfg: &'a ThemeConfig,
    covers: Option<&Vec<image::Handle>>,
    size: f32,
    on_press: Message,
) -> Element<'a, Message> {
    let radius = theme_cfg.radius_md();
    let border_color = theme_cfg.border_color();

    match covers {
        Some(list) if list.len() >= 4 => {
            let half = (size / 2.0).floor();
            let mut top_row = row![].spacing(0);
            top_row = top_row.push(
                image(list[0].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );
            top_row = top_row.push(
                image(list[1].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );

            let mut bottom_row = row![].spacing(0);
            bottom_row = bottom_row.push(
                image(list[2].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );
            bottom_row = bottom_row.push(
                image(list[3].clone())
                    .width(Length::Fixed(half))
                    .height(Length::Fixed(half))
                    .content_fit(ContentFit::Cover),
            );

            let grid_content = column![top_row, bottom_row].spacing(0);
            let image_layer = container(grid_content)
                .width(size)
                .height(size)
                .clip(true)
                .style(move |_theme| container::Style {
                    border: Border {
                        radius,
                        color: border_color,
                        width: 1.0,
                    },
                    ..Default::default()
                });

            let overlay_btn = button(Space::new().width(Length::Fill).height(Length::Fill))
                .width(size)
                .height(size)
                .style(move |_theme, status| {
                    let bg = match status {
                        button::Status::Active => Color::TRANSPARENT,
                        button::Status::Hovered | button::Status::Pressed => {
                            Color::from_rgba(0.0, 0.0, 0.0, 0.5)
                        }
                        button::Status::Disabled => Color::TRANSPARENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border {
                            radius,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(on_press);

            iced::widget::stack([image_layer.boxed(), overlay_btn.boxed()]).boxed()
        }
        Some(list) if !list.is_empty() => {
            // 1 to 3 covers: render first cover full size (classic music player behavior)
            components::catalog_cover_art(
                theme_cfg,
                Some(&list[0]),
                size,
                radius,
                icons::PLAYLIST_SVG,
                size * 0.34,
                theme_cfg.primary_color(),
                on_press,
            )
        }
        _ => components::catalog_cover_art(
            theme_cfg,
            None,
            size,
            radius,
            icons::PLAYLIST_SVG,
            size * 0.34,
            theme_cfg.primary_color(),
            on_press,
        ),
    }
}
