use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use core_engine::{format_duration, AlbumSummary, SortDirection};
use iced::border::Radius;
use iced::widget::{button, column, container, image, row, scrollable, Space};
use iced::{Alignment, Element, Length, Padding};

use crate::components::{self, text};
use crate::icons;
use crate::message::{AlbumSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub const DEFAULT_CATALOG_CHUNK_SIZE: usize = 60;
#[allow(dead_code)]
pub const DEFAULT_ALBUM_TILE_WIDTH: u32 = 140;
/// Minimum number of decoded album cover textures retained in RAM/VRAM.
/// The effective cap is `max(MAX_ALBUM_COVERS, visible_count)` so that covers
/// currently on screen are never evicted (which would cause reload thrashing).
pub const MAX_ALBUM_COVERS: usize = 120;

/// Cache key used for album covers.
fn cover_key(album: &AlbumSummary) -> String {
    format!("{}:{}", album.artist, album.album)
}

/// Self-contained state and view component for the Albums catalog screen.
pub struct AlbumsView {
    pub view_mode: ViewMode,
    pub tile_width: u32,
    pub albums: Vec<AlbumSummary>,
    pub album_covers: HashMap<String, image::Handle>,
    /// Recency order of cached covers: front = least recently used, back = most recently used.
    pub cover_order: VecDeque<String>,
    pub sort_field: AlbumSortField,
    pub sort_dir: SortDirection,
    pub visible_count: usize,
    pub in_flight: HashSet<String>,
    pub failed: HashSet<String>,
}

impl AlbumsView {
    #[allow(dead_code)]
    pub fn new(view_mode: ViewMode, sort_field: AlbumSortField) -> Self {
        Self::with_tile_width(view_mode, sort_field, DEFAULT_ALBUM_TILE_WIDTH)
    }

    pub fn with_tile_width(
        view_mode: ViewMode,
        sort_field: AlbumSortField,
        tile_width: u32,
    ) -> Self {
        Self {
            view_mode,
            tile_width,
            albums: Vec::new(),
            album_covers: HashMap::new(),
            cover_order: VecDeque::new(),
            sort_field,
            sort_dir: SortDirection::Ascending,
            visible_count: DEFAULT_CATALOG_CHUNK_SIZE,
            in_flight: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    pub fn set_tile_width(&mut self, width: u32) {
        self.tile_width = width;
    }

    pub fn sort_albums(albums: &mut [AlbumSummary], field: AlbumSortField, dir: SortDirection) {
        fn cmp_case_insensitive(a: &str, b: &str) -> std::cmp::Ordering {
            let a_chars = a.chars().flat_map(|c| c.to_lowercase());
            let b_chars = b.chars().flat_map(|c| c.to_lowercase());
            a_chars.cmp(b_chars)
        }

        albums.sort_by(|a, b| {
            let ord = match field {
                AlbumSortField::Title => cmp_case_insensitive(&a.album, &b.album),
                AlbumSortField::Artist => cmp_case_insensitive(&a.artist, &b.artist)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::Year => a
                    .year
                    .cmp(&b.year)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::Duration => a
                    .total_duration_ms
                    .cmp(&b.total_duration_ms)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
                AlbumSortField::DateAdded => a
                    .added_timestamp
                    .cmp(&b.added_timestamp)
                    .then_with(|| cmp_case_insensitive(&a.album, &b.album)),
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    pub fn sort(&mut self) {
        Self::sort_albums(&mut self.albums, self.sort_field, self.sort_dir);
    }

    pub fn set_albums(&mut self, mut albums: Vec<AlbumSummary>) {
        Self::sort_albums(&mut albums, self.sort_field, self.sort_dir);
        self.albums = albums;
        self.failed.clear();
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn toggle_sort(&mut self, field: AlbumSortField) {
        if self.sort_field == field {
            self.sort_dir = match self.sort_dir {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_field = field;
            self.sort_dir = match field {
                AlbumSortField::Year
                | AlbumSortField::TrackCount
                | AlbumSortField::Duration
                | AlbumSortField::DateAdded => SortDirection::Descending,
                _ => SortDirection::Ascending,
            };
        }
        self.sort();
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn on_scroll(&mut self, viewport: &iced::widget::scrollable::Viewport) -> bool {
        let rel_offset = viewport.relative_offset();
        if rel_offset.y >= 0.75 && self.visible_count < self.albums.len() {
            self.visible_count =
                (self.visible_count + DEFAULT_CATALOG_CHUNK_SIZE).min(self.albums.len());
            true
        } else {
            false
        }
    }

    pub fn get_missing_covers_to_load(&self) -> Vec<(String, PathBuf)> {
        self.visible_albums()
            .iter()
            .filter_map(|alb| {
                let key = cover_key(alb);
                if self.album_covers.contains_key(&key)
                    || self.in_flight.contains(&key)
                    || self.failed.contains(&key)
                {
                    return None;
                }
                alb.cover_track_path
                    .as_ref()
                    .map(|p| (key, PathBuf::from(p)))
            })
            .collect()
    }

    pub fn mark_loading(&mut self, keys: impl IntoIterator<Item = String>) {
        self.in_flight.extend(keys);
    }

    pub fn is_in_flight(&self, key: &str) -> bool {
        self.in_flight.contains(key)
    }

    /// Marks every currently visible, cached cover as most recently used.
    ///
    /// Rendering only has `&self`, so recency can't be bumped during `view`.
    /// Call this whenever the visible set changes (e.g. scroll, sort change, view-mode change).
    pub fn touch_visible(&mut self) {
        let visible: Vec<String> = self
            .visible_albums()
            .iter()
            .map(cover_key)
            .filter(|k| self.album_covers.contains_key(k))
            .collect();

        if visible.is_empty() {
            return;
        }

        let visible_set: HashSet<&str> = visible.iter().map(String::as_str).collect();
        self.cover_order
            .retain(|k| !visible_set.contains(k.as_str()));

        self.cover_order.extend(visible);
    }

    pub fn insert_covers(&mut self, loaded: Vec<(String, Vec<u8>)>) {
        for (key, bytes) in loaded {
            if !self.in_flight.remove(&key) {
                continue;
            }

            if bytes.is_empty() {
                self.failed.insert(key);
                continue;
            }

            let handle = image::Handle::from_bytes(bytes);
            if let Some(pos) = self.cover_order.iter().position(|k| k == &key) {
                self.cover_order.remove(pos);
            }

            self.cover_order.push_back(key.clone());
            self.album_covers.insert(key, handle);
        }

        self.touch_visible();

        let cap = MAX_ALBUM_COVERS.max(self.visible_count);
        if self.cover_order.len() > cap {
            let excess = self.cover_order.len() - cap;
            for key in self.cover_order.drain(0..excess) {
                self.album_covers.remove(&key);
            }
        }
    }

    pub fn clear(&mut self) {
        // Replace with empty collections to drop the backing allocations entirely.
        // `.clear()` would retain the heap capacity; we want the memory gone.
        self.albums = Vec::new();
        self.failed = HashSet::new();
        self.clear_covers();
    }

    pub fn clear_covers(&mut self) {
        // Same reasoning: replace rather than clear so capacity is freed.
        self.album_covers = HashMap::new();
        self.cover_order = VecDeque::new();
        self.in_flight = HashSet::new();
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    pub fn visible_albums(&self) -> &[AlbumSummary] {
        &self.albums[..self.visible_count.min(self.albums.len())]
    }

    pub fn len(&self) -> usize {
        self.albums.len()
    }

    pub fn is_empty(&self) -> bool {
        self.albums.is_empty()
    }

    pub fn view<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let albums = self.visible_albums();
        if albums.is_empty() {
            return components::empty_state(
                theme_cfg,
                "No albums indexed",
                "Scan a folder to populate the album catalog",
            );
        }

        let header_bar = row![
            components::sort_chip_list(
                theme_cfg,
                &[
                    ("Title", AlbumSortField::Title),
                    ("Artist", AlbumSortField::Artist),
                    ("Year", AlbumSortField::Year),
                    ("Tracks", AlbumSortField::TrackCount),
                    ("Duration", AlbumSortField::Duration),
                    ("Date Added", AlbumSortField::DateAdded),
                ],
                self.sort_field,
                self.sort_dir,
                Message::AlbumSortChanged,
            ),
            Space::with_width(Length::Fill),
            components::view_mode_toggle(theme_cfg, self.view_mode, Message::SetAlbumsViewMode),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 0.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        });

        let content_area: Element<'a, Message> = match self.view_mode {
            ViewMode::Tile => {
                let mut tiles = row![].spacing(14);
                let tile_w = self.tile_width as f32;
                let disc_icon_size = (tile_w * 0.32).clamp(32.0, 72.0);

                for album in albums {
                    let key = cover_key(album);
                    let cover_art = components::cover_art(
                        theme_cfg,
                        self.album_covers.get(&key),
                        tile_w,
                        Radius::default(),
                        icons::DISC_SVG,
                        disc_icon_size,
                        theme_cfg.primary_color(),
                    );

                    let year_str = album.year.map(|y| format!("{}", y)).unwrap_or_default();
                    let meta = container(
                        column![
                            text(theme_cfg, &album.album)
                                .size(13)
                                .color(theme_cfg.text_color()),
                            row![
                                text(theme_cfg, &album.artist)
                                    .size(11)
                                    .color(theme_cfg.text_muted_color())
                                    .width(Length::Fill),
                                text(theme_cfg, year_str)
                                    .size(11)
                                    .color(theme_cfg.text_muted_color()),
                            ]
                            .align_y(Alignment::Center),
                            row![
                                text(theme_cfg, format!("{} tracks", album.track_count))
                                    .size(10)
                                    .color(theme_cfg.text_muted_color()),
                                Space::with_width(Length::Fill),
                                text(theme_cfg, format_duration(album.total_duration_ms))
                                    .size(10)
                                    .color(theme_cfg.text_muted_color()),
                            ]
                            .align_y(Alignment::Center),
                        ]
                        .spacing(3)
                        .padding(8),
                    )
                    .width(tile_w);

                    let card_content = column![cover_art, meta];
                    let card_btn = button(card_content)
                        .style(styles::tile(theme_cfg))
                        .padding(0)
                        .on_press(Message::SelectAlbum(album.album.clone()));

                    tiles = tiles.push(card_btn);
                }

                scrollable(
                    container(tiles.wrap())
                        .width(Length::Fill)
                        .padding(Padding {
                            left: 16.0,
                            right: 16.0,
                            ..Default::default()
                        }),
                )
                .on_scroll(Message::AlbumsScrolled)
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg))
                .into()
            }
            ViewMode::List => {
                let mut album_list = column![].spacing(4);

                for album in albums {
                    let key = cover_key(album);
                    let disc_thumbnail = components::cover_art(
                        theme_cfg,
                        self.album_covers.get(&key),
                        42.0,
                        Radius::from(4.0),
                        icons::DISC_SVG,
                        20.0,
                        theme_cfg.primary_color(),
                    );

                    let subtitle = if let Some(year) = album.year {
                        format!("{} • {}", album.artist, year)
                    } else {
                        album.artist.clone()
                    };

                    let title_block = column![
                        text(theme_cfg, &album.album)
                            .size(13)
                            .color(theme_cfg.text_color()),
                        text(theme_cfg, subtitle)
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .spacing(2)
                    .width(Length::Fill);

                    let details = column![
                        text(theme_cfg, format!("{} tracks", album.track_count))
                            .size(12)
                            .color(theme_cfg.text_muted_color()),
                        text(theme_cfg, format_duration(album.total_duration_ms))
                            .size(11)
                            .color(theme_cfg.text_muted_color()),
                    ]
                    .spacing(2)
                    .align_x(Alignment::End)
                    .width(100);

                    let album_row =
                        row![disc_thumbnail, Space::with_width(12), title_block, details,]
                            .spacing(4)
                            .align_y(Alignment::Center)
                            .padding([0, 8]);

                    let row_btn = button(album_row)
                        .style(styles::tile(theme_cfg))
                        .width(Length::Fill)
                        .on_press(Message::SelectAlbum(album.album.clone()));

                    album_list = album_list.push(row_btn);
                }

                scrollable(album_list)
                    .on_scroll(Message::AlbumsScrolled)
                    .height(Length::Fill)
                    .style(styles::scrollable_style(theme_cfg))
                    .into()
            }
        };

        column![header_bar, content_area]
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
