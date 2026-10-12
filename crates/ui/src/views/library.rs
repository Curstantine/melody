use core_engine::{SortDirection, SortField, Track};
use iced::widget::{button, column, container, mouse_area, row, scrollable, text_input, Space};
use iced::{Alignment, Element, Length};

use crate::components::{self, text};
use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

/// Self-contained state and view component for the music tracks library screen.
pub struct LibraryView {
    pub visible_tracks: Vec<Track>,
    pub total_tracks: usize,
    pub page: usize,
    pub page_size: usize,
    pub is_loading_more: bool,
    pub search_query: String,
    pub sort_field: SortField,
    pub sort_dir: SortDirection,
    pub selected_track_id: Option<i64>,
    pub last_track_click: Option<(i64, std::time::Instant)>,
}

impl LibraryView {
    pub fn new(page_size: usize) -> Self {
        Self {
            visible_tracks: Vec::new(),
            total_tracks: 0,
            page: 0,
            page_size,
            is_loading_more: false,
            search_query: String::new(),
            sort_field: SortField::Title,
            sort_dir: SortDirection::Ascending,
            selected_track_id: None,
            last_track_click: None,
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>, total: usize) {
        self.visible_tracks = tracks;
        self.total_tracks = total;
        self.is_loading_more = false;
    }

    pub fn append_tracks(&mut self, tracks: Vec<Track>) {
        self.is_loading_more = false;
        if !tracks.is_empty() {
            self.visible_tracks.extend(tracks);
        }
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.page = 0;
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
        self.page = 0;
    }

    pub fn set_page_size(&mut self, size: usize) {
        self.page_size = size;
        self.page = 0;
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

    pub fn on_scroll(&mut self, viewport: &iced::widget::scrollable::Viewport) -> bool {
        let rel_offset = viewport.relative_offset();
        if rel_offset.y >= 0.8
            && !self.is_loading_more
            && self.visible_tracks.len() < self.total_tracks
        {
            self.is_loading_more = true;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.visible_tracks.clear();
        self.total_tracks = 0;
        self.selected_track_id = None;
        self.last_track_click = None;
    }

    pub fn find_track(&self, id: i64) -> Option<&Track> {
        self.visible_tracks.iter().find(|t| t.id == id)
    }

    pub fn get_next_track(
        &self,
        current_id: Option<i64>,
        is_repeat: bool,
        is_shuffle: bool,
        position_seed: u64,
        current_track: Option<&Track>,
    ) -> Option<Track> {
        if is_repeat {
            return current_track.cloned();
        }

        if is_shuffle && !self.visible_tracks.is_empty() {
            let rand_idx = (position_seed as usize ^ 0x9e3779b9) % self.visible_tracks.len();
            return Some(self.visible_tracks[rand_idx].clone());
        }

        if let Some(cur_id) = current_id {
            if let Some(pos) = self.visible_tracks.iter().position(|t| t.id == cur_id) {
                if pos + 1 < self.visible_tracks.len() {
                    return Some(self.visible_tracks[pos + 1].clone());
                }
            }
        }

        if let Some(first) = self.visible_tracks.first() {
            if Some(first.id) != current_id {
                return Some(first.clone());
            }
        }

        None
    }

    pub fn get_previous_track(&self, current_id: i64) -> Option<Track> {
        if let Some(pos) = self.visible_tracks.iter().position(|t| t.id == current_id) {
            if pos > 0 {
                return Some(self.visible_tracks[pos - 1].clone());
            }
        }
        None
    }

    pub fn view<'a>(
        &'a self,
        theme_cfg: &'a ThemeConfig,
        current_track_id: Option<i64>,
        is_playing: bool,
    ) -> Element<'a, Message> {
        // 1. Search Bar & View Mode Toolbar
        let search_input = text_input("Search tracks, artists, albums...", &self.search_query)
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
                    Space::with_width(4),
                    icons::icon(sort_icon_svg, 12.0, theme_cfg.primary_color()),
                ]
                .align_y(Alignment::Center)
                .into()
            } else {
                text(theme_cfg, label).size(12).into()
            };

            button(content)
                .style(styles::column_header_button(theme_cfg, is_active))
                .padding([2, 0])
                .width(width)
                .on_press(Message::SortChanged(field))
        };

        let table_header = row![
            container(
                text(theme_cfg, "#")
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
            )
            .width(36)
            .center_x(36),
            header_btn("Title", SortField::Title, Length::FillPortion(4)),
            header_btn("Artist", SortField::Artist, Length::FillPortion(3)),
            header_btn("Album", SortField::Album, Length::FillPortion(3)),
            header_btn("Duration", SortField::Duration, Length::Fixed(80.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 2.0,
            right: 16.0,
            bottom: 2.0,
            left: 0.0,
        });

        // 3. Track Content (List view)
        let content: Element<'a, Message> = if self.visible_tracks.is_empty() {
            components::empty_state(
                theme_cfg,
                "No tracks found",
                "Scan a folder in Preferences to index your music library",
            )
        } else {
            let mut track_items = column![].spacing(2);

            for (idx, track) in self.visible_tracks.iter().enumerate() {
                let is_active = current_track_id == Some(track.id);
                let is_selected = self.selected_track_id == Some(track.id);
                let display_index = (self.page * self.page_size) + idx + 1;

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
                    .into()
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
                    .into()
                };

                let track_row = row![
                    left_action,
                    text(theme_cfg, &track.title)
                        .size(13)
                        .color(if is_active {
                            theme_cfg.primary_color()
                        } else {
                            theme_cfg.text_color()
                        })
                        .width(Length::FillPortion(4)),
                    text(theme_cfg, &track.artist)
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                        .width(Length::FillPortion(3)),
                    text(theme_cfg, &track.album)
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                        .width(Length::FillPortion(3)),
                    text(theme_cfg, track.formatted_duration())
                        .size(12)
                        .color(theme_cfg.text_muted_color())
                        .width(80),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding(iced::Padding {
                    top: 2.0,
                    right: 15.0,
                    bottom: 2.0,
                    left: 0.0,
                });

                let row_container = container(track_row)
                    .style(styles::table_row_container(
                        theme_cfg,
                        is_active || is_selected,
                    ))
                    .width(Length::Fill);

                let interactive_row =
                    mouse_area(row_container).on_press(Message::TrackClicked(track.id));

                track_items = track_items.push(interactive_row);
            }

            scrollable(track_items)
                .on_scroll(Message::LibraryScrolled)
                .height(Length::Fill)
                .width(Length::Fill)
                .style(styles::scrollable_style(theme_cfg))
                .into()
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
            .into()
    }
}
