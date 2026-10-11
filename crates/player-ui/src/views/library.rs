use iced::border::Radius;
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Border, Element, Length};
use player_core::{SortDirection, SortField, Track};

use crate::components;
use crate::icons;
use crate::message::{Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    tracks: &'a [Track],
    current_track_id: Option<i64>,
    selected_track_id: Option<i64>,
    hovered_track_id: Option<i64>,
    is_playing: bool,
    view_mode: ViewMode,
    search_query: &'a str,
    sort_field: SortField,
    sort_dir: SortDirection,
    page: usize,
    page_size: usize,
) -> Element<'a, Message> {
    // 1. Search Bar & View Mode Toolbar
    let search_input = text_input("Search tracks, artists, albums...", search_query)
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
        Space::with_width(12),
        components::view_mode_toggle(theme_cfg, view_mode, Message::SetLibraryViewMode),
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
        let is_active = sort_field == field;

        let content: Element<'a, Message> = if is_active {
            let sort_icon_svg = match sort_dir {
                SortDirection::Ascending => icons::CHEVRON_UP_SVG,
                SortDirection::Descending => icons::CHEVRON_DOWN_SVG,
            };
            row![
                text(label).size(12),
                Space::with_width(4),
                icons::icon(sort_icon_svg, 12.0, theme_cfg.primary_color()),
            ]
            .align_y(Alignment::Center)
            .into()
        } else {
            text(label).size(12).into()
        };

        button(content)
            .style(styles::column_header_button(theme_cfg, is_active))
            .padding([2, 0])
            .width(width)
            .on_press(Message::SortChanged(field))
    };

    let table_header = row![
        container(text("#").size(12).color(theme_cfg.text_muted_color()),)
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
        bottom: 6.0,
        left: 0.0,
    });

    // 3. Track Content (List vs Tile)
    let content: Element<'a, Message> = if tracks.is_empty() {
        components::empty_state(
            theme_cfg,
            "No tracks found",
            "Scan a folder in Preferences to index your music library",
        )
    } else {
        match view_mode {
            ViewMode::List => {
                let mut track_items = column![].spacing(2);

                for (idx, track) in tracks.iter().enumerate() {
                    let is_active = current_track_id == Some(track.id);
                    let is_selected = selected_track_id == Some(track.id);
                    let is_hovered = hovered_track_id == Some(track.id);
                    let display_index = (page * page_size) + idx + 1;

                    // Left-most action: # index replaced by Play button on hover, or playing status
                    let left_action: Element<'a, Message> = if is_active {
                        if is_hovered {
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
                            container(icons::icon(
                                icons::SPEAKER_PLAYING_SVG,
                                13.0,
                                theme_cfg.primary_color(),
                            ))
                            .width(36)
                            .height(24)
                            .center_x(36)
                            .center_y(24)
                            .into()
                        }
                    } else if is_hovered {
                        button(
                            container(icons::icon(
                                icons::PLAY_SVG,
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
                        .on_press(Message::PlayTrack(track.id))
                        .into()
                    } else {
                        container(
                            text(format!("{}", display_index))
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
                        components::cjk_text(&track.title)
                            .size(13)
                            .color(if is_active {
                                theme_cfg.primary_color()
                            } else {
                                theme_cfg.text_color()
                            })
                            .width(Length::FillPortion(4)),
                        components::cjk_text(&track.artist)
                            .size(12)
                            .color(theme_cfg.text_muted_color())
                            .width(Length::FillPortion(3)),
                        components::cjk_text(&track.album)
                            .size(12)
                            .color(theme_cfg.text_muted_color())
                            .width(Length::FillPortion(3)),
                        text(track.formatted_duration())
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
                            is_active || is_selected || is_hovered,
                        ))
                        .width(Length::Fill);

                    let interactive_row = mouse_area(row_container)
                        .on_press(Message::TrackClicked(track.id))
                        .on_enter(Message::TrackHovered(Some(track.id)))
                        .on_exit(Message::TrackHovered(None));

                    track_items = track_items.push(interactive_row);
                }

                scrollable(track_items)
                    .on_scroll(Message::LibraryScrolled)
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(styles::scrollable_style(theme_cfg))
                    .into()
            }
            ViewMode::Tile => {
                let mut tiles_wrap = row![].spacing(12);

                for track in tracks.iter() {
                    let is_active = current_track_id == Some(track.id);
                    let is_selected = selected_track_id == Some(track.id);
                    let is_hovered = hovered_track_id == Some(track.id);

                    let disc_box = container(if is_active && !is_hovered {
                        icons::icon(icons::SPEAKER_PLAYING_SVG, 32.0, theme_cfg.primary_color())
                    } else if is_hovered {
                        icons::icon(icons::PLAY_SVG, 32.0, theme_cfg.primary_color())
                    } else {
                        icons::icon(icons::DISC_SVG, 36.0, theme_cfg.text_muted_color())
                    })
                    .width(160)
                    .height(110)
                    .center_x(160)
                    .center_y(110)
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
                    });

                    let info_box = container(
                        column![
                            components::cjk_text(&track.title)
                                .size(13)
                                .color(if is_active {
                                    theme_cfg.primary_color()
                                } else {
                                    theme_cfg.text_color()
                                }),
                            components::cjk_text(&track.artist)
                                .size(11)
                                .color(theme_cfg.text_muted_color()),
                            row![
                                text(track.formatted_duration())
                                    .size(10)
                                    .color(theme_cfg.text_muted_color()),
                                Space::with_width(Length::Fill),
                                if is_active {
                                    text("Playing").size(10).color(theme_cfg.primary_color())
                                } else {
                                    text("").size(10).color(theme_cfg.text_muted_color())
                                },
                            ]
                            .align_y(Alignment::Center),
                        ]
                        .spacing(3)
                        .padding(8),
                    )
                    .width(160);

                    let card = container(column![disc_box, info_box])
                        .style(move |_theme| container::Style {
                            background: Some(iced::Background::Color(
                                if is_active || is_selected {
                                    theme_cfg.surface_hover_color()
                                } else {
                                    theme_cfg.surface_color()
                                },
                            )),
                            border: Border {
                                radius: Radius::from(6.0),
                                color: if is_active || is_selected {
                                    theme_cfg.primary_color()
                                } else {
                                    theme_cfg.border_color()
                                },
                                width: 1.0,
                            },
                            ..Default::default()
                        })
                        .width(160);

                    let interactive_card = mouse_area(card)
                        .on_press(Message::TrackClicked(track.id))
                        .on_enter(Message::TrackHovered(Some(track.id)))
                        .on_exit(Message::TrackHovered(None));

                    tiles_wrap = tiles_wrap.push(interactive_card);
                }

                scrollable(
                    container(tiles_wrap.wrap())
                        .padding([8, 4])
                        .width(Length::Fill),
                )
                .on_scroll(Message::LibraryScrolled)
                .height(Length::Fill)
                .width(Length::Fill)
                .style(styles::scrollable_style(theme_cfg))
                .into()
            }
        }
    };

    let mut main_col = column![toolbar];
    if view_mode == ViewMode::List {
        main_col = main_col.push(table_header);
    }
    main_col = main_col.push(content);

    main_col
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
