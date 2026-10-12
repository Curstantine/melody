use iced::border::Radius;
use iced::widget::{button, column, container, row, scrollable, Space};
use iced::{Alignment, Border, Element, Length, Padding};
use core_engine::{ArtistSummary, SortDirection};

use crate::components::{self, text};
use crate::icons;
use crate::message::{ArtistSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};



pub const DEFAULT_CATALOG_CHUNK_SIZE: usize = 60;

/// Self-contained state and view component for the Artists catalog screen.
pub struct ArtistsView {
    pub view_mode: ViewMode,
    pub artists: Vec<ArtistSummary>,
    pub sort_field: ArtistSortField,
    pub sort_dir: SortDirection,
    pub visible_count: usize,
}

impl ArtistsView {
    pub fn new(view_mode: ViewMode, sort_field: ArtistSortField) -> Self {
        Self {
            view_mode,
            artists: Vec::new(),
            sort_field,
            sort_dir: SortDirection::Ascending,
            visible_count: DEFAULT_CATALOG_CHUNK_SIZE,
        }
    }

    pub fn sort_artists(artists: &mut [ArtistSummary], field: ArtistSortField, dir: SortDirection) {
        artists.sort_by(|a, b| {
            let ord = match field {
                ArtistSortField::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
                ArtistSortField::TrackCount => a
                    .track_count
                    .cmp(&b.track_count)
                    .then_with(|| a.artist.to_lowercase().cmp(&b.artist.to_lowercase())),
                ArtistSortField::AlbumCount => a
                    .album_count
                    .cmp(&b.album_count)
                    .then_with(|| a.artist.to_lowercase().cmp(&b.artist.to_lowercase())),
            };
            match dir {
                SortDirection::Ascending => ord,
                SortDirection::Descending => ord.reverse(),
            }
        });
    }

    pub fn sort(&mut self) {
        Self::sort_artists(&mut self.artists, self.sort_field, self.sort_dir);
    }

    pub fn set_artists(&mut self, mut artists: Vec<ArtistSummary>) {
        Self::sort_artists(&mut artists, self.sort_field, self.sort_dir);
        self.artists = artists;
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn toggle_sort(&mut self, field: ArtistSortField) {
        if self.sort_field == field {
            self.sort_dir = match self.sort_dir {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_field = field;
            self.sort_dir = match field {
                ArtistSortField::TrackCount | ArtistSortField::AlbumCount => {
                    SortDirection::Descending
                }
                _ => SortDirection::Ascending,
            };
        }
        self.sort();
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn on_scroll(&mut self, viewport: &iced::widget::scrollable::Viewport) {
        let rel_offset = viewport.relative_offset();
        if rel_offset.y >= 0.75 && self.visible_count < self.artists.len() {
            self.visible_count =
                (self.visible_count + DEFAULT_CATALOG_CHUNK_SIZE).min(self.artists.len());
        }
    }

    pub fn clear(&mut self) {
        self.artists.clear();
        self.visible_count = DEFAULT_CATALOG_CHUNK_SIZE;
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    pub fn visible_artists(&self) -> &[ArtistSummary] {
        &self.artists[..self.visible_count.min(self.artists.len())]
    }

    pub fn len(&self) -> usize {
        self.artists.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artists.is_empty()
    }

    pub fn view<'a>(&'a self, theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
        let artists = self.visible_artists();
        if artists.is_empty() {
            return components::empty_state(
                theme_cfg,
                "No artists indexed",
                "Scan a folder in Preferences to populate the artist catalog",
            );
        }

        let header_bar = row![
            components::sort_chip_list(
                theme_cfg,
                &[
                    ("Artist", ArtistSortField::Artist),
                    ("Tracks", ArtistSortField::TrackCount),
                    ("Albums", ArtistSortField::AlbumCount),
                ],
                self.sort_field,
                self.sort_dir,
                Message::ArtistSortChanged,
            ),
            Space::with_width(Length::Fill),
            components::view_mode_toggle(theme_cfg, self.view_mode, Message::SetArtistsViewMode),
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

                for artist in artists {
                    let avatar = container(icons::icon(
                        icons::USER_AVATAR_SVG,
                        42.0,
                        theme_cfg.primary_color(),
                    ))
                    .width(80)
                    .height(80)
                    .center_x(80)
                    .center_y(80)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                        border: Border {
                            radius: Radius::from(40.0),
                            color: theme_cfg.border_color(),
                            width: 1.0,
                        },
                        ..Default::default()
                    });

                    let card_content = container(
                        column![
                            avatar,
                            Space::with_height(6),
                            text(theme_cfg, &artist.artist)
                                .size(13)
                                .color(theme_cfg.text_color()),
                            text(
                                theme_cfg,
                                format!(
                                    "{} albums • {} tracks",
                                    artist.album_count, artist.track_count
                                )
                            )
                            .size(10)
                            .color(theme_cfg.text_muted_color()),
                        ]
                        .spacing(3)
                        .align_x(Alignment::Center)
                        .padding(12),
                    )
                    .width(160)
                    .align_x(Alignment::Center)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.surface_color())),
                        border: Border {
                            radius: Radius::from(6.0),
                            color: theme_cfg.border_color(),
                            width: 1.0,
                        },
                        ..Default::default()
                    });

                    let card_btn = button(card_content)
                        .style(styles::ghost_button(theme_cfg, false))
                        .padding(0)
                        .on_press(Message::SelectArtist(artist.artist.clone()));

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
                .on_scroll(Message::ArtistsScrolled)
                .height(Length::Fill)
                .style(styles::scrollable_style(theme_cfg))
                .into()
            }
            ViewMode::List => {
                let mut artist_list = column![].spacing(4);

                for artist in artists {
                    let avatar = container(icons::icon(
                        icons::USER_AVATAR_SVG,
                        18.0,
                        theme_cfg.primary_color(),
                    ))
                    .width(36)
                    .height(36)
                    .center_x(36)
                    .center_y(36)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                        border: Border {
                            color: theme_cfg.border_color(),
                            width: 1.0,
                            radius: Radius::from(18.0),
                        },
                        ..Default::default()
                    });

                    let title_block = column![
                        text(theme_cfg, &artist.artist)
                            .size(13)
                            .color(theme_cfg.text_color()),
                        text(
                            theme_cfg,
                            format!(
                                "{} albums • {} tracks",
                                artist.album_count, artist.track_count
                            )
                        )
                        .size(11)
                        .color(theme_cfg.text_muted_color()),
                    ]
                    .spacing(2)
                    .width(Length::Fill);

                    let artist_row = row![avatar, Space::with_width(12), title_block,]
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .padding([6, 12]);

                    let row_btn = button(artist_row)
                        .style(styles::tile(theme_cfg))
                        .width(Length::Fill)
                        .on_press(Message::SelectArtist(artist.artist.clone()));

                    artist_list = artist_list.push(row_btn);
                }

                scrollable(artist_list)
                    .on_scroll(Message::ArtistsScrolled)
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

