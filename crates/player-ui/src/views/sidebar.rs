use iced::widget::{button, column, container, pick_list, text, Space};
use iced::{Element, Length};

use crate::message::{Message, ViewTab};
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    active_tab: ViewTab,
    theme_cfg: &'a ThemeConfig,
    theme_names: Vec<String>,
    active_theme: String,
    total_tracks: usize,
    is_scanning: bool,
) -> Element<'a, Message> {
    let brand = column![
        text("VIBECORE")
            .size(20)
            .color(theme_cfg.primary_color()),
        text("Hi-Fi Rust Music Player")
            .size(11)
            .color(theme_cfg.text_muted_color()),
    ]
    .spacing(2);

    let nav_btn = |label: &'static str, tab: ViewTab| {
        let is_selected = active_tab == tab;
        button(
            text(label)
                .size(14)
                .color(if is_selected {
                    theme_cfg.primary_color()
                } else {
                    theme_cfg.text_color()
                }),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(styles::ghost_button(theme_cfg, is_selected))
        .on_press(Message::SelectTab(tab))
    };

    let nav_items = column![
        nav_btn("Tracks", ViewTab::Tracks),
        nav_btn("Albums", ViewTab::Albums),
        nav_btn("Artists", ViewTab::Artists),
        nav_btn("Now Playing", ViewTab::Lyrics),
    ]
    .spacing(4);

    let scan_label = if is_scanning {
        "Scanning..."
    } else {
        "+ Scan Folder"
    };

    let scan_btn = button(
        text(scan_label)
            .size(13)
            .color(theme_cfg.surface_color()),
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
        text("THEME")
            .size(11)
            .color(theme_cfg.text_muted_color()),
        pick_list(theme_names, Some(active_theme), Message::SelectTheme)
            .width(Length::Fill),
    ]
    .spacing(4);

    let stats = text(format!("{} tracks indexed", total_tracks))
        .size(11)
        .color(theme_cfg.text_muted_color());

    let content = column![
        brand,
        Space::with_height(16),
        nav_items,
        Space::with_height(16),
        scan_btn,
        Space::with_height(Length::Fill),
        theme_picker,
        Space::with_height(8),
        stats,
    ]
    .spacing(6)
    .padding(16)
    .width(220);

    container(content)
        .style(styles::sidebar_container(theme_cfg))
        .height(Length::Fill)
        .into()
}
