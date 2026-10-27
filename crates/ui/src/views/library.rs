use std::collections::VecDeque;

use core_engine::{format_template, SortDirection, SortField, TrackFilter, TrackRow};
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text_input, Space, Widget,
};
use iced::{Alignment, Element, Length};

use crate::components::{self, text, TrackMenuContext};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

/// Fixed pixel height of a single track row. The virtualization math depends on
/// every row rendering at exactly this height.
pub const ROW_HEIGHT: f32 = 28.0;

/// Number of extra pages kept above and below the viewport. The window is
/// reloaded only when the visible range approaches its edges, which prevents
/// thrashing while keeping resident rows bounded.
pub const WINDOW_BUFFER_PAGES: usize = 2;

/// Self-contained state and view component for the music tracks library screen.
///
/// Only a bounded window of [`TrackRow`]s is kept in memory. The scrollable
/// renders two spacers sized from `window_start`/`total_tracks` so the scrollbar
/// reflects the whole library while only the visible window is materialized.
pub struct LibraryView {
    pub rows: VecDeque<TrackRow>,
    pub total_tracks: usize,
    pub window_start: usize,
    pub page_size: usize,
    pub row_height: f32,
    pub search_query: String,
    /// Parsed form of `search_query`, used for the database query and to derive
    /// the queue scope when playback starts.
    pub filter: TrackFilter,
    pub sort_field: SortField,
    pub sort_dir: SortDirection,
    pub selected_track_id: Option<i64>,
    pub last_track_click: Option<(i64, std::time::Instant)>,
    /// Row offset of an in-flight window request, used to avoid duplicate loads.
    pub pending_window: Option<usize>,
    pub last_visible_offset: usize,
}

impl LibraryView {
    pub fn new(page_size: usize) -> Self {
        Self {
            rows: VecDeque::new(),
            total_tracks: 0,
            window_start: 0,
            page_size,
            row_height: ROW_HEIGHT,
            search_query: String::new(),
            filter: TrackFilter::default(),
            sort_field: SortField::Title,
            sort_dir: SortDirection::Ascending,
            selected_track_id: None,
            last_track_click: None,
            pending_window: None,
            last_visible_offset: 0,
        }
    }

    /// Maximum number of rows retained in the sliding window.
    pub fn window_capacity(&self) -> usize {
        self.page_size * (1 + 2 * WINDOW_BUFFER_PAGES)
    }

    /// Replaces the whole library view, resetting to the top (offset 0).
    pub fn set_window(&mut self, rows: Vec<TrackRow>, total: usize) {
        self.rows = rows.into();
        self.total_tracks = total;
        self.window_start = 0;
        self.pending_window = None;
    }

    /// Swaps in a windowed slice loaded for `window_start`, preserving total.
    pub fn replace_window(&mut self, window_start: usize, rows: Vec<TrackRow>) {
        self.rows = rows.into();
        self.window_start = window_start;
        self.pending_window = None;
    }

    pub fn set_search_query(&mut self, query: String) {
        self.filter = TrackFilter::parse(&query);
        self.search_query = query;
    }

    /// Applies a structured filter and mirrors it into the search box text.
    pub fn set_filter(&mut self, filter: TrackFilter) {
        self.search_query = filter.to_query_string();
        self.filter = filter;
    }

    /// The parsed filter, or `None` when it matches the whole library.
    pub fn active_filter(&self) -> Option<TrackFilter> {
        if self.filter.is_empty() {
            None
        } else {
            Some(self.filter.clone())
        }
    }

    pub fn toggle_sort(&mut self, field: SortField) {
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

    pub fn set_page_size(&mut self, size: usize) {
        self.page_size = size;
    }

    /// Handles a track click event. Returns `true` if it's a double-click (meaning playback should start).
    pub fn on_track_click(&mut self, id: i64) -> bool {
        let now = std::time::Instant::now();
        let is_double_click = if let Some((last_id, last_time)) = self.last_track_click {
            last_id == id && now.duration_since(last_time).as_millis() <= 400
        } else {
            false
        };

        if is_double_click {
            self.last_track_click = None;
            self.selected_track_id = Some(id);
            true
        } else {
            self.last_track_click = Some((id, now));
            self.selected_track_id = Some(id);
            false
        }
    }

    /// Pure virtualization logic operating on an absolute scroll offset in
    /// pixels. Returns the row offset to load when the visible range leaves the
    /// buffered window, or `None` if the current window still covers it.
    pub fn on_scroll_offset(&mut self, scroll_y: f32) -> Option<usize> {
        if self.total_tracks == 0 || self.row_height <= 0.0 {
            return None;
        }

        let first_visible = (scroll_y.max(0.0) / self.row_height).floor() as usize;
        self.last_visible_offset = first_visible;

        let window_end = self.window_start + self.rows.len();
        let visible_rows = self.page_size.max(1);
        let near_top = first_visible < self.window_start;
        let near_bottom = first_visible + visible_rows > window_end;
        if !near_top && !near_bottom {
            return None;
        }

        let capacity = self.window_capacity();
        let max_start = self.total_tracks.saturating_sub(capacity);
        let buffer = self.page_size * WINDOW_BUFFER_PAGES;
        let desired = first_visible.saturating_sub(buffer).min(max_start);

        if desired == self.window_start && !self.rows.is_empty() {
            return None;
        }
        if self.pending_window == Some(desired) {
            return None;
        }

        self.pending_window = Some(desired);
        Some(desired)
    }

    /// Convenience wrapper over [`Self::on_scroll_offset`] for iced viewports.
    pub fn on_scroll(&mut self, viewport: &iced::widget::scrollable::Viewport) -> Option<usize> {
        self.on_scroll_offset(viewport.absolute_offset().y)
    }

    pub fn clear(&mut self) {
        self.rows = VecDeque::new();
        self.total_tracks = 0;
        self.window_start = 0;
        self.pending_window = None;
        self.selected_track_id = None;
        self.last_track_click = None;
    }

    #[allow(dead_code)]
    pub fn find_row(&self, id: i64) -> Option<&TrackRow> {
        self.rows.iter().find(|t| t.id == id)
    }

    pub fn get_next_row(
        &self,
        current_id: Option<i64>,
        is_shuffle: bool,
        position_seed: u64,
    ) -> Option<&TrackRow> {
        if is_shuffle && !self.rows.is_empty() {
            let rand_idx = (position_seed as usize ^ 0x9e3779b9) % self.rows.len();
            return self.rows.get(rand_idx);
        }

        if let Some(cur_id) = current_id {
            if let Some(pos) = self.rows.iter().position(|t| t.id == cur_id) {
                if pos + 1 < self.rows.len() {
                    return self.rows.get(pos + 1);
                }
            }
        }

        self.rows.front()
    }

    pub fn get_previous_row(&self, current_id: i64) -> Option<&TrackRow> {
        if let Some(pos) = self.rows.iter().position(|t| t.id == current_id) {
            if pos > 0 {
                return self.rows.get(pos - 1);
            }
        }
        None
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        current_track_id: Option<i64>,
        is_playing: bool,
        format_cfg: &'a crate::layout::LibraryFormatConfig,
    ) -> Element<'a, Message> {
        // 1. Search Bar & View Mode Toolbar
        let search_input = text_input(
            "Search tracks... (album:\"…\" artist:… genre:… year:…)",
            &self.search_query,
        )
        .font(theme_cfg.font())
        .size(12)
        .on_input(Message::SearchInputChanged)
        .padding([4, 8])
        .style(styles::search_input_style(theme_cfg))
        .width(Length::Fill);

        let toolbar = row![
            container(icons::icon(
                icons::SEARCH_SVG,
                16.0,
                theme_cfg.text_muted_color(),
            ))
            .width(24)
            .center_x(24),
            search_input,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 0.0,
            left: 8.0,
        });

        // 2. Table Header (for List view)
        let header_btn = |label: &'a str, field: SortField, width: Length| {
            let is_active = self.sort_field == field;

            let content: Element<'a, Message> = if is_active {
                let sort_icon_svg = match self.sort_dir {
                    SortDirection::Ascending => icons::CHEVRON_UP_SVG,
                    SortDirection::Descending => icons::CHEVRON_DOWN_SVG,
                };
                row![
                    text(theme_cfg, label).size(12),
                    Space::new().width(4),
                    icons::icon(sort_icon_svg, 12.0, theme_cfg.primary_color()),
                ]
                .align_y(Alignment::Center)
                .boxed()
            } else {
                text(theme_cfg, label).size(12).boxed()
            };

            button(content)
                .style(styles::column_header_button(theme_cfg, is_active))
                .padding([2, 0])
                .width(width)
                .on_press(Message::SortChanged(field))
        };

        let mut table_header = row![container(
            text(theme_cfg, "#")
                .size(12)
                .color(theme_cfg.text_muted_color()),
        )
        .width(36)
        .center_x(36),]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 2.0,
            right: 16.0,
            bottom: 2.0,
            left: 0.0,
        });

        for col in &format_cfg.columns {
            let width = col.width.to_iced_length();
            let btn: Element<'a, Message> = if let Some(field) = col.sort_field {
                header_btn(&col.header, field, width).boxed()
            } else {
                container(
                    text(theme_cfg, &col.header)
                        .size(12)
                        .color(theme_cfg.text_muted_color()),
                )
                .width(width)
                .padding([2, 0])
                .boxed()
            };
            table_header = table_header.push(btn);
        }

        // 3. Track Content (virtualized list)
        let content: Element<'a, Message> = if self.total_tracks == 0 {
            components::empty_state(
                theme_cfg,
                "No tracks found",
                "Scan a folder in Preferences to index your music library",
            )
        } else {
            let mut items = column![].spacing(0).width(Length::Fill);

            let top_spacer = self.window_start as f32 * self.row_height;
            if top_spacer > 0.0 {
                items = items.push(Space::new().height(top_spacer).boxed());
            }

            for (idx, track) in self.rows.iter().enumerate() {
                let is_active = current_track_id == Some(track.id);
                let is_selected = self.selected_track_id == Some(track.id);
                let display_index = self.window_start + idx + 1;

                // Left-most action: # index, or playing speaker indicator if active
                let left_action: Element<'a, Message> = if is_active {
                    button(
                        container(icons::icon(
                            if is_playing {
                                icons::PAUSE_SVG
                            } else {
                                icons::PLAY_SVG
                            },
                            12.0,
                            theme_cfg.primary_color(),
                        ))
                        .width(36)
                        .height(24)
                        .center_x(36)
                        .center_y(24),
                    )
                    .style(styles::icon_control_button(theme_cfg))
                    .padding(0)
                    .width(36)
                    .height(24)
                    .on_press(Message::TogglePlayPause)
                    .boxed()
                } else {
                    container(
                        text(theme_cfg, format!("{}", display_index))
                            .size(12)
                            .color(theme_cfg.text_muted_color()),
                    )
                    .width(36)
                    .height(24)
                    .center_x(36)
                    .center_y(24)
                    .boxed()
                };

                let mut track_row = row![left_action]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .height(self.row_height)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 15.0,
                        bottom: 0.0,
                        left: 0.0,
                    });

                for (col_idx, col) in format_cfg.columns.iter().enumerate() {
                    let cell_value = format_template(&col.format, track);
                    let is_primary_col = col_idx == 0;
                    let col_text = text(theme_cfg, cell_value)
                        .size(if is_primary_col { 13 } else { 12 })
                        .color(if is_primary_col && is_active {
                            theme_cfg.primary_color()
                        } else if is_primary_col {
                            theme_cfg.text_color()
                        } else {
                            theme_cfg.text_muted_color()
                        })
                        .width(col.width.to_iced_length())
                        .wrapping(iced::widget::text::Wrapping::None)
                        .ellipsis(iced::widget::text::Ellipsis::End);

                    track_row = track_row.push(col_text.boxed());
                }

                let row_container = container(track_row)
                    .style(styles::table_row_container(
                        theme_cfg,
                        is_active || is_selected,
                    ))
                    .width(Length::Fill)
                    .height(self.row_height);

                let interactive_row = mouse_area(row_container)
                    .on_press(Message::TrackClicked(track.id))
                    .on_right_press(Message::OpenTrackContextMenu {
                        track_id: track.id,
                        context: TrackMenuContext::Library,
                    });

                items = items.push(interactive_row.boxed());
            }

            let window_end = self.window_start + self.rows.len();
            let bottom_spacer =
                self.total_tracks.saturating_sub(window_end) as f32 * self.row_height;
            if bottom_spacer > 0.0 {
                items = items.push(Space::new().height(bottom_spacer).boxed());
            }

            scrollable(items)
                .id(iced::widget::Id::new("library_tracks"))
                .on_scroll(|scroll| Message::LibraryScrolled(scroll.viewport))
                .height(Length::Fill)
                .width(Length::Fill)
                .style(styles::scrollable_style(theme_cfg))
                .boxed()
        };

        column![toolbar, table_header, content]
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LibraryFormatConfig;
    use crate::theme::ThemeConfig;
    use core_engine::TrackRow;

    /// Simulated catalog size for the virtualization tests.
    const LARGE_LIBRARY_TRACK_COUNT: usize = 100_000;

    fn make_rows(start: usize, count: usize) -> Vec<TrackRow> {
        (start..start + count)
            .map(|i| TrackRow {
                id: i as i64 + 1,
                title: format!("Track {:06}", i),
                artist: format!("Artist {:04}", i % 5_000).into(),
                album: format!("Album {:05}", i % 12_500).into(),
                duration_ms: 180_000,
                file_path: format!("/music/track_{:06}.flac", i),
                duration_label: "3:00".to_string(),
                year: None,
                track_num: None,
                disc_num: None,
                album_artist: None,
                genre: None,
            })
            .collect()
    }

    #[test]
    fn window_capacity_is_bounded_by_page_size() {
        let view = LibraryView::new(50);
        assert_eq!(view.window_capacity(), 50 * (1 + 2 * WINDOW_BUFFER_PAGES));
    }

    #[test]
    fn scrolling_a_large_library_keeps_memory_bounded() {
        let total = LARGE_LIBRARY_TRACK_COUNT;

        let mut view = LibraryView::new(50);
        let capacity = view.window_capacity();

        // Initial load at offset 0.
        let initial = make_rows(0, capacity);
        view.set_window(initial, total);
        assert_eq!(view.total_tracks, total);
        assert!(view.rows.len() <= capacity);

        // Simulate a user scrolling all the way to the bottom. Every time the view
        // requests a window reload we emulate the DB returning exactly one window.
        let mut y = 0.0_f32;
        let bottom = total as f32 * ROW_HEIGHT;
        let mut loads = 0usize;
        let mut peak_loaded = view.rows.len();

        while y < bottom {
            if let Some(offset) = view.on_scroll_offset(y) {
                let count = capacity.min(total - offset);
                view.replace_window(offset, make_rows(offset, count));
                loads += 1;
                peak_loaded = peak_loaded.max(view.rows.len());
                assert!(
                    view.rows.len() <= capacity,
                    "loaded {} rows, capacity is {capacity}",
                    view.rows.len()
                );
            }
            y += ROW_HEIGHT * 5.0;
        }

        assert!(
            loads > 1,
            "expected multiple window reloads while scrolling"
        );
        assert!(
            peak_loaded <= capacity,
            "resident rows grew beyond the window capacity"
        );
    }

    #[test]
    fn no_reload_within_buffer_region() {
        let mut view = LibraryView::new(50);
        let capacity = view.window_capacity();
        view.set_window(make_rows(0, capacity), 100_000);

        // The viewport sits comfortably inside the buffered window.
        assert_eq!(view.on_scroll_offset(0.0), None);
        assert_eq!(view.on_scroll_offset(ROW_HEIGHT * 40.0), None);
    }

    #[test]
    fn empty_library_never_requests_a_window() {
        let mut view = LibraryView::new(50);
        assert_eq!(view.on_scroll_offset(0.0), None);
        assert_eq!(view.on_scroll_offset(10_000.0), None);
    }

    #[test]
    fn navigation_and_lookup_use_the_window() {
        let mut view = LibraryView::new(50);
        view.set_window(make_rows(0, 120), 100_000);

        assert_eq!(view.find_row(1).map(|r| r.id), Some(1));
        assert_eq!(view.get_next_row(Some(1), false, 0).map(|r| r.id), Some(2));
        assert_eq!(view.get_previous_row(2).map(|r| r.id), Some(1));
        assert_eq!(view.get_previous_row(1).map(|r| r.id), None);
    }

    #[test]
    fn renders_virtualized_window_with_spacers() {
        let theme_cfg = ThemeConfig::default();
        let mut view = LibraryView::new(50);
        view.set_window(make_rows(0, view.window_capacity()), 100_000);
        // Shifting the window deep into the library must still render.
        view.replace_window(50_000, make_rows(50_000, view.window_capacity()));

        let format_cfg = LibraryFormatConfig::default();
        let _element = view.view(&theme_cfg, None, false, &format_cfg);
    }
}
