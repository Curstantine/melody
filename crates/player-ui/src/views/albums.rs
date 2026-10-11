use std::collections::HashMap;

use iced::border::Radius;
use iced::widget::{button, column, container, image, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use player_core::{format_duration, AlbumSummary, SortDirection};

use crate::components;
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
            ],
            active_sort_field,
            sort_dir,
            Message::AlbumSortChanged,
        ),
        Space::with_width(Length::Fill),
        components::view_mode_toggle(theme_cfg, view_mode, Message::SetAlbumsViewMode),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding(iced::Padding {
        top: 0.0,
        right: 16.0,
        bottom: 6.0,
        left: 0.0,
    });

    let content_area: Element<'a, Message> = match view_mode {
        ViewMode::Tile => {
            let mut tiles = row![].spacing(14);

            for album in albums {
                let key = format!("{}:{}", album.artist, album.album);
                let cover_art = components::cover_art(
                    theme_cfg,
                    album_covers.get(&key),
                    170.0,
                    Radius {
                        top_left: 6.0,
                        top_right: 6.0,
                        bottom_left: 0.0,
                        bottom_right: 0.0,
                    },
                    icons::DISC_SVG,
                    54.0,
                    theme_cfg.primary_color(),
                );

                let year_str = album.year.map(|y| format!("{}", y)).unwrap_or_default();
                let meta = container(
                    column![
                        components::cjk_text(&album.album)
                            .size(13)
                            .color(theme_cfg.text_color()),
                        row![
                            components::cjk_text(&album.artist)
                                .size(11)
                                .color(theme_cfg.text_muted_color())
                                .width(Length::Fill),
                            text(year_str).size(11).color(theme_cfg.text_muted_color()),
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
                .style(styles::scrollable_style(theme_cfg))
                .into()
        }
        ViewMode::List => {
            let mut album_list = column![].spacing(4);

            for album in albums {
                let key = format!("{}:{}", album.artist, album.album);
                let disc_thumbnail = components::cover_art(
                    theme_cfg,
                    album_covers.get(&key),
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
                    components::cjk_text(&album.album)
                        .size(13)
                        .color(theme_cfg.text_color()),
                    components::cjk_text(subtitle)
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

                let album_row = row![disc_thumbnail, Space::with_width(12), title_block, details,]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .padding([6, 12]);

                let row_btn = button(album_row)
                    .style(styles::ghost_button(theme_cfg, false))
                    .width(Length::Fill)
                    .on_press(Message::SelectAlbum(album.album.clone()));

                album_list = album_list.push(row_btn);
            }

            scrollable(album_list)
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
            left: 16.0,
        })
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
