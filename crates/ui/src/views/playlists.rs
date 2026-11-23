use std::{collections::HashMap, time::Instant};

use iced::widget::{column, container, image, row, Space, Widget};
use iced::{Alignment, Element, Length};

use ::queue::QueueStore;
use component::{self, icons};
use config::PlaylistSection;
use model::{
    format_template, FieldLookup, PlaylistSortField, PlaylistSummary, SortDirection, TrackRow,
    ViewMode,
};
use playlist::{PlaylistEntry, PlaylistItem};
use theme::ThemeConfig;

use super::ScrollOffset;
use crate::message::Message;

mod catalog;
mod detail;
mod queue;
/// Square playlist tile size in the Playlists catalog grid.
pub const PLAYLIST_TILE_SIZE: f32 = 150.0;

#[derive(Clone, Debug, Default)]
pub struct RenderedPlaylistSummary {
    pub tile_line1: String,
    pub tile_line2: String,
    pub list_line1: String,
    pub list_line2: String,
    pub list_details: String,
}

#[derive(Clone, Debug, Default)]
pub struct RenderedPlaylistItem {
    pub primary: String,
    pub secondary: String,
    pub details: String,
}

#[derive(Clone, Debug, Default)]
pub struct RenderedQueueTrack {
    pub primary: String,
    pub secondary: String,
    pub details: String,
}

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
    /// Pre-rendered playlist detail strings so detail rendering does not allocate per frame.
    pub detail_tracks_str: String,
    pub rendered_playlists: Vec<RenderedPlaylistSummary>,
    pub rendered_detail_items: Vec<RenderedPlaylistItem>,
    pub queue_header_tracks_str: String,
    pub rendered_queue_chips: Vec<(u64, String)>,
    pub rendered_queue_window: Vec<RenderedQueueTrack>,
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
    pub last_playlists_scroll: ScrollOffset,
    pub last_detail_scroll: ScrollOffset,
    pub last_queue_scroll: ScrollOffset,
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
            detail_tracks_str: String::new(),
            rendered_playlists: Vec::new(),
            rendered_detail_items: Vec::new(),
            queue_header_tracks_str: String::new(),
            rendered_queue_chips: Vec::new(),
            rendered_queue_window: Vec::new(),
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
            last_playlists_scroll: ScrollOffset::default(),
            last_detail_scroll: ScrollOffset::default(),
            last_queue_scroll: ScrollOffset::default(),
        }
    }

    /// Resets the queue detail window to the top (queue switch, delete
    /// fallback, rebuild). Callers also snap the scrollable to the top since
    /// iced preserves scroll offsets across content swaps.
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
        self.last_detail_scroll.set(0.0);
    }

    pub fn clear_detail(&mut self) {
        self.selected_id = None;
        self.detail_name.clear();
        self.detail_path.clear();
        self.detail_items.clear();
        self.detail_tracks.clear();
        self.detail_entries.clear();
        self.rendered_detail_items.clear();
        self.detail_tracks_str.clear();
        self.last_detail_click = None;
        self.selected_detail_track = None;
        self.hovered_detail_row = None;
        self.renaming = false;
        self.last_detail_scroll.set(0.0);
    }

    pub fn clear_covers(&mut self) {
        self.playlist_covers.clear();
    }

    pub fn recompute_rendered_playlists(&mut self, playlists_fmt: &layout::PlaylistsFormatConfig) {
        self.rendered_playlists = self
            .playlists
            .iter()
            .map(|p| RenderedPlaylistSummary {
                tile_line1: format_template(&playlists_fmt.tile_line1, p),
                tile_line2: format_template(&playlists_fmt.tile_line2, p),
                list_line1: format_template(&playlists_fmt.list_line1, p),
                list_line2: format_template(&playlists_fmt.list_line2, p),
                list_details: format_template(&playlists_fmt.list_details, p),
            })
            .collect();
    }

    pub fn recompute_rendered_details(&mut self, playlists_fmt: &layout::PlaylistsFormatConfig) {
        self.detail_tracks_str = format!("{} tracks", self.detail_items.len());
        let track_map: HashMap<i64, &TrackRow> =
            self.detail_tracks.iter().map(|t| (t.id, t)).collect();
        self.rendered_detail_items = self
            .detail_items
            .iter()
            .map(|item| {
                let track_row = item.track_id.and_then(|tid| track_map.get(&tid).copied());
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
                RenderedPlaylistItem {
                    primary: format_template(&playlists_fmt.detail_primary, &lookup),
                    secondary: format_template(&playlists_fmt.detail_secondary, &lookup),
                    details: format_template(&playlists_fmt.detail_details, &lookup),
                }
            })
            .collect();
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

    #[allow(clippy::too_many_arguments)]
    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: impl Into<font::Font>,
        store: &'a QueueStore,
        dragging_track: Option<(u64, usize)>,
        dragging_detail: Option<(i64, usize)>,
        playlists_fmt: &'a layout::PlaylistsFormatConfig,
        queue_fmt: &'a layout::QueueFormatConfig,
    ) -> Element<'a, Message> {
        let font = font.into();
        let toolbar = self.view_toolbar(theme_cfg, font, store);
        let content: Element<'a, Message> = match self.section {
            PlaylistSection::Playlists => {
                if self.selected_id.is_some() {
                    self.view_detail(theme_cfg, font, dragging_detail, playlists_fmt)
                } else {
                    self.view_catalog(theme_cfg, font, playlists_fmt)
                }
            }
            PlaylistSection::Queues => {
                self.view_queues(theme_cfg, font, store, dragging_track, queue_fmt)
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

    fn view_toolbar<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        font: font::Font,
        store: &'a QueueStore,
    ) -> Element<'a, Message> {
        let section_chip = |label: &'static str, section: PlaylistSection, icon: &'static str| {
            component::chip_with_icon(
                theme_cfg,
                font,
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
                    bar = bar.push(component::chip_with_icon(
                        theme_cfg,
                        font,
                        icons::ARROW_LEFT_SVG,
                        "Back",
                        false,
                        Message::SelectPlaylist(-1),
                    ));
                } else {
                    bar = bar
                        .push(component::chip_with_icon(
                            theme_cfg,
                            font,
                            icons::FOLDER_SVG,
                            "Import",
                            false,
                            Message::ImportPlaylistsClicked,
                        ))
                        .push(component::chip_with_icon(
                            theme_cfg,
                            font,
                            icons::PLUS_SVG,
                            "New",
                            false,
                            Message::NewPlaylistClicked,
                        ));
                }
            }
            PlaylistSection::Queues => {
                if store.active.is_some() {
                    bar = bar.push(component::chip_with_icon(
                        theme_cfg,
                        font,
                        icons::TRASH_SVG,
                        "Remove all other queues",
                        false,
                        Message::QueueDeleteOthers,
                    ));
                }
                bar = bar.push(component::chip_with_icon(
                    theme_cfg,
                    font,
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
}

fn case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
    super::albums::cmp_case_insensitive_fast(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use component::TrackRow;
    use model::{PlaylistSortField, ViewMode};
    use playlist::{parse_m3u, write_m3u, PlaylistEntry, PlaylistItem};
    use std::path::PathBuf;

    fn item(title: &str, track_id: Option<i64>) -> PlaylistItem {
        PlaylistItem {
            title: title.to_string(),
            artist: "Artist".to_string(),
            duration_label: "3:00".to_string(),
            track_id,
        }
    }

    fn entry(raw: &str) -> PlaylistEntry {
        PlaylistEntry {
            raw_path: raw.to_string(),
            resolved_path: PathBuf::from(raw),
            title: None,
            duration_ms: Some(180_000),
        }
    }

    fn detail_view() -> PlaylistsView {
        let mut view = PlaylistsView::new(ViewMode::List, PlaylistSortField::Name);
        view.set_detail(
            7,
            "Mixtape".to_string(),
            "/playlists/mixtape.m3u8".to_string(),
            vec![
                item("One", Some(1)),
                item("Missing", None),
                item("Three", Some(3)),
            ],
            Vec::new(),
            vec![
                entry("/music/1.flac"),
                entry("/music/x.flac"),
                entry("/music/3.flac"),
            ],
        );
        view
    }

    #[test]
    fn move_detail_entry_permutes_items_and_entries_together() {
        let mut view = detail_view();
        assert!(view.move_detail_entry(0, 2));
        assert_eq!(
            view.detail_items
                .iter()
                .map(|i| i.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Missing", "Three", "One"]
        );
        assert_eq!(
            view.detail_entries
                .iter()
                .map(|e| e.raw_path.as_str())
                .collect::<Vec<_>>(),
            vec!["/music/x.flac", "/music/3.flac", "/music/1.flac"],
            "raw file entries (including unmatched lines) follow the same move"
        );
    }

    #[test]
    fn move_detail_entry_rejects_bad_moves() {
        let mut view = detail_view();
        assert!(!view.move_detail_entry(1, 1), "same slot is a no-op");
        assert!(!view.move_detail_entry(0, 3), "out of bounds is a no-op");
        assert!(!view.move_detail_entry(9, 0), "out of bounds is a no-op");
        assert_eq!(view.detail_items[0].title, "One");
    }

    #[test]
    fn detail_click_needs_same_row_twice_quickly() {
        let mut view = detail_view();
        assert!(!view.on_detail_click(7, 0), "first click only selects");
        assert!(
            view.on_detail_click(7, 0),
            "second click on the same row plays"
        );
        assert!(
            !view.on_detail_click(7, 1),
            "a different row starts a new arm"
        );
        assert!(
            !view.on_detail_click(9, 1),
            "a different playlist starts a new arm"
        );
        assert_eq!(view.selected_detail_track, Some((9, 1)));
    }

    #[test]
    fn reordered_entries_round_trip_through_m3u_file() {
        let temp_music =
            std::env::temp_dir().join(format!("melody-test-music-{}", std::process::id()));
        let t1 = temp_music.join("1.flac");
        let tx = temp_music.join("x.flac");
        let t3 = temp_music.join("3.flac");

        let mut view = PlaylistsView::new(ViewMode::List, PlaylistSortField::Name);
        view.set_detail(
            7,
            "Mixtape".to_string(),
            "/playlists/mixtape.m3u8".to_string(),
            vec![
                item("One", Some(1)),
                item("Missing", None),
                item("Three", Some(3)),
            ],
            Vec::new(),
            vec![
                entry(&t1.to_string_lossy()),
                entry(&tx.to_string_lossy()),
                entry(&t3.to_string_lossy()),
            ],
        );
        assert!(view.move_detail_entry(2, 0));

        let path = std::env::temp_dir().join(format!(
            "melody-playlist-reorder-{}.m3u8",
            std::process::id()
        ));
        write_m3u(&path, &view.detail_entries).expect("write reordered playlist");
        let content = std::fs::read_to_string(&path).expect("read reordered playlist");
        assert!(content.contains(&t3.to_string_lossy().to_string()));
        assert!(content.contains(&t1.to_string_lossy().to_string()));
        assert!(content.contains(&tx.to_string_lossy().to_string()));

        let parsed = parse_m3u(&path).expect("parse reordered playlist");
        let _ = std::fs::remove_file(&path);
        // write_m3u writes absolute paths, so verify both order and absolute paths survive.
        assert_eq!(
            parsed
                .entries
                .iter()
                .map(|e| e.resolved_path.clone())
                .collect::<Vec<_>>(),
            vec![t3, t1, tx]
        );
    }

    #[test]
    fn shared_track_row_renders_without_panicking() {
        let theme_cfg = ThemeConfig::default();

        // Open-playlist context: clickable row.
        let _ = TrackRow {
            theme: &theme_cfg,
            font: font::Font::DEFAULT,
            index: 0,
            title: "One".into(),
            artist: "Artist".into(),
            duration: "3:00".into(),
            highlighted: true,
            dragging: false,
            show_handle: true,
            on_press: Some(Message::PlaylistDetailClicked {
                playlist_id: 7,
                index: 0,
            }),
            drag_start: Message::PlaylistDetailDragStarted {
                playlist_id: 7,
                index: 0,
            },
            drag_release: Message::PlaylistDetailDragReleased,
            hover_enter: Message::PlaylistDetailHovered(Some((7, 0))),
            hover_exit: Message::PlaylistDetailHovered(None),
            on_remove: Message::PlaylistRemoveTrack {
                playlist_id: 7,
                index: 0,
            },
            on_right_press: None,
        }
        .view();

        // Open-playlist context: unmatched (unclickable) entry.
        let _ = TrackRow {
            theme: &theme_cfg,
            font: font::Font::DEFAULT,
            index: 1,
            title: "Missing".into(),
            artist: "Artist".into(),
            duration: "3:00".into(),
            highlighted: false,
            dragging: false,
            show_handle: false,
            on_press: None,
            drag_start: Message::PlaylistDetailDragStarted {
                playlist_id: 7,
                index: 1,
            },
            drag_release: Message::PlaylistDetailDragReleased,
            hover_enter: Message::PlaylistDetailHovered(Some((7, 1))),
            hover_exit: Message::PlaylistDetailHovered(None),
            on_remove: Message::PlaylistRemoveTrack {
                playlist_id: 7,
                index: 1,
            },
            on_right_press: None,
        }
        .view();

        // Queue context.
        let _ = TrackRow {
            theme: &theme_cfg,
            font: font::Font::DEFAULT,
            index: 2,
            title: "Three".into(),
            artist: "Artist".into(),
            duration: "4:12".into(),
            highlighted: false,
            dragging: false,
            show_handle: false,
            on_press: Some(Message::QueueClicked {
                queue_id: 1,
                index: 2,
            }),
            drag_start: Message::QueueDragStarted {
                queue_id: 1,
                index: 2,
            },
            drag_release: Message::QueueDragReleased,
            hover_enter: Message::QueueRowHovered(Some((1, 2))),
            hover_exit: Message::QueueRowHovered(None),
            on_remove: Message::QueueRemoveTrack {
                queue_id: 1,
                index: 2,
            },
            on_right_press: None,
        }
        .view();
    }

    #[test]
    fn detail_state_resets_with_detail() {
        let mut view = detail_view();
        view.on_detail_click(7, 0);
        view.hovered_detail_row = Some((7, 0));
        view.last_detail_scroll.set(120.0);
        view.clear_detail();
        assert!(view.detail_entries.is_empty());
        assert!(view.selected_detail_track.is_none());
        assert!(view.hovered_detail_row.is_none());
        assert!(view.last_detail_click.is_none());
        assert_eq!(view.last_detail_scroll.get(), 0.0);
        assert_eq!(view.section, PlaylistSection::Playlists);
    }

    #[test]
    fn test_playlists_scroll_state_tracks() {
        let mut view = PlaylistsView::new(ViewMode::List, PlaylistSortField::Name);
        assert_eq!(view.last_playlists_scroll.get(), 0.0);
        assert_eq!(view.last_detail_scroll.get(), 0.0);
        assert_eq!(view.last_queue_scroll.get(), 0.0);

        view.last_playlists_scroll.set(300.0);
        view.last_detail_scroll.set(150.0);
        view.on_queue_scroll_offset(450.0, 100);
        assert_eq!(view.last_queue_scroll.get(), 450.0);

        view.clear_detail();
        assert_eq!(view.last_detail_scroll.get(), 0.0);
        // Catalog scroll is still preserved after closing detail
        assert_eq!(view.last_playlists_scroll.get(), 300.0);

        view.reset_queue_window();
        assert_eq!(view.last_queue_scroll.get(), 0.0);
    }

    #[test]
    fn remove_detail_entry_keeps_items_and_entries_in_sync() {
        let mut view = detail_view();
        assert!(view.remove_detail_entry(7, 1));
        assert_eq!(
            view.detail_items
                .iter()
                .map(|i| i.title.as_str())
                .collect::<Vec<_>>(),
            vec!["One", "Three"]
        );
        assert_eq!(
            view.detail_entries
                .iter()
                .map(|e| e.raw_path.as_str())
                .collect::<Vec<_>>(),
            vec!["/music/1.flac", "/music/3.flac"],
            "raw file entries (including unmatched lines) follow the same removal"
        );
    }

    #[test]
    fn remove_detail_entry_rejects_bad_removals() {
        let mut view = detail_view();
        assert!(!view.remove_detail_entry(7, 3), "out of bounds is a no-op");
        assert!(!view.remove_detail_entry(9, 0), "wrong playlist is a no-op");
        assert_eq!(view.detail_items.len(), 3);
        assert_eq!(view.detail_entries.len(), 3);
    }

    #[test]
    fn remove_detail_entry_shifts_selection_state() {
        let mut view = detail_view();
        view.selected_detail_track = Some((7, 2));
        view.hovered_detail_row = Some((7, 2));
        assert!(view.remove_detail_entry(7, 0));
        assert_eq!(view.selected_detail_track, Some((7, 1)));
        assert_eq!(view.hovered_detail_row, Some((7, 1)));

        assert!(view.remove_detail_entry(7, 1));
        assert!(view.selected_detail_track.is_none());
        assert!(view.hovered_detail_row.is_none());
    }
}
