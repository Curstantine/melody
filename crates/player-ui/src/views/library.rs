use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};
use player_core::{SortDirection, SortField, Track};

use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    tracks: &'a [Track],
    current_track_id: Option<i64>,
    search_query: &'a str,
    sort_field: SortField,
    sort_dir: SortDirection,
    page: usize,
    page_size: usize,
    total_tracks: usize,
) -> Element<'a, Message> {
    // 1. Search Bar
    let search_input = text_input("Search tracks, artists, albums...", search_query)
        .on_input(Message::SearchInputChanged)
        .padding(10)
        .style(styles::search_input_style(theme_cfg))
        .width(Length::Fill);

    let search_bar = row![search_input]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // 2. Table Header
    let dir_arrow = match sort_dir {
        SortDirection::Ascending => " ▲",
        SortDirection::Descending => " ▼",
    };

    let header_btn = |label: &str, field: SortField, width: Length| {
        let is_active = sort_field == field;
        let title = if is_active {
            format!("{}{}", label, dir_arrow)
        } else {
            label.to_string()
        };

        button(
            text(title)
                .size(12)
                .color(if is_active {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_muted_color()
                }),
        )
        .style(styles::ghost_button(theme_cfg, false))
        .padding([4, 6])
        .width(width)
        .on_press(Message::SortChanged(field))
    };

    let table_header = row![
        text("#")
            .size(12)
            .color(theme_cfg.text_muted_color())
            .width(40),
        header_btn("Title", SortField::Title, Length::FillPortion(4)),
        header_btn("Artist", SortField::Artist, Length::FillPortion(3)),
        header_btn("Album", SortField::Album, Length::FillPortion(3)),
        header_btn("Duration", SortField::Duration, Length::Fixed(80.0)),
        Space::with_width(50), // For play button column
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([8, 12]);

    // 3. Track Rows
    let rows: Element<'a, Message> = if tracks.is_empty() {
        container(
            column![
                text("No tracks found")
                    .size(16)
                    .color(theme_cfg.text_color()),
                text("Click '+ Scan Folder' on the sidebar to index your music library")
                    .size(13)
                    .color(theme_cfg.text_muted_color()),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .padding(40)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
    } else {
        let mut track_items = column![].spacing(2);

        for (idx, track) in tracks.iter().enumerate() {
            let is_active = current_track_id == Some(track.id);
            let display_index = (page * page_size) + idx + 1;

            let play_svg = if is_active {
                icons::PAUSE_SVG
            } else {
                icons::PLAY_SVG
            };
            let play_color = if is_active {
                theme_cfg.primary_color()
            } else {
                theme_cfg.text_color()
            };

            let play_btn = button(icons::icon(play_svg, 12.0, play_color))
                .style(styles::icon_control_button(theme_cfg))
                .padding([4, 8])
                .on_press(Message::PlayTrack(track.id));

            let track_row = row![
                text(format!("{}", display_index))
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(40),
                text(&track.title)
                    .size(13)
                    .color(if is_active {
                        theme_cfg.primary_color()
                    } else {
                        theme_cfg.text_color()
                    })
                    .width(Length::FillPortion(4)),
                text(&track.artist)
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(Length::FillPortion(3)),
                text(&track.album)
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(Length::FillPortion(3)),
                text(track.formatted_duration())
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(80),
                play_btn,
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([6, 12]);

            let row_container = container(track_row)
                .style(styles::table_row_container(theme_cfg, is_active))
                .width(Length::Fill);

            track_items = track_items.push(row_container);
        }

        scrollable(track_items)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    };

    // 4. Pagination Footer
    let total_pages = if total_tracks == 0 {
        1
    } else {
        (total_tracks + page_size - 1) / page_size
    };

    let start_idx = if total_tracks == 0 {
        0
    } else {
        page * page_size + 1
    };
    let end_idx = (start_idx + tracks.len()).saturating_sub(1);

    let prev_btn = button(text("◀ Prev").size(12).color(theme_cfg.text_color()))
        .style(styles::ghost_button(theme_cfg, false))
        .padding([4, 10])
        .on_press_maybe(if page > 0 {
            Some(Message::PrevPage)
        } else {
            None
        });

    let next_btn = button(text("Next ▶").size(12).color(theme_cfg.text_color()))
        .style(styles::ghost_button(theme_cfg, false))
        .padding([4, 10])
        .on_press_maybe(if page + 1 < total_pages {
            Some(Message::NextPage)
        } else {
            None
        });

    let pagination_info = text(format!(
        "Showing {}-{} of {} tracks (Page {} of {})",
        start_idx,
        end_idx,
        total_tracks,
        page + 1,
        total_pages
    ))
    .size(12)
    .color(theme_cfg.text_muted_color());

    let pagination_bar = row![
        pagination_info,
        Space::with_width(Length::Fill),
        prev_btn,
        next_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([8, 12]);

    column![search_bar, table_header, rows, pagination_bar]
        .spacing(8)
        .padding(16)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
