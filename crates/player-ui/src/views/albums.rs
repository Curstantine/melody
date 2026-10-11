use std::collections::HashMap;

use iced::border::Radius;
use iced::widget::{button, column, container, image, row, scrollable, text, Space};
use iced::{Alignment, Border, ContentFit, Element, Length};
use player_core::{format_duration, AlbumSummary, SortDirection};

use crate::icons;
use crate::message::{AlbumSortField, Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    albums: &'a [AlbumSummary],
    album_covers: &'a HashMap<String, image::Handle>,
    view_mode: ViewMode,
    active_sort_field: AlbumSortField,
    sort_dir: SortDirection,
) -> Element<'a, Message> {
    if albums.is_empty() {
        return container(
            column![
                text("No albums indexed")
                    .size(16)
                    .color(theme_cfg.text_color()),
                text("Scan a folder to populate the album catalog")
                    .size(13)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .padding(40)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into();
    }

    let sort_chip = |label: &str, field: AlbumSortField| {
        let is_active = active_sort_field == field;
        let arrow = if is_active {
            match sort_dir {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            }
        } else {
            ""
        };
        button(
            text(format!("{}{}", label, arrow))
                .size(11)
                .color(if is_active {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_muted_color()
                }),
        )
        .padding([4, 8])
        .style(styles::ghost_button(theme_cfg, is_active))
        .on_press(Message::AlbumSortChanged(field))
    };

    let tile_btn = button(
        row![
            icons::icon(
                icons::GRID_VIEW_SVG,
                13.0,
                if view_mode == ViewMode::Tile {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_muted_color()
                },
            ),
            Space::with_width(4),
            text("Tiles")
                .size(11)
                .color(if view_mode == ViewMode::Tile {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_muted_color()
                }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([5, 8])
    .style(styles::ghost_button(theme_cfg, view_mode == ViewMode::Tile))
    .on_press(Message::SetAlbumsViewMode(ViewMode::Tile));

    let list_btn = button(
        row![
            icons::icon(
                icons::LIST_VIEW_SVG,
                13.0,
                if view_mode == ViewMode::List {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_muted_color()
                },
            ),
            Space::with_width(4),
            text("List")
                .size(11)
                .color(if view_mode == ViewMode::List {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_muted_color()
                }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([5, 8])
    .style(styles::ghost_button(theme_cfg, view_mode == ViewMode::List))
    .on_press(Message::SetAlbumsViewMode(ViewMode::List));

    let header_bar = row![
        text(format!("Albums ({})", albums.len()))
            .size(18)
            .color(theme_cfg.text_color()),
        Space::with_width(16),
        row![
            text("Sort:")
                .size(11)
                .color(theme_cfg.text_muted_color()),
            Space::with_width(4),
            sort_chip("Title", AlbumSortField::Title),
            sort_chip("Artist", AlbumSortField::Artist),
            sort_chip("Year", AlbumSortField::Year),
            sort_chip("Tracks", AlbumSortField::TrackCount),
            sort_chip("Duration", AlbumSortField::Duration),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
        Space::with_width(Length::Fill),
        tile_btn,
        list_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([0, 4]);

    let content_area: Element<'a, Message> = match view_mode {
        ViewMode::Tile => {
            let mut tiles = row![].spacing(14);

            for album in albums {
                let key = format!("{}:{}", album.artist, album.album);
                let cover_art = if let Some(handle) = album_covers.get(&key) {
                    container(
                        image(handle.clone())
                            .width(170)
                            .height(170)
                            .content_fit(ContentFit::Cover),
                    )
                    .width(170)
                    .height(170)
                    .center_x(170)
                    .center_y(170)
                    .style(move |_theme| container::Style {
                        border: Border {
                            radius: Radius {
                                top_left: 6.0,
                                top_right: 6.0,
                                bottom_left: 0.0,
                                bottom_right: 0.0,
                            },
                            color: theme_cfg.border_color(),
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                } else {
                    container(
                        icons::icon(icons::DISC_SVG, 54.0, theme_cfg.primary_color()),
                    )
                    .width(170)
                    .height(170)
                    .center_x(170)
                    .center_y(170)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                        border: Border {
                            radius: Radius {
                                top_left: 6.0,
                                top_right: 6.0,
                                bottom_left: 0.0,
                                bottom_right: 0.0,
                            },
                            color: theme_cfg.border_color(),
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                };

                let year_str = album.year.map(|y| format!("{}", y)).unwrap_or_default();
                let meta = container(
                    column![
                        text(&album.album)
                            .size(13)
                            .color(theme_cfg.text_color()),
                        row![
                            text(&album.artist)
                                .size(11)
                                .color(theme_cfg.text_muted_color())
                                .width(Length::Fill),
                            text(year_str)
                                .size(11)
                                .color(theme_cfg.text_muted_color()),
                        ]
                        .align_y(Alignment::Center),
                        row![
                            text(format!("{} tracks", album.track_count))
                                .size(10)
                                .color(theme_cfg.text_muted_color()),
                            Space::with_width(Length::Fill),
                            text(format_duration(album.total_duration_ms))
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
                    .style(styles::ghost_button(theme_cfg, false))
                    .padding(0)
                    .on_press(Message::SelectAlbum(album.album.clone()));

                tiles = tiles.push(card_btn);
            }

            scrollable(container(tiles.wrap()).width(Length::Fill))
                .height(Length::Fill)
                .into()
        }
        ViewMode::List => {
            let mut album_list = column![].spacing(4);

            for album in albums {
                let key = format!("{}:{}", album.artist, album.album);
                let disc_thumbnail = if let Some(handle) = album_covers.get(&key) {
                    container(
                        image(handle.clone())
                            .width(42)
                            .height(42)
                            .content_fit(ContentFit::Cover),
                    )
                    .width(42)
                    .height(42)
                    .center_x(42)
                    .center_y(42)
                    .style(move |_theme| container::Style {
                        border: Border {
                            color: theme_cfg.border_color(),
                            width: 1.0,
                            radius: Radius::from(4.0),
                        },
                        ..Default::default()
                    })
                } else {
                    container(
                        icons::icon(icons::DISC_SVG, 20.0, theme_cfg.primary_color()),
                    )
                    .width(42)
                    .height(42)
                    .center_x(42)
                    .center_y(42)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
                        border: Border {
                            color: theme_cfg.border_color(),
                            width: 1.0,
                            radius: Radius::from(4.0),
                        },
                        ..Default::default()
                    })
                };

                let subtitle = if let Some(year) = album.year {
                    format!("{} • {}", album.artist, year)
                } else {
                    album.artist.clone()
                };

                let title_block = column![
                    text(&album.album)
                        .size(13)
                        .color(theme_cfg.text_color()),
                    text(subtitle)
                        .size(11)
                        .color(theme_cfg.text_muted_color()),
                ]
                .spacing(2)
                .width(Length::Fill);

                let details = column![
                    text(format!("{} tracks", album.track_count))
                        .size(12)
                        .color(theme_cfg.text_muted_color()),
                    text(format_duration(album.total_duration_ms))
                        .size(11)
                        .color(theme_cfg.text_muted_color()),
                ]
                .spacing(2)
                .align_x(Alignment::End)
                .width(100);

                let album_row = row![
                    disc_thumbnail,
                    Space::with_width(12),
                    title_block,
                    details,
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([6, 12]);

                let row_btn = button(album_row)
                    .style(styles::ghost_button(theme_cfg, false))
                    .width(Length::Fill)
                    .on_press(Message::SelectAlbum(album.album.clone()));

                album_list = album_list.push(row_btn);
            }

            scrollable(album_list).height(Length::Fill).into()
        }
    };

    column![header_bar, Space::with_height(6), content_area]
        .spacing(6)
        .padding(16)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
