use core_engine::SortDirection;
use iced::border::Radius;
use iced::widget::{button, column, container, image, row, Space, Widget};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length, Shadow};

use crate::icons;
use crate::message::{Message, ViewMode};
use crate::theme::{styles, ThemeConfig};

use std::borrow::Cow;

pub mod catalog;
pub mod context_menu;
pub mod marquee;
pub mod spinner;
#[allow(unused_imports)]
pub use catalog::{
    CatalogRow, CatalogTile, TrackRow, CATALOG_LIST_ART_SIZE, CATALOG_LIST_ROW_HEIGHT,
    CATALOG_ROW_SPACING, CATALOG_TILE_META_HEIGHT, TRACK_ROW_HEIGHT,
};
#[allow(unused_imports)]
pub use context_menu::{
    render_context_menu, ContextMenuState, ContextMenuSubmenu, ContextMenuTarget, TrackMenuContext,
};
#[allow(unused_imports)]
pub use marquee::Marquee;
pub use spinner::Spinner;

/// A text widget configured with adaptive text shaping:
/// - Uses `Shaping::Basic` for pure ASCII text for zero-cost immediate shaping and high frame rates.
/// - Uses `Shaping::Advanced` for Chinese, Japanese, Korean, and multilingual Unicode characters
///   to ensure cross-script font fallback without tofu or missing glyphs.
pub fn text<'a>(
    theme_cfg: &ThemeConfig,
    content: impl Into<Cow<'a, str>>,
) -> iced::widget::Text<'a> {
    iced::widget::text(content.into()).font(theme_cfg.font())
}

pub fn border_vert<'a>(width: f32, color: Color) -> iced::Element<'a, Message> {
    container("")
        .width(Length::Fill)
        .height(width)
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        })
        .boxed()
}

pub fn border_hori<'a>(width: f32, color: Color) -> iced::Element<'a, Message> {
    container("")
        .width(width)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        })
        .boxed()
}

/// A reusable chip button with active state styling.
pub fn chip<'a>(
    theme_cfg: &'a ThemeConfig,
    label: impl Into<String>,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(theme_cfg, label.into()).size(11).color(if is_active {
        theme_cfg.primary_color()
    } else {
        theme_cfg.text_muted_color()
    }))
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .boxed()
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
            Space::new().width(4),
            text(theme_cfg, label.into()).size(11).color(color),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .boxed()
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
    .boxed()
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
    let text_color = if is_active {
        theme_cfg.primary_color()
    } else {
        theme_cfg.text_muted_color()
    };

    let content: Element<'a, Message> = if is_active {
        let sort_icon_svg = match sort_dir {
            SortDirection::Ascending => icons::CHEVRON_UP_SVG,
            SortDirection::Descending => icons::CHEVRON_DOWN_SVG,
        };
        row![
            text(theme_cfg, label).size(11).color(text_color),
            Space::new().width(3),
            icons::icon(sort_icon_svg, 11.0, text_color),
        ]
        .align_y(Alignment::Center)
        .boxed()
    } else {
        text(theme_cfg, label).size(11).color(text_color).boxed()
    };

    button(content)
        .padding([3, 6])
        .style(styles::ghost_button(theme_cfg, is_active))
        .on_press(on_press)
        .boxed()
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
        text(theme_cfg, "Sort:")
            .size(11)
            .color(theme_cfg.text_muted_color()),
        Space::new().width(4),
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

    sort_row.boxed()
}

/// A centered empty state view with a title and muted description.
pub fn empty_state<'a>(
    theme_cfg: &'a ThemeConfig,
    title: &'a str,
    subtitle: &'a str,
) -> Element<'a, Message> {
    container(
        column![
            text(theme_cfg, title)
                .size(16)
                .color(theme_cfg.text_color()),
            text(theme_cfg, subtitle)
                .size(13)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(8)
        .align_x(Alignment::Center),
    )
    .padding(40)
    .width(Length::Fill)
    .align_x(Alignment::Center)
    .boxed()
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
    let placeholder_bg = theme_cfg.surface_color();

    match cover {
        Some(handle) => container(
            image(handle.clone())
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .content_fit(ContentFit::Cover)
                .border_radius(radius),
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
        .boxed(),
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
            background: Some(iced::Background::Color(placeholder_bg)),
            border: Border {
                radius,
                color: border_color,
                width: 1.0,
            },
            ..Default::default()
        })
        .boxed(),
    }
}

/// An interactive catalog cover displaying artwork or a placeholder icon.
///
/// On hover:
/// - If artwork image is present: darkens the cover with a semi-transparent overlay.
/// - If fallback placeholder: transitions the background from `surface` to `surface_hover`.
///
/// Clicks trigger the provided `on_press` message.
#[allow(clippy::too_many_arguments)]
pub fn catalog_cover_art<'a>(
    theme_cfg: &'a ThemeConfig,
    cover: Option<&image::Handle>,
    size: f32,
    radius: impl Into<Radius>,
    fallback_icon_svg: &'static str,
    fallback_icon_size: f32,
    fallback_icon_color: iced::Color,
    on_press: Message,
) -> Element<'a, Message> {
    let radius: Radius = radius.into();
    let border_color = theme_cfg.border_color();

    match cover {
        Some(handle) => {
            let image_layer = container(
                image(handle.clone())
                    .width(Length::Fixed(size))
                    .height(Length::Fixed(size))
                    .content_fit(ContentFit::Cover)
                    .border_radius(radius),
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
            });

            let overlay_btn = button(Space::new().width(Length::Fill).height(Length::Fill))
                .width(size)
                .height(size)
                .padding(0)
                .style(move |_theme, status| {
                    let bg = match status {
                        button::Status::Active => Color::TRANSPARENT,
                        button::Status::Hovered => Color::from_rgba(0.0, 0.0, 0.0, 0.42),
                        button::Status::Pressed => Color::from_rgba(0.0, 0.0, 0.0, 0.42),
                        button::Status::Disabled => Color::TRANSPARENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border {
                            radius,
                            color: border_color,
                            width: 1.0,
                        },
                        shadow: Shadow::default(),
                        snap: true,
                        text_color: Color::TRANSPARENT,
                    }
                })
                .on_press(on_press);

            iced::widget::stack([image_layer.boxed(), overlay_btn.boxed()]).boxed()
        }
        None => {
            let placeholder_bg = theme_cfg.surface_color();
            let placeholder_hover_bg = theme_cfg.surface_hover_color();

            let placeholder_content = container(icons::icon(
                fallback_icon_svg,
                fallback_icon_size,
                fallback_icon_color,
            ))
            .width(size)
            .height(size)
            .center_x(size)
            .center_y(size);

            button(placeholder_content)
                .width(size)
                .height(size)
                .padding(0)
                .style(move |_theme, status| {
                    let bg = match status {
                        button::Status::Active => placeholder_bg,
                        button::Status::Hovered => placeholder_hover_bg,
                        button::Status::Pressed => placeholder_hover_bg,
                        button::Status::Disabled => placeholder_bg,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        border: Border {
                            radius,
                            color: border_color,
                            width: 1.0,
                        },
                        shadow: Shadow::default(),
                        snap: true,
                        text_color: Color::TRANSPARENT,
                    }
                })
                .on_press(on_press)
                .boxed()
        }
    }
}

pub fn settings_header<'a>(
    theme_cfg: &'a ThemeConfig,
    title: &'static str,
    description: Option<&'static str>,
) -> Element<'a, Message> {
    let mut header = column![text(theme_cfg, title)
        .size(20)
        .color(theme_cfg.text_color()),];

    if let Some(x) = description {
        header = header.push(
            text(theme_cfg, x)
                .size(12)
                .color(theme_cfg.text_muted_color())
                .boxed(),
        );
    }

    header.spacing(4).boxed()
}

/// A card container with a primary colored title header.
pub fn settings_card<'a>(
    theme_cfg: &'a ThemeConfig,
    title: &'static str,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        text(theme_cfg, title)
            .size(11)
            .color(theme_cfg.primary_color()),
        Space::new().height(4),
        border_vert(1.0, theme_cfg.border_color()),
        Space::new().height(12),
        content
    ]
    .boxed()
}

/// A metric card displaying a prominent value and muted label.
pub fn metric_box<'a>(
    theme_cfg: &'a ThemeConfig,
    label: &'static str,
    value: String,
) -> Element<'a, Message> {
    container(
        column![
            text(theme_cfg, value)
                .size(16)
                .color(theme_cfg.text_color()),
            text(theme_cfg, label)
                .size(11)
                .color(theme_cfg.text_muted_color()),
        ]
        .spacing(2)
        .padding([8, 12]),
    )
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(theme_cfg.surface_color())),
        border: Border {
            radius: theme_cfg.radius_sm(),
            color: theme_cfg.border_color(),
            width: 1.0,
        },
        ..Default::default()
    })
    .width(Length::FillPortion(1))
    .boxed()
}

/// A self-contained scanning indicator component powered by the `Spinner` widget.
pub fn scanning_indicator<'a>(
    theme_cfg: &'a ThemeConfig,
    progress: Option<(usize, usize)>,
) -> Element<'a, Message> {
    let handle = iced::widget::svg::Handle::from_memory(icons::REFRESH_SVG.as_bytes());
    let spinner = Spinner::new(handle)
        .size(11.0)
        .color(theme_cfg.text_muted_color());

    let label = match progress {
        Some((cur, tot)) if tot > 0 => format!("Scanning library... ({}/{})", cur, tot),
        _ => "Scanning library...".to_string(),
    };

    row![
        spinner,
        text(theme_cfg, label)
            .size(11)
            .line_height(1.0)
            .color(theme_cfg.text_muted_color()),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .boxed()
}

/// A rotating indicator shown while background album thumbnails are being cached to disk.
pub fn thumbnail_caching_indicator<'a>(theme_cfg: &'a ThemeConfig) -> Element<'a, Message> {
    let handle = iced::widget::svg::Handle::from_memory(icons::REFRESH_SVG.as_bytes());
    let spinner = Spinner::new(handle)
        .size(11.0)
        .color(theme_cfg.text_muted_color());

    row![
        spinner,
        text(theme_cfg, "Caching album artwork...")
            .size(11)
            .line_height(1.0)
            .color(theme_cfg.text_muted_color()),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .boxed()
}
