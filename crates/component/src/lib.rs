use std::borrow::Cow;

use iced::{
    border::Radius,
    widget::{button, column, container, image, row, rule, Space, Widget},
    Alignment, Border, Color, ContentFit, Element, Length,
};

use model::{SortDirection, ViewMode};
use theme::ThemeConfig;

pub mod catalog;
pub mod context_menu;
pub mod icons;
pub mod marquee;
pub mod settings;
pub mod spinner;
pub mod styles;

pub use crate::{
    catalog::{
        catalog_cover_art, CatalogRow, CatalogTile, TrackRow, CATALOG_LIST_ART_SIZE,
        CATALOG_LIST_ROW_HEIGHT, CATALOG_ROW_SPACING, CATALOG_TILE_META_HEIGHT, TRACK_ROW_HEIGHT,
    },
    context_menu::{
        render_context_menu, ContextMenuAction, ContextMenuState, ContextMenuSubmenu,
        ContextMenuTarget, TrackMenuContext,
    },
    marquee::Marquee,
    settings::{metric_box, settings_card, settings_danger_row, settings_header, settings_row},
    spinner::Spinner,
};

/// A text widget configured with the given font.
pub fn text<'a>(font: iced::Font, content: impl Into<Cow<'a, str>>) -> iced::widget::Text<'a> {
    iced::widget::text(content.into()).font(font)
}

pub fn border_vert<'a, M: 'a>(width: f32, color: Color) -> iced::Element<'a, M> {
    rule::horizontal(width)
        .style(styles::border_rule(color))
        .boxed()
}

pub fn border_hori<'a, M: 'a>(width: f32, color: Color) -> iced::Element<'a, M> {
    rule::vertical(width)
        .style(styles::border_rule(color))
        .boxed()
}

/// A reusable chip button with active state styling.
pub fn chip<'a, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    label: impl Into<String>,
    is_active: bool,
    on_press: M,
) -> Element<'a, M> {
    button(text(font, label.into()).size(11).color(if is_active {
        theme_cfg.primary
    } else {
        theme_cfg.text_muted
    }))
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .boxed()
}

/// A reusable chip button with an icon and label.
pub fn chip_with_icon<'a, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    icon_svg: &'static str,
    label: impl Into<String>,
    is_active: bool,
    on_press: M,
) -> Element<'a, M> {
    let color = if is_active {
        theme_cfg.primary
    } else {
        theme_cfg.text_muted
    };

    button(
        row![
            icons::icon(icon_svg, 13.0, color),
            Space::new().width(4),
            text(font, label.into()).size(11).color(color),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(styles::ghost_button(theme_cfg, is_active))
    .on_press(on_press)
    .boxed()
}

/// A chip button representing a view mode (List or Tiles).
fn view_mode_chip<'a, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    mode: ViewMode,
    is_active: bool,
    on_press: M,
) -> Element<'a, M> {
    let (icon_svg, label) = match mode {
        ViewMode::Tile => (icons::GRID_VIEW_SVG, "Tiles"),
        ViewMode::List => (icons::LIST_VIEW_SVG, "List"),
    };
    chip_with_icon(theme_cfg, font, icon_svg, label, is_active, on_press)
}

/// A unified view mode toggle row containing List and Tiles mode chips.
pub fn view_mode_toggle<'a, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    active_mode: ViewMode,
    on_change: impl Fn(ViewMode) -> M,
) -> Element<'a, M> {
    row![
        view_mode_chip(
            theme_cfg,
            font,
            ViewMode::List,
            active_mode == ViewMode::List,
            on_change(ViewMode::List),
        ),
        view_mode_chip(
            theme_cfg,
            font,
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
fn sort_chip<'a, T: Copy + PartialEq, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    label: &'a str,
    field: T,
    active_field: T,
    sort_dir: SortDirection,
    on_press: M,
) -> Element<'a, M> {
    let is_active = active_field == field;
    let text_color = if is_active {
        theme_cfg.primary
    } else {
        theme_cfg.text_muted
    };

    let content: Element<'a, M> = if is_active {
        let sort_icon_svg = match sort_dir {
            SortDirection::Ascending => icons::CHEVRON_UP_SVG,
            SortDirection::Descending => icons::CHEVRON_DOWN_SVG,
        };
        row![
            text(font, label).size(11).color(text_color),
            Space::new().width(3),
            icons::icon(sort_icon_svg, 11.0, text_color),
        ]
        .align_y(Alignment::Center)
        .boxed()
    } else {
        text(font, label).size(11).color(text_color).boxed()
    };

    button(content)
        .padding([3, 6])
        .style(styles::ghost_button(theme_cfg, is_active))
        .on_press(on_press)
        .boxed()
}

/// A unified sort chip list with a "Sort:" label and multiple sort chips.
pub fn sort_chip_list<'a, T: Copy + PartialEq, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    options: &[(&'a str, T)],
    active_field: T,
    sort_dir: SortDirection,
    on_press: impl Fn(T) -> M,
) -> Element<'a, M> {
    let mut sort_row = row![
        text(font, "Sort:").size(11).color(theme_cfg.text_muted),
        Space::new().width(4),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    for &(label, field) in options {
        sort_row = sort_row.push(sort_chip(
            theme_cfg,
            font,
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
pub fn empty_state<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    title: &'a str,
    subtitle: &'a str,
) -> Element<'a, M> {
    container(
        column![
            text(font, title).size(16).color(theme_cfg.text),
            text(font, subtitle).size(13).color(theme_cfg.text_muted),
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
pub fn cover_art<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    cover: Option<&image::Handle>,
    size: f32,
    radius: impl Into<Radius>,
    fallback_icon_svg: &'static str,
    fallback_icon_size: f32,
    fallback_icon_color: iced::Color,
) -> Element<'a, M> {
    let radius: Radius = radius.into();
    let border_color = theme_cfg.border;
    let placeholder_bg = theme_cfg.surface;

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

/// A self-contained scanning indicator component powered by the `Spinner` widget.
pub fn scanning_indicator<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    progress: Option<(usize, usize)>,
) -> Element<'a, M> {
    let handle = iced::widget::svg::Handle::from_memory(icons::REFRESH_SVG.as_bytes());
    let spinner = Spinner::new(handle).size(11.0).color(theme_cfg.text_muted);

    let label = match progress {
        Some((cur, tot)) if tot > 0 => format!("Scanning library... ({}/{})", cur, tot),
        _ => "Scanning library...".to_string(),
    };

    row![
        spinner,
        text(font, label)
            .size(11)
            .line_height(1.0)
            .color(theme_cfg.text_muted),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .boxed()
}

/// A rotating indicator shown while background album thumbnails are being cached to disk.
pub fn thumbnail_caching_indicator<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
) -> Element<'a, M> {
    let handle = iced::widget::svg::Handle::from_memory(icons::REFRESH_SVG.as_bytes());
    let spinner = Spinner::new(handle).size(11.0).color(theme_cfg.text_muted);

    row![
        spinner,
        text(font, "Caching album artwork...")
            .size(11)
            .line_height(1.0)
            .color(theme_cfg.text_muted),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use theme::ThemeConfig;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestMessage {
        CloseContextMenu,
    }

    #[test]
    fn test_adaptive_cjk_text_creation() {
        let font = iced::Font::DEFAULT;
        let _ascii = text(font, "The Beatles - Abbey Road");
        let _japanese = text(font, "宇多田ヒカル - First Love");
        let _chinese = text(font, "周杰伦 - 范特西");
        let _korean = text(font, "아이유 - Palette");
    }

    #[test]
    fn test_catalog_tile_renders_with_ellipsis() {
        let theme_cfg = ThemeConfig::default();
        let tile = CatalogTile {
            theme: &theme_cfg,
            font: iced::Font::DEFAULT,
            tile_width: 180.0,
            leading: iced::widget::Space::new().boxed(),
            title:
                "Very Long Album Title That Exceeds The Normal Card Boundaries And Needs Ellipsis"
                    .into(),
            mid_left:
                "Very Long Artist Name That Exceeds The Subtitle Row Width And Needs Ellipsis"
                    .into(),
            mid_right: "2024".into(),
            bottom_left: "12 tracks".into(),
            bottom_right: "45:00".into(),
            on_select: TestMessage::CloseContextMenu,
            on_play: TestMessage::CloseContextMenu,
            on_shuffle: TestMessage::CloseContextMenu,
            on_right_press: None,
        };
        let _rendered = tile.view();

        let row = CatalogRow {
            theme: &theme_cfg,
            font: iced::Font::DEFAULT,
            leading: iced::widget::Space::new().boxed(),
            title: "Very Long Album Title That Exceeds The Normal Row Width".into(),
            subtitle_left: "Very Long Artist Name".into(),
            subtitle_right: "2024".into(),
            details: Some(("12 tracks".into(), "45:00".into())),
            on_select: TestMessage::CloseContextMenu,
            on_play: TestMessage::CloseContextMenu,
            on_shuffle: TestMessage::CloseContextMenu,
            on_right_press: None,
            width: iced::Length::Fill,
            height: iced::Length::Fixed(56.0),
        };
        let _rendered_row = row.view();
    }

    #[test]
    fn test_catalog_cover_art_renders_placeholder_and_with_cover() {
        let theme_cfg = ThemeConfig::default();

        // 1. Placeholder fallback (None cover)
        let placeholder = catalog_cover_art(
            &theme_cfg,
            None,
            160.0,
            theme_cfg.border_radius.md,
            icons::DISC_SVG,
            50.0,
            theme_cfg.primary,
            TestMessage::CloseContextMenu,
        );
        let _rendered_placeholder = placeholder;

        // 2. Tile using catalog_cover_art
        let leading = catalog_cover_art(
            &theme_cfg,
            None,
            140.0,
            theme_cfg.border_radius.md,
            icons::USER_AVATAR_SVG,
            45.0,
            theme_cfg.primary,
            TestMessage::CloseContextMenu,
        );
        let tile = CatalogTile {
            theme: &theme_cfg,
            font: iced::Font::DEFAULT,
            tile_width: 140.0,
            leading,
            title: "Artist Name".into(),
            mid_left: "5 albums".into(),
            mid_right: "".into(),
            bottom_left: "".into(),
            bottom_right: "".into(),
            on_select: TestMessage::CloseContextMenu,
            on_play: TestMessage::CloseContextMenu,
            on_shuffle: TestMessage::CloseContextMenu,
            on_right_press: Some(TestMessage::CloseContextMenu),
        };
        let _rendered_tile = tile.view();
    }
}
