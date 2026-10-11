use iced::border::Radius;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Border, Element, Length};
use player_core::ArtistSummary;

use crate::icons;
use crate::message::{Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    artists: &'a [ArtistSummary],
    view_mode: ViewMode,
) -> Element<'a, Message> {
    if artists.is_empty() {
        return container(
            column![
                text("No artists indexed")
                    .size(16)
                    .color(theme_cfg.text_color()),
                text("Scan a folder in Preferences to populate the artist catalog")
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
    .on_press(Message::SetArtistsViewMode(ViewMode::Tile));

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
    .on_press(Message::SetArtistsViewMode(ViewMode::List));

    let header_bar = row![
        text(format!("Artists ({})", artists.len()))
            .size(18)
            .color(theme_cfg.text_color()),
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

            for artist in artists {
                let avatar = container(
                    icons::icon(icons::USER_AVATAR_SVG, 42.0, theme_cfg.primary_color()),
                )
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
                        text(&artist.artist)
                            .size(13)
                            .color(theme_cfg.text_color()),
                        text(format!(
                            "{} albums • {} tracks",
                            artist.album_count, artist.track_count
                        ))
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

            scrollable(container(tiles.wrap()).width(Length::Fill))
                .height(Length::Fill)
                .into()
        }
        ViewMode::List => {
            let mut artist_list = column![].spacing(4);

            for artist in artists {
                let avatar = container(
                    icons::icon(icons::USER_AVATAR_SVG, 18.0, theme_cfg.primary_color()),
                )
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
                    text(&artist.artist)
                        .size(13)
                        .color(theme_cfg.text_color()),
                    text(format!(
                        "{} albums • {} tracks",
                        artist.album_count, artist.track_count
                    ))
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
                ]
                .spacing(2)
                .width(Length::Fill);

                let artist_row = row![
                    avatar,
                    Space::with_width(12),
                    title_block,
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([6, 12]);

                let row_btn = button(artist_row)
                    .style(styles::ghost_button(theme_cfg, false))
                    .width(Length::Fill)
                    .on_press(Message::SelectArtist(artist.artist.clone()));

                artist_list = artist_list.push(row_btn);
            }

            scrollable(artist_list).height(Length::Fill).into()
        }
    };

    column![header_bar, Space::with_height(6), content_area]
        .spacing(6)
        .padding(16)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
