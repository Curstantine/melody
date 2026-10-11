use iced::widget::{button, column, container, pick_list, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeConfig};

#[allow(dead_code)]
pub fn view<'a>(
    active_tab: ViewTab,
    theme_cfg: &'a ThemeConfig,
    theme_names: Vec<String>,
    active_theme: String,
    total_tracks: usize,
    is_scanning: bool,
) -> Element<'a, Message> {
    let brand = row![
        icons::icon(icons::MUSIC_NOTE_SVG, 20.0, theme_cfg.primary_color()),
        Space::with_width(8),
        column![
            text("VIBECORE")
                .size(15)
                .color(theme_cfg.text_color()),
            text("Native Audio Player")
                .size(10)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(1),
    ]
    .align_y(Alignment::Center);

    let nav_btn = |label: &'static str, tab: ViewTab, svg: &'static str| {
        let is_selected = active_tab == tab;
        let color = if is_selected {
            theme_cfg.primary_color()
        } else {
            theme_cfg.text_color()
        };

        let content = row![
            icons::icon(svg, 15.0, color),
            Space::with_width(10),
            text(label)
                .size(13)
                .color(color),
        ]
        .align_y(Alignment::Center)
        .padding([6, 10]);

        button(content)
            .width(Length::Fill)
            .style(styles::ghost_button(theme_cfg, is_selected))
            .on_press(Message::SelectTab(tab))
    };

    let library_header = text("LIBRARY")
        .size(11)
        .color(theme_cfg.text_muted_color());

    let nav_items = column![
        library_header,
        Space::with_height(4),
        nav_btn("Tracks", ViewTab::Tracks, icons::MUSIC_NOTE_SVG),
        nav_btn("Albums", ViewTab::Albums, icons::ALBUM_SVG),
        nav_btn("Artists", ViewTab::Artists, icons::ARTIST_SVG),
        nav_btn("Now Playing", ViewTab::Lyrics, icons::SPECTRUM_SVG),
    ]
    .spacing(4);

    let scan_label = if is_scanning {
        "Scanning Library..."
    } else {
        "+ Scan Music Folder"
    };

    let scan_btn = button(
        row![
            text(scan_label)
                .size(12)
                .color(theme_cfg.surface_color()),
        ]
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .style(styles::primary_button(theme_cfg))
    .on_press_maybe(if is_scanning {
        None
    } else {
        Some(Message::ScanFolderClicked)
    });

    let theme_picker = column![
        text("SKIN / THEME")
            .size(11)
            .color(theme_cfg.text_muted_color()),
        pick_list(theme_names, Some(active_theme), Message::SelectTheme)
            .width(Length::Fill),
    ]
    .spacing(4);

    let stats = text(format!("{} tracks in library", total_tracks))
        .size(11)
        .color(theme_cfg.text_muted_color());

    let content = column![
        brand,
        Space::with_height(16),
        nav_items,
        Space::with_height(14),
        scan_btn,
        Space::with_height(Length::Fill),
        theme_picker,
        Space::with_height(6),
        stats,
    ]
    .spacing(4)
    .padding([14, 14])
    .width(200);

    container(content)
        .style(styles::sidebar_container(theme_cfg))
        .height(Length::Fill)
        .into()
}
