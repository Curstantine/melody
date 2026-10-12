use std::collections::HashMap;
use std::path::PathBuf;

use iced::border::Radius;
use iced::widget::{button, column, container, image, row, scrollable, Space};
use iced::{Alignment, Element, Length, Padding};
use player_core::{format_duration, AlbumSummary, SortDirection};

use crate::components::{self, text};
use crate::icons;
use crate::message::{AlbumSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};



pub const DEFAULT_CATALOG_CHUNK_SIZE: usize = 60;

/// Self-contained state and view component for the Albums catalog screen.
pub struct AlbumsView {
    pub view_mode: ViewMode,
    pub albums: Vec<AlbumSummary>,
    pub album_covers: HashMap<String, image::Handle>,
    pub sort_field: AlbumSortField,
    pub sort_dir: SortDirection,
    pub visible_count: usize,
}

impl AlbumsView {
    pub fn new(view_mode: ViewMode, sort_field: AlbumSortField) -> Self {
        Self {
            view_mode,
            albums: Vec::new(),
            album_covers: HashMap::new(),
            sort_field,
            sort_dir: SortDirection::Ascending,
            visible_count: DEFAULT_CATALOG_CHUNK_SIZE,
        }
    }

    pub fn sort_albums(albums: &mut [AlbumSummary], field: AlbumSortField, dir: SortDirection) {
        albums.sort_by(|a, b| {
            let ord = match field {
                AlbumSortField::Title => a.album.to_lowercase().cmp(&b.album.to_lowercase()),
                AlbumSortField::Artist => a
                    .artist
                    .to_lowercase()
                    .cmp(&b.artist.to_lowercase())
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::Year => a
                    .year
                    .cmp(&b.year)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::Duration => a
                    .total_duration_ms
                    .cmp(&b.total_duration_ms)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
                AlbumSortField::DateAdded => a
                    .added_timestamp
                    .cmp(&b.added_timestamp)
                    .then_with(|| a.album.to_lowercase().cmp(&b.album.to_lowercase())),
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
        let visible_slice = self.visible_albums();
        visible_slice
            .iter()
            .filter_map(|alb| {
                let key = format!("{}:{}", alb.artist, alb.album);
                if self.album_covers.contains_key(&key) {
                    None
                } else {
                    alb.cover_track_path
                        .as_ref()
                        .map(|p| (key, PathBuf::from(p)))
                }
            })
            .collect()
    }

    pub fn insert_covers(&mut self, loaded: Vec<(String, Vec<u8>)>) {
        for (key, bytes) in loaded {
            let handle = image::Handle::from_bytes(bytes);
            self.album_covers.insert(key, handle);
        }
    }

    pub fn clear(&mut self) {
        self.albums.clear();
        self.album_covers.clear();
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn clear_covers(&mut self) {
        self.album_covers.clear();
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

                for album in albums {
                    let key = format!("{}:{}", album.artist, album.album);
                    let cover_art = components::cover_art(
                        theme_cfg,
                        self.album_covers.get(&key),
                        170.0,
                        Radius::default(),
                        icons::DISC_SVG,
                        54.0,
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
                    .width(170);

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
                    let key = format!("{}:{}", album.artist, album.album);
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

                    let album_row = row![disc_thumbnail, Space::with_width(12), title_block, details,]
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

