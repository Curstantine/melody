use iced::border::Radius;
use iced::widget::{button, column, container, row, scrollable, Space};
use iced::{Alignment, Border, Element, Length};
use player_core::ArtistSummary;

use crate::components::{self, text};
use crate::icons;
use crate::message::{Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    artists: &'a [ArtistSummary],
    view_mode: ViewMode,
) -> Element<'a, Message> {
    if artists.is_empty() {
        return components::empty_state(
            theme_cfg,
            "No artists indexed",
            "Scan a folder in Preferences to populate the artist catalog",
        );
    }

    let header_bar = row![
        Space::with_width(Length::Fill),
        components::view_mode_toggle(theme_cfg, view_mode, Message::SetArtistsViewMode),
    ]
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

            scrollable(container(tiles.wrap()).width(Length::Fill))
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
                    .style(styles::ghost_button(theme_cfg, false))
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
            left: 16.0,
        })
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
