use iced::widget::{button, container, row};
use iced::{Alignment, Element, Length};

use crate::icons;
use crate::message::Message;
use crate::theme::{styles, ThemeConfig};

use super::text;

/// Height of a track row in the open-playlist and queue lists.
pub const TRACK_ROW_HEIGHT: f32 = 26.0;
/// Width of the leading track-number / drag-handle slot. Fixed in both
/// states so the number/handle swap never shifts layout or re-fires hover.
const SLOT_WIDTH: f32 = 36.0;
/// Width of the duration label column.
const DURATION_WIDTH: f32 = 64.0;

/// A single track row shared by the open-playlist detail list and the queue
/// detail list so both render exactly the same.
///
/// The component owns the full look — leading slot, typography, highlight,
/// and delete button. Callers supply row data, state flags, and their
/// context-specific messages (playlist vs. queue).
pub struct TrackRowProps<'a> {
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
    pub drag_start: Message,
    pub drag_release: Message,
    pub hover_enter: Message,
    pub hover_exit: Message,
    pub on_remove: Message,
}

/// Builds the shared track row element.
pub fn track_row(props: TrackRowProps<'_>) -> Element<'_, Message> {
    let TrackRowProps {
        theme: theme_cfg,
        index,
        title,
        artist,
        duration,
        highlighted,
        dragging,
        show_handle,
        on_press,
        drag_start,
        drag_release,
        hover_enter,
        hover_exit,
        on_remove,
    } = props;

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
        .width(SLOT_WIDTH)
        .height(TRACK_ROW_HEIGHT)
        .center_x(SLOT_WIDTH)
        .center_y(TRACK_ROW_HEIGHT)
        .into()
    } else {
        container(
            text(theme_cfg, format!("{}", index + 1))
                .size(12)
                .color(number_color),
        )
        .width(SLOT_WIDTH)
        .height(TRACK_ROW_HEIGHT)
        .center_x(SLOT_WIDTH)
        .center_y(TRACK_ROW_HEIGHT)
        .into()
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
            .width(Length::FillPortion(5)),
        text(theme_cfg, artist)
            .size(11)
            .color(theme_cfg.text_muted_color())
            .width(Length::FillPortion(4)),
        text(theme_cfg, duration)
            .size(12)
            .color(theme_cfg.text_muted_color())
            .width(DURATION_WIDTH),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .height(TRACK_ROW_HEIGHT);

    let row_container = container(row_content)
        .width(Length::Fill)
        .height(TRACK_ROW_HEIGHT);

    // Single click selects (arming double-click to play). Unavailable rows
    // stay unclickable for playback but remain draggable via the handle.
    let body: Element<'_, Message> = match on_press {
        Some(message) => iced::widget::mouse_area(row_container)
            .on_press(message)
            .on_enter(hover_enter)
            .on_exit(hover_exit)
            .interaction(iced::mouse::Interaction::Pointer)
            .into(),
        None => iced::widget::mouse_area(row_container)
            .on_enter(hover_enter)
            .on_exit(hover_exit)
            .into(),
    };

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
    .into()
}
