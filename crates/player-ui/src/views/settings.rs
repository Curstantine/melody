use iced::border::Radius;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Border, Element, Length};

use crate::icons;
use crate::message::{Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

pub fn view<'a>(
    theme_cfg: &'a ThemeConfig,
    theme_names: Vec<String>,
    active_theme_name: &'a str,
    total_tracks: usize,
    total_albums: usize,
    total_artists: usize,
    is_scanning: bool,
    page_size: usize,
    library_view_mode: ViewMode,
    volume: f32,
) -> Element<'a, Message> {
    // 1. Settings Header
    let header = column![
        text("Preferences & Settings")
            .size(22)
            .color(theme_cfg.text_color()),
        text("Configure your music library, theme appearance, and audio engine")
            .size(13)
            .color(theme_cfg.text_muted_color()),
    ]
    .spacing(4);

    // Section 1: Appearance & Theming
    let theme_buttons = {
        let mut row_wrap = row![].spacing(8);
        for name in &theme_names {
            let is_active = name == active_theme_name;
            let btn = button(
                row![
                    if is_active {
                        icons::icon(icons::CHECK_SVG, 12.0, theme_cfg.primary_color())
                    } else {
                        icons::icon(icons::DISC_SVG, 12.0, theme_cfg.text_muted_color())
                    },
                    Space::with_width(6),
                    text(name.clone())
                        .size(12)
                        .color(if is_active {
                            theme_cfg.primary_color()
                        } else {
                            theme_cfg.text_color()
                        }),
                ]
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(styles::ghost_button(theme_cfg, is_active))
            .on_press(Message::SelectTheme(name.clone()));

            row_wrap = row_wrap.push(btn);
        }
        row_wrap.wrap()
    };

    let palette_preview = row![
        swatch_card("Primary", theme_cfg.primary_color()),
        swatch_card("Accent", theme_cfg.accent_color()),
        swatch_card("Player Bar", theme_cfg.player_bar_bg_color()),
        swatch_card("Surface", theme_cfg.surface_color()),
        swatch_card("Background", theme_cfg.bg_color()),
    ]
    .spacing(12);

    let reload_themes_btn = button(
        row![
            icons::icon(icons::REFRESH_SVG, 14.0, theme_cfg.text_color()),
            Space::with_width(6),
            text("Reload Themes from Disk").size(12).color(theme_cfg.text_color()),
        ]
        .align_y(Alignment::Center),
    )
    .padding([6, 12])
    .style(styles::ghost_button(theme_cfg, false))
    .on_press(Message::ReloadThemes);

    let appearance_section = settings_card(
        theme_cfg,
        "APPEARANCE & THEME",
        column![
            text("Active Skin & Theme Palette")
                .size(13)
                .color(theme_cfg.text_color()),
            theme_buttons,
            Space::with_height(8),
            text("Current Theme Color Swatches")
                .size(12)
                .color(theme_cfg.text_muted_color()),
            palette_preview,
            Space::with_height(4),
            reload_themes_btn,
        ]
        .spacing(10),
    );

    // Section 2: Library & Music Folders
    let scan_btn = button(
        row![
            icons::icon(icons::FOLDER_SVG, 14.0, theme_cfg.primary_color()),
            Space::with_width(8),
            text(if is_scanning {
                "Scanning Library..."
            } else {
                "+ Scan Music Folder"
            })
            .size(13)
            .color(theme_cfg.text_color()),
        ]
        .align_y(Alignment::Center),
    )
    .padding([8, 16])
    .style(styles::ghost_button(theme_cfg, false))
    .on_press_maybe(if is_scanning {
        None
    } else {
        Some(Message::ScanFolderClicked)
    });

    let page_sizes = [25, 50, 100, 200];
    let page_size_buttons = {
        let mut r = row![].spacing(6).align_y(Alignment::Center);
        r = r.push(
            text("Viewport Virtualization Limit:")
                .size(12)
                .color(theme_cfg.text_muted_color()),
        );
        for &size in &page_sizes {
            let is_sel = page_size == size;
            let btn = button(
                text(format!("{} tracks", size))
                    .size(11)
                    .color(if is_sel {
                        theme_cfg.primary_color()
                    } else {
                        theme_cfg.text_color()
                    }),
            )
            .padding([4, 8])
            .style(styles::ghost_button(theme_cfg, is_sel))
            .on_press(Message::SetPageSize(size));
            r = r.push(btn);
        }
        r
    };

    let library_stats = row![
        metric_box(theme_cfg, "Tracks Indexed", format!("{}", total_tracks)),
        metric_box(theme_cfg, "Albums", format!("{}", total_albums)),
        metric_box(theme_cfg, "Artists", format!("{}", total_artists)),
        metric_box(theme_cfg, "Storage Engine", "SQLite WAL".to_string()),
    ]
    .spacing(12);

    let library_section = settings_card(
        theme_cfg,
        "MUSIC LIBRARY & STORAGE",
        column![
            text("Library Overview & Music Directories")
                .size(13)
                .color(theme_cfg.text_color()),
            library_stats,
            Space::with_height(6),
            row![
                scan_btn,
                Space::with_width(12),
                page_size_buttons,
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    );

    // Section 3: View Modes & Layout
    let view_mode_section = settings_card(
        theme_cfg,
        "DEFAULT VIEW MODES",
        column![
            text("Preferred display style for library views")
                .size(13)
                .color(theme_cfg.text_color()),
            row![
                text("Tracks View:")
                    .size(12)
                    .color(theme_cfg.text_muted_color())
                    .width(100),
                button(
                    row![
                        icons::icon(icons::LIST_VIEW_SVG, 12.0, theme_cfg.text_color()),
                        Space::with_width(4),
                        text("List Table").size(12).color(theme_cfg.text_color()),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([4, 10])
                .style(styles::ghost_button(theme_cfg, library_view_mode == ViewMode::List))
                .on_press(Message::SetLibraryViewMode(ViewMode::List)),
                Space::with_width(8),
                button(
                    row![
                        icons::icon(icons::GRID_VIEW_SVG, 12.0, theme_cfg.text_color()),
                        Space::with_width(4),
                        text("Tile Cards").size(12).color(theme_cfg.text_color()),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding([4, 10])
                .style(styles::ghost_button(theme_cfg, library_view_mode == ViewMode::Tile))
                .on_press(Message::SetLibraryViewMode(ViewMode::Tile)),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    );

    // Section 4: Audio Engine & Architecture
    let audio_section = settings_card(
        theme_cfg,
        "AUDIO PIPELINE & PERFORMANCE",
        column![
            text("Lock-Free Realtime Audio Architecture")
                .size(13)
                .color(theme_cfg.text_color()),
            row![
                metric_box(theme_cfg, "Worker Thread", "Decoupled".to_string()),
                metric_box(theme_cfg, "Resampler", "Linear Interp".to_string()),
                metric_box(theme_cfg, "Ring Buffer", "SPSC Lock-Free".to_string()),
                metric_box(theme_cfg, "Volume", format!("{:.0}%", volume * 100.0)),
            ]
            .spacing(12),
            text("Audio thread operates with zero UI locks and zero allocations in realtime callbacks.")
                .size(12)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(10),
    );

    let content = column![
        header,
        appearance_section,
        library_section,
        view_mode_section,
        audio_section,
        Space::with_height(20),
    ]
    .spacing(20)
    .padding(24)
    .width(Length::Fill);

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn settings_card<'a>(
    theme_cfg: &'a ThemeConfig,
    title: &'static str,
    content: iced::widget::Column<'a, Message>,
) -> Element<'a, Message> {
    container(
        column![
            text(title)
                .size(11)
                .color(theme_cfg.primary_color()),
            content,
        ]
        .spacing(12)
        .padding(16),
    )
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(theme_cfg.surface_color())),
        border: Border {
            radius: Radius::from(6.0),
            color: theme_cfg.border_color(),
            width: 1.0,
        },
        ..Default::default()
    })
    .width(Length::Fill)
    .into()
}

fn swatch_card<'a>(label: &'static str, color: iced::Color) -> Element<'a, Message> {
    column![
        container(Space::with_width(60).height(24))
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(color)),
                border: Border {
                    radius: Radius::from(4.0),
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1),
                    width: 1.0,
                },
                ..Default::default()
            }),
        text(label).size(10).color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .into()
}

fn metric_box<'a>(
    theme_cfg: &'a ThemeConfig,
    label: &'static str,
    value: String,
) -> Element<'a, Message> {
    container(
        column![
            text(value)
                .size(16)
                .color(theme_cfg.text_color()),
            text(label)
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .padding([8, 12]),
    )
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(theme_cfg.surface_hover_color())),
        border: Border {
            radius: Radius::from(4.0),
            color: theme_cfg.border_color(),
            width: 1.0,
        },
        ..Default::default()
    })
    .width(Length::FillPortion(1))
    .into()
}
