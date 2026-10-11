use iced::border::Radius;
use iced::widget::{button, column, container, image, row, text, Space};
use iced::{Alignment, Border, ContentFit, Element, Length};
use player_core::SortDirection;

use crate::icons;
use crate::message::{Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

/// A reusable chip button with active state styling.
pub fn chip<'a>(
    theme_cfg: &'a ThemeConfig,
    label: impl Into<String>,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(label.into()).size(11).color(if is_active {
        theme_cfg.primary_color()
    } else {
        theme_cfg.text_muted_color()
    }))
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .into()
}

/// A reusable chip button with an icon and label.
pub fn chip_with_icon<'a>(
    theme_cfg: &'a ThemeConfig,
    icon_svg: &'static str,
    label: impl Into<String>,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let color = if is_active {
        theme_cfg.primary_color()
    } else {
        theme_cfg.text_muted_color()
    };

    button(
        row![
            icons::icon(icon_svg, 13.0, color),
            Space::with_width(4),
            text(label.into()).size(11).color(color),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .into()
}

/// A chip button representing a view mode (List or Tiles).
pub fn view_mode_chip<'a>(
    theme_cfg: &'a ThemeConfig,
    mode: ViewMode,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let (icon_svg, label) = match mode {
        ViewMode::Tile => (icons::GRID_VIEW_SVG, "Tiles"),
        ViewMode::List => (icons::LIST_VIEW_SVG, "List"),
    };
    chip_with_icon(theme_cfg, icon_svg, label, is_active, on_press)
}

/// A unified view mode toggle row containing List and Tiles mode chips.
pub fn view_mode_toggle<'a>(
    theme_cfg: &'a ThemeConfig,
    active_mode: ViewMode,
    on_change: impl Fn(ViewMode) -> Message,
) -> Element<'a, Message> {
    row![
        view_mode_chip(
            theme_cfg,
            ViewMode::List,
            active_mode == ViewMode::List,
            on_change(ViewMode::List),
        ),
        view_mode_chip(
            theme_cfg,
            ViewMode::Tile,
            active_mode == ViewMode::Tile,
            on_change(ViewMode::Tile),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

/// A sort chip displaying the field label and an active sort direction indicator arrow.
pub fn sort_chip<'a, T: Copy + PartialEq>(
    theme_cfg: &'a ThemeConfig,
    label: &'a str,
    field: T,
    active_field: T,
    sort_dir: SortDirection,
    on_press: Message,
) -> Element<'a, Message> {
    let is_active = active_field == field;
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
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .into()
}

/// A unified sort chip list with a "Sort:" label and multiple sort chips.
pub fn sort_chip_list<'a, T: Copy + PartialEq>(
    theme_cfg: &'a ThemeConfig,
    options: &[(&'a str, T)],
    active_field: T,
    sort_dir: SortDirection,
    on_press: impl Fn(T) -> Message,
) -> Element<'a, Message> {
    let mut sort_row = row![
        text("Sort:").size(11).color(theme_cfg.text_muted_color()),
        Space::with_width(4),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    for &(label, field) in options {
        sort_row = sort_row.push(sort_chip(
            theme_cfg,
            label,
            field,
            active_field,
            sort_dir,
            on_press(field),
        ));
    }

    sort_row.into()
}

/// A centered empty state view with a title and muted description.
pub fn empty_state<'a>(
    theme_cfg: &'a ThemeConfig,
    title: &'a str,
    subtitle: &'a str,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(16).color(theme_cfg.text_color()),
            text(subtitle).size(13).color(theme_cfg.text_muted_color()),
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .padding(40)
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .into()
}

/// A container displaying cover art image or a fallback vector icon placeholder.
pub fn cover_art<'a>(
    theme_cfg: &'a ThemeConfig,
    cover: Option<&image::Handle>,
    size: f32,
    radius: impl Into<Radius>,
    fallback_icon_svg: &'static str,
    fallback_icon_size: f32,
    fallback_icon_color: iced::Color,
) -> Element<'a, Message> {
    let radius: Radius = radius.into();
    let border_color = theme_cfg.border_color();
    let hover_bg = theme_cfg.surface_hover_color();

    match cover {
        Some(handle) => container(
            image(handle.clone())
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .content_fit(ContentFit::Contain),
        )
        .width(size)
        .height(size)
        .center_x(size)
        .center_y(size)
        .clip(true)
        .style(move |_theme| container::Style {
            border: Border {
                radius,
                color: border_color,
                width: 1.0,
            },
            ..Default::default()
        })
        .into(),
        None => container(icons::icon(
            fallback_icon_svg,
            fallback_icon_size,
            fallback_icon_color,
        ))
        .width(size)
        .height(size)
        .center_x(size)
        .center_y(size)
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(hover_bg)),
            border: Border {
                radius,
                color: border_color,
                width: 1.0,
            },
            ..Default::default()
        })
        .into(),
    }
}

/// A card container with a primary colored title header.
pub fn settings_card<'a>(
    theme_cfg: &'a ThemeConfig,
    title: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(11).color(theme_cfg.primary_color()),
            content.into(),
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

/// A metric card displaying a prominent value and muted label.
pub fn metric_box<'a>(
    theme_cfg: &'a ThemeConfig,
    label: &'static str,
    value: String,
) -> Element<'a, Message> {
    container(
        column![
            text(value).size(16).color(theme_cfg.text_color()),
            text(label).size(11).color(theme_cfg.text_muted_color()),
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
