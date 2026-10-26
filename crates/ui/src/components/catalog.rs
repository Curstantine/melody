use iced::widget::text::{Ellipsis, Wrapping};
use iced::widget::{button, column, container, row, Space, Widget};
use iced::{Alignment, Element, Length, Padding};

use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

use super::text;

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
pub struct CatalogRow<'a> {
    pub theme: &'a ThemeConfig,
    /// Leading artwork element (caller-built via `components::cover_art` so
    /// each view keeps its own fallback icon and image cache).
    pub leading: Element<'a, Message>,
    pub title: &'a str,
    /// Split subtitle so callers pass borrowed labels without allocating a
    /// combined string: rendered as `left`, or `left • right` when `right` is
    /// non-empty.
    pub subtitle_left: &'a str,
    pub subtitle_right: &'a str,
    /// Optional right-hand `(top, bottom)` details. `None` renders no details
    /// column (artists); `Some` renders tracks/duration (albums).
    pub details: Option<(&'a str, &'a str)>,
    pub on_select: Message,
    pub on_play: Message,
    pub on_shuffle: Message,
    pub on_right_press: Option<Message>,
    pub width: Length,
    pub height: Length,
}

impl<'a> CatalogRow<'a> {
    /// Builds the shared catalog list row element with the caller-provided
    /// outer sizing, so callers don't wrap it for virtualization.
    pub fn view(self) -> Element<'a, Message> {
        let Self {
            theme: theme_cfg,
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

        let mut subtitle = row![text(theme_cfg, subtitle_left)
            .size(11)
            .color(theme_cfg.text_muted_color())];
        if !subtitle_right.is_empty() {
            subtitle = subtitle
                .push(
                    text(theme_cfg, " • ")
                        .size(11)
                        .color(theme_cfg.text_muted_color())
                        .boxed(),
                )
                .push(
                    text(theme_cfg, subtitle_right)
                        .size(11)
                        .color(theme_cfg.text_muted_color())
                        .boxed(),
                );
        }

        let title_block = column![
            text(theme_cfg, title)
                .size(13)
                .color(theme_cfg.text_color())
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
            subtitle,
        ]
        .spacing(2)
        .width(Length::Fill);

        let mut row_content = row![leading, Space::new().width(12), title_block,].spacing(4);

        if let Some((top, bottom)) = details {
            let details_col = column![
                text(theme_cfg, top)
                    .size(12)
                    .color(theme_cfg.text_muted_color()),
                text(theme_cfg, bottom)
                    .size(11)
                    .color(theme_cfg.text_muted_color()),
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

        let select_element: Element<'a, Message> = match on_right_press {
            Some(msg) => iced::widget::mouse_area(select_btn)
                .on_right_press(msg)
                .boxed(),
            None => select_btn.boxed(),
        };

        let play_btn = button(icons::icon(
            icons::PLAY_SVG,
            14.0,
            theme_cfg.text_muted_color(),
        ))
        .style(styles::icon_control_button(theme_cfg))
        .padding(4)
        .on_press(on_play);

        let shuffle_btn = button(icons::icon(
            icons::SHUFFLE_SVG,
            14.0,
            theme_cfg.text_muted_color(),
        ))
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
pub struct CatalogTile<'a> {
    pub theme: &'a ThemeConfig,
    pub tile_width: f32,
    pub leading: Element<'a, Message>,
    pub title: &'a str,
    pub mid_left: &'a str,
    pub mid_right: &'a str,
    pub bottom_left: &'a str,
    pub bottom_right: &'a str,
    pub on_select: Message,
    pub on_play: Message,
    pub on_shuffle: Message,
    pub on_right_press: Option<Message>,
}

impl<'a> CatalogTile<'a> {
    /// Builds the shared catalog tile element.
    pub fn view(self) -> Element<'a, Message> {
        let Self {
            theme: theme_cfg,
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
                // Single-line title: width-bounded with no wrapping and ellipsis
                // so long Latin and CJK titles truncate with an ellipsis at the
                // tile edge instead of wrapping or hard clipping.
                text(theme_cfg, title)
                    .size(13)
                    .width(Length::Fill)
                    .wrapping(Wrapping::None)
                    .ellipsis(Ellipsis::End),
                row![
                    text(theme_cfg, mid_left)
                        .size(11)
                        .color(theme_cfg.text_muted_color())
                        .width(Length::Fill)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End),
                    text(theme_cfg, mid_right)
                        .size(11)
                        .color(theme_cfg.text_muted_color()),
                ]
                .align_y(Alignment::Center),
                if bottom_right.is_empty() {
                    row![text(theme_cfg, bottom_left)
                        .size(10)
                        .color(theme_cfg.text_muted_color())
                        .width(Length::Fill)
                        .wrapping(Wrapping::None)
                        .ellipsis(Ellipsis::End),]
                    .align_y(Alignment::Center)
                } else {
                    row![
                        text(theme_cfg, bottom_left)
                            .size(10)
                            .color(theme_cfg.text_muted_color()),
                        Space::new().width(Length::Fill),
                        text(theme_cfg, bottom_right)
                            .size(10)
                            .color(theme_cfg.text_muted_color()),
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

        let play_btn = button(icons::icon(icons::PLAY_SVG, 14.0, theme_cfg.text_color()))
            .style(styles::icon_control_button(theme_cfg))
            .padding(4)
            .on_press(on_play);

        let shuffle_btn = button(icons::icon(
            icons::SHUFFLE_SVG,
            14.0,
            theme_cfg.text_color(),
        ))
        .style(styles::icon_control_button(theme_cfg))
        .padding(4)
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
pub struct TrackRow<'a> {
    pub theme: &'a ThemeConfig,
    /// Zero-based position; rendered as `index + 1` in the leading slot.
    pub index: usize,
    pub title: &'a str,
    pub artist: &'a str,
    pub duration: &'a str,
    /// Selected (playlist) or selected-or-playing (queue) row: highlighted
    /// background with primary-colored title and track number.
    pub highlighted: bool,
    pub dragging: bool,
    /// Reveals the drag handle in place of the track number.
    pub show_handle: bool,
    /// Click action for the row body. `None` marks an unmatched playlist
    /// entry: muted title with no click action (still draggable and
    /// removable).
    pub on_press: Option<Message>,
    pub on_right_press: Option<Message>,
    pub drag_start: Message,
    pub drag_release: Message,
    pub hover_enter: Message,
    pub hover_exit: Message,
    pub on_remove: Message,
}

impl<'a> TrackRow<'a> {
    /// Builds the shared track row element.
    pub fn view(self) -> Element<'a, Message> {
        let Self {
            theme: theme_cfg,
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
            theme_cfg.primary_color()
        } else {
            theme_cfg.text_muted_color()
        };
        let title_color = match &on_press {
            None => theme_cfg.text_muted_color(),
            Some(_) if highlighted => theme_cfg.primary_color(),
            Some(_) => theme_cfg.text_color(),
        };

        let slot_content: Element<'_, Message> = if show_handle {
            container(icons::icon(
                icons::QUEUE_SVG,
                12.0,
                theme_cfg.text_muted_color(),
            ))
            .width(TRACK_ROW_SLOT_WIDTH)
            .height(TRACK_ROW_HEIGHT)
            .center_x(TRACK_ROW_SLOT_WIDTH)
            .center_y(TRACK_ROW_HEIGHT)
            .boxed()
        } else {
            container(
                text(theme_cfg, format!("{}", index + 1))
                    .size(12)
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
            text(theme_cfg, title)
                .size(13)
                .color(title_color)
                .width(Length::FillPortion(5))
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
            text(theme_cfg, artist)
                .size(11)
                .color(theme_cfg.text_muted_color())
                .width(Length::FillPortion(4))
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
            text(theme_cfg, duration)
                .size(12)
                .color(theme_cfg.text_muted_color())
                .width(TRACK_ROW_DURATION_WIDTH),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .height(TRACK_ROW_HEIGHT);

        let row_container = container(row_content)
            .width(Length::Fill)
            .height(TRACK_ROW_HEIGHT);

        // Single click selects (arming double-click to play). Unavailable rows
        // stay unclickable for playback but remain draggable via the handle.
        let mut area = iced::widget::mouse_area(row_container)
            .on_enter(hover_enter)
            .on_exit(hover_exit);
        if let Some(message) = on_press {
            area = area
                .on_press(message)
                .interaction(iced::mouse::Interaction::Pointer);
        }
        if let Some(message) = on_right_press {
            area = area.on_right_press(message);
        }
        let body: Element<'_, Message> = area.boxed();

        let remove_btn = button(icons::icon(
            icons::TRASH_SVG,
            12.0,
            theme_cfg.text_muted_color(),
        ))
        .style(styles::icon_control_button(theme_cfg))
        .padding([3, 5])
        .on_press(on_remove);

        // The highlight wraps the whole row so the background spans the handle
        // slot, the content, and the delete button.
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
