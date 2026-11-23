use std::borrow::Cow;

use iced::border::Radius;
use iced::widget::button::{Status as ButtonStatus, Style as ButtonStyle};
use iced::widget::text::{Ellipsis, Wrapping};
use iced::widget::{column, container, image, row, Space, Widget};
use iced::{Alignment, Background, Border, Color, ContentFit, Element, Length, Padding, Shadow};

use theme::ThemeConfig;

use crate::{button, icons, styles, text};

/// Fixed pixel height of a single catalog list row, shared by Albums and Artists.
pub const CATALOG_LIST_ROW_HEIGHT: f32 = 56.0;
/// Size of the leading artwork box in list rows.
pub const CATALOG_LIST_ART_SIZE: f32 = 42.0;
/// Fixed height of the metadata block under a grid tile.
pub const CATALOG_TILE_META_HEIGHT: f32 = 64.0;
/// Vertical gap between catalog rows (baked into the virtualized row height).
pub const CATALOG_ROW_SPACING: f32 = 14.0;
/// Width of the right-hand details column in list rows.
const DETAILS_WIDTH: f32 = 100.0;

/// A single catalog list row shared by the Albums and Artists list views so
/// both render exactly the same.
///
/// The component owns the full look — leading art, typography, details
/// column, and play/shuffle actions. Callers supply pre-rendered strings and
/// their context-specific messages (album vs. artist scope).
pub struct CatalogRow<'a, M> {
    pub theme: &'a ThemeConfig,
    pub font: font::Font,
    pub leading: Element<'a, M>,
    pub title: Cow<'a, str>,
    pub subtitle_left: Cow<'a, str>,
    pub subtitle_right: Cow<'a, str>,
    pub details: Option<(Cow<'a, str>, Cow<'a, str>)>,
    pub on_select: M,
    pub on_play: M,
    pub on_shuffle: M,
    pub on_right_press: Option<M>,
    pub width: Length,
    pub height: Length,
}

impl<'a, M: Clone + 'static> CatalogRow<'a, M> {
    pub fn view(self) -> Element<'a, M> {
        let Self {
            theme: theme_cfg,
            font,
            leading,
            title,
            subtitle_left,
            subtitle_right,
            details,
            on_select,
            on_play,
            on_shuffle,
            on_right_press,
            width,
            height,
        } = self;

        let mut subtitle = row![text(font, subtitle_left)
            .size(font.px(11.0))
            .color(theme_cfg.text_muted)];
        if !subtitle_right.is_empty() {
            subtitle = subtitle
                .push(
                    text(font, " • ")
                        .size(font.px(11.0))
                        .color(theme_cfg.text_muted)
                        .boxed(),
                )
                .push(
                    text(font, subtitle_right)
                        .size(font.px(11.0))
                        .color(theme_cfg.text_muted)
                        .boxed(),
                );
        }

        let title_block = column![
            text(font, title)
                .size(font.px(13.0))
                .color(theme_cfg.text)
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
            subtitle,
        ]
        .spacing(2)
        .width(Length::Fill);

        let mut row_content = row![leading, Space::new().width(12), title_block,].spacing(4);

        if let Some((top, bottom)) = details {
            let details_col = column![
                text(font, top)
                    .size(font.px(12.0))
                    .color(theme_cfg.text_muted),
                text(font, bottom)
                    .size(font.px(11.0))
                    .color(theme_cfg.text_muted),
            ]
            .spacing(2)
            .align_x(Alignment::End)
            .width(DETAILS_WIDTH);
            row_content = row_content.push(details_col.boxed());
        }

        let row_content = row_content
            .align_y(Alignment::Center)
            .height(Length::Fill)
            .padding([0, 8]);

        let select_btn = button(row_content)
            .style(styles::list_row(theme_cfg))
            .width(Length::Fill)
            .height(Length::Fill)
            .on_press(on_select);

        let select_element: Element<'a, M> = match on_right_press {
            Some(msg) => iced::widget::mouse_area(select_btn)
                .on_right_press(msg)
                .boxed(),
            None => select_btn.boxed(),
        };

        let play_btn = button(icons::icon(icons::PLAY_SVG, 14.0, theme_cfg.text_muted))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .on_press(on_play);

        let shuffle_btn = button(icons::icon(icons::SHUFFLE_SVG, 14.0, theme_cfg.text_muted))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .on_press(on_shuffle);

        let actions = row![play_btn, shuffle_btn]
            .spacing(4)
            .align_y(Alignment::Center);

        row![select_element, actions]
            .spacing(4)
            .align_y(Alignment::Center)
            .width(width)
            .height(height)
            .padding(Padding {
                right: 20.0,
                ..Default::default()
            })
            .boxed()
    }
}

/// A single catalog grid tile shared by the Albums and Artists tile views.
///
/// Mirrors the album tile look: square artwork on top, three-line metadata
/// block below, and play/shuffle overlay buttons anchored to the artwork.
/// Callers supply the leading artwork (keeping each view's fallback icon and
/// image cache) plus pre-rendered labels and context messages.
pub struct CatalogTile<'a, M> {
    pub theme: &'a ThemeConfig,
    pub font: font::Font,
    pub tile_width: f32,
    pub leading: Element<'a, M>,
    pub title: Cow<'a, str>,
    pub mid_left: Cow<'a, str>,
    pub mid_right: Cow<'a, str>,
    pub bottom_left: Cow<'a, str>,
    pub bottom_right: Cow<'a, str>,
    pub on_select: M,
    pub on_play: M,
    pub on_shuffle: M,
    pub on_right_press: Option<M>,
}

impl<'a, M: Clone + 'static> CatalogTile<'a, M> {
    pub fn view(self) -> Element<'a, M> {
        let Self {
            theme: theme_cfg,
            font,
            tile_width: tile_w,
            leading,
            title,
            mid_left,
            mid_right,
            bottom_left,
            bottom_right,
            on_select,
            on_play,
            on_shuffle,
            on_right_press,
        } = self;

        let meta = container(
            column![
                text(font, title)
                    .size(font.px(13.0))
                    .width(Length::Fill)
                    .wrapping(Wrapping::None)
                    .ellipsis(Ellipsis::End),
                row![
                    text(font, mid_left)
                        .size(font.px(11.0))
                        .color(theme_cfg.text_muted)
                        .width(Length::Fill)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End),
                    text(font, mid_right)
                        .size(font.px(11.0))
                        .color(theme_cfg.text_muted),
                ]
                .align_y(Alignment::Center),
                if bottom_right.is_empty() {
                    row![text(font, bottom_left)
                        .size(font.px(10.0))
                        .color(theme_cfg.text_muted)
                        .width(Length::Fill)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End),]
                    .align_y(Alignment::Center)
                } else {
                    row![
                        text(font, bottom_left)
                            .size(font.px(10.0))
                            .color(theme_cfg.text_muted),
                        Space::new().width(Length::Fill),
                        text(font, bottom_right)
                            .size(font.px(10.0))
                            .color(theme_cfg.text_muted),
                    ]
                    .align_y(Alignment::Center)
                },
            ]
            .spacing(3)
            .padding(8),
        )
        .width(tile_w)
        .height(CATALOG_TILE_META_HEIGHT);

        let meta_btn = button(meta)
            .style(styles::tile(theme_cfg))
            .padding(0)
            .on_press(on_select);

        let play_btn = button(icons::icon(icons::PLAY_SVG, 14.0, theme_cfg.text))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .interaction(iced::mouse::Interaction::Pointer)
            .on_press(on_play);

        let shuffle_btn = button(icons::icon(icons::SHUFFLE_SVG, 14.0, theme_cfg.text))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .interaction(iced::mouse::Interaction::Pointer)
            .on_press(on_shuffle);

        let actions = container(row![play_btn, shuffle_btn].spacing(4))
            .width(tile_w)
            .height(tile_w)
            .align_x(Alignment::End)
            .align_y(Alignment::End)
            .padding(6);

        let cover_block = iced::widget::stack([leading, actions.boxed()]);

        let card_content = column![cover_block, meta_btn]
            .width(tile_w)
            .height(tile_w + CATALOG_TILE_META_HEIGHT);

        match on_right_press {
            Some(msg) => iced::widget::mouse_area(card_content)
                .on_right_press(msg)
                .boxed(),
            None => card_content.boxed(),
        }
    }
}

/// Height of a track row in the open-playlist and queue lists.
pub const TRACK_ROW_HEIGHT: f32 = 26.0;
/// Width of the leading track-number / drag-handle slot. Fixed in both
/// states so the number/handle swap never shifts layout or re-fires hover.
const TRACK_ROW_SLOT_WIDTH: f32 = 36.0;
/// Width of the duration label column.
const TRACK_ROW_DURATION_WIDTH: f32 = 64.0;

/// A single track row shared by the open-playlist detail list and the queue
/// detail list so both render exactly the same.
///
/// The component owns the full look — leading slot, typography, highlight,
/// and delete button. Callers supply row data, state flags, and their
/// context-specific messages (playlist vs. queue).
fn format_index(index: usize) -> Cow<'static, str> {
    const PRECOMPUTED_MAX: usize = 1024;
    static NUM_CACHE: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let cache = NUM_CACHE.get_or_init(|| (1..=PRECOMPUTED_MAX).map(|n| n.to_string()).collect());
    if index < PRECOMPUTED_MAX {
        Cow::Borrowed(cache[index].as_str())
    } else {
        Cow::Owned((index + 1).to_string())
    }
}

pub struct TrackRow<'a, M> {
    pub theme: &'a ThemeConfig,
    pub font: font::Font,
    /// Zero-based position; rendered as `index + 1` in the leading slot.
    pub index: usize,
    pub title: Cow<'a, str>,
    pub artist: Cow<'a, str>,
    pub duration: Cow<'a, str>,
    /// Selected (playlist) or selected-or-playing (queue) row: highlighted
    /// background with primary-colored title and track number.
    pub highlighted: bool,
    pub dragging: bool,
    pub show_handle: bool,
    /// Click action for the row body. `None` marks an unmatched playlist
    /// entry: muted title with no click action (still draggable and
    /// removable).
    pub on_press: Option<M>,
    pub on_right_press: Option<M>,
    pub drag_start: M,
    pub drag_release: M,
    pub hover_enter: M,
    pub hover_exit: M,
    pub on_remove: M,
}

impl<'a, M: Clone + 'static> TrackRow<'a, M> {
    /// Builds the shared track row element.
    pub fn view(self) -> Element<'a, M> {
        let Self {
            theme: theme_cfg,
            font,
            index,
            title,
            artist,
            duration,
            highlighted,
            dragging,
            show_handle,
            on_press,
            on_right_press,
            drag_start,
            drag_release,
            hover_enter,
            hover_exit,
            on_remove,
        } = self;

        let number_color = if highlighted {
            theme_cfg.primary
        } else {
            theme_cfg.text_muted
        };
        let title_color = match &on_press {
            None => theme_cfg.text_muted,
            Some(_) if highlighted => theme_cfg.primary,
            Some(_) => theme_cfg.text,
        };

        let slot_content: Element<'_, M> = if show_handle {
            container(icons::icon(icons::QUEUE_SVG, 12.0, theme_cfg.text_muted))
                .width(TRACK_ROW_SLOT_WIDTH)
                .height(TRACK_ROW_HEIGHT)
                .center_x(TRACK_ROW_SLOT_WIDTH)
                .center_y(TRACK_ROW_HEIGHT)
                .boxed()
        } else {
            container(
                text(font, format_index(index))
                    .size(font.px(12.0))
                    .color(number_color),
            )
            .width(TRACK_ROW_SLOT_WIDTH)
            .height(TRACK_ROW_HEIGHT)
            .center_x(TRACK_ROW_SLOT_WIDTH)
            .center_y(TRACK_ROW_HEIGHT)
            .boxed()
        };
        let mut slot = iced::widget::mouse_area(slot_content)
            .on_enter(hover_enter.clone())
            .on_exit(hover_exit.clone());
        if show_handle {
            // Only the revealed handle starts a drag (never the bare number,
            // which also keeps touch scrolling safe).
            slot = slot
                .on_press(drag_start)
                .on_release(drag_release)
                .interaction(iced::mouse::Interaction::Grab);
        }

        let row_content = row![
            text(font, title)
                .size(font.px(13.0))
                .color(title_color)
                .width(Length::FillPortion(5))
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
            text(font, artist)
                .size(font.px(11.0))
                .color(theme_cfg.text_muted)
                .width(Length::FillPortion(4))
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
            text(font, duration)
                .size(font.px(12.0))
                .color(theme_cfg.text_muted)
                .width(TRACK_ROW_DURATION_WIDTH),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .height(TRACK_ROW_HEIGHT);

        let row_container = container(row_content)
            .width(Length::Fill)
            .height(TRACK_ROW_HEIGHT);

        let mut area = iced::widget::mouse_area(row_container)
            .on_enter(hover_enter)
            .on_exit(hover_exit);
        if let Some(message) = on_press {
            area = area.on_press(message);
        }
        if let Some(message) = on_right_press {
            area = area.on_right_press(message);
        }
        let body: Element<'_, M> = area.boxed();

        let remove_btn = button(icons::icon(icons::TRASH_SVG, 12.0, theme_cfg.text_muted))
            .style(styles::icon_control_button(theme_cfg))
            .padding([3, 5])
            .on_press(on_remove);

        container(
            row![slot, body, remove_btn]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 20.0,
                    bottom: 0.0,
                    left: 4.0,
                }),
        )
        .style(styles::queue_row_container(
            theme_cfg,
            highlighted,
            dragging,
        ))
        .width(Length::Fill)
        .height(TRACK_ROW_HEIGHT)
        .boxed()
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
pub fn catalog_cover_art<'a, M: Clone + 'static>(
    theme_cfg: &'a ThemeConfig,
    cover: Option<&image::Handle>,
    size: f32,
    radius: impl Into<Radius>,
    fallback_icon_svg: &'static str,
    fallback_icon_size: f32,
    fallback_icon_color: iced::Color,
    on_press: M,
) -> Element<'a, M> {
    let radius: Radius = radius.into();
    let border_color = theme_cfg.border;

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
                .style(move |_theme, status| {
                    let bg = match status {
                        ButtonStatus::Active => Color::TRANSPARENT,
                        ButtonStatus::Hovered => Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                        ButtonStatus::Pressed => Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                        ButtonStatus::Disabled => Color::TRANSPARENT,
                    };
                    ButtonStyle {
                        background: Some(Background::Color(bg)),
                        border: Border {
                            radius,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(on_press);

            iced::widget::stack([image_layer.boxed(), overlay_btn.boxed()]).boxed()
        }
        None => {
            let placeholder_bg = theme_cfg.surface;
            let placeholder_hover_bg = theme_cfg.surface_hover;

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
                        ButtonStatus::Active => placeholder_bg,
                        ButtonStatus::Hovered => placeholder_hover_bg,
                        ButtonStatus::Pressed => placeholder_hover_bg,
                        ButtonStatus::Disabled => placeholder_bg,
                    };
                    ButtonStyle {
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
