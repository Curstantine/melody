use std::borrow::Cow;

use iced::{
    widget::{column, container, row, Space, Widget},
    Alignment, Border, Color, Element, Length,
};

use theme::ThemeConfig;

use crate::{border_vert, text};

/// A section header for settings pages with an optional muted description.
pub fn settings_header<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    title: &'static str,
    description: Option<&'static str>,
) -> Element<'a, M> {
    let mut header = column![text(font, title).size(20).color(theme_cfg.text),];

    if let Some(x) = description {
        header = header.push(text(font, x).size(12).color(theme_cfg.text_muted).boxed());
    }

    header.spacing(4).boxed()
}

/// A card container with a primary colored title header.
pub fn settings_card<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    title: impl Into<Cow<'a, str>>,
    content: Element<'a, M>,
) -> Element<'a, M> {
    column![
        text(font, title).size(11).color(theme_cfg.primary),
        Space::new().height(4),
        border_vert(1.0, theme_cfg.border),
        Space::new().height(12),
        content
    ]
    .boxed()
}

/// A metric card displaying a prominent value and muted label.
pub fn metric_box<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    label: &'static str,
    value: String,
) -> Element<'a, M> {
    container(
        column![
            text(font, value).size(16).color(theme_cfg.text),
            text(font, label).size(11).color(theme_cfg.text_muted),
        ]
        .spacing(2)
        .padding([8, 12]),
    )
    .style(move |_theme| container::Style {
        background: Some(iced::Background::Color(theme_cfg.surface)),
        border: Border {
            radius: theme_cfg.border_radius.sm,
            color: theme_cfg.border,
            width: 1.0,
        },
        ..Default::default()
    })
    .width(Length::FillPortion(1))
    .boxed()
}

fn settings_row_with_color<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    title: impl Into<Cow<'a, str>>,
    title_color: Color,
    subtitle: impl Into<Cow<'a, str>>,
    control: impl Widget<M, iced::Theme, iced::Renderer> + 'a,
) -> Element<'a, M> {
    row![
        column![
            text(font, title).size(12).color(title_color),
            text(font, subtitle).size(11).color(theme_cfg.text_muted),
        ]
        .spacing(4)
        .width(Length::Fill),
        control.boxed(),
    ]
    .spacing(1)
    .align_y(Alignment::Center)
    .boxed()
}

/// A settings row with a title and subtitle on the left and a control widget on the right.
pub fn settings_row<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    title: impl Into<Cow<'a, str>>,
    subtitle: impl Into<Cow<'a, str>>,
    control: impl Widget<M, iced::Theme, iced::Renderer> + 'a,
) -> Element<'a, M> {
    settings_row_with_color(
        theme_cfg,
        font,
        title,
        theme_cfg.text_muted,
        subtitle,
        control,
    )
}

/// A destructive settings row whose title uses the theme's danger color.
pub fn settings_danger_row<'a, M: 'a>(
    theme_cfg: &'a ThemeConfig,
    font: iced::Font,
    title: impl Into<Cow<'a, str>>,
    subtitle: impl Into<Cow<'a, str>>,
    control: impl Widget<M, iced::Theme, iced::Renderer> + 'a,
) -> Element<'a, M> {
    settings_row_with_color(theme_cfg, font, title, theme_cfg.danger, subtitle, control)
}
