use iced::mouse::Interaction;
use iced::widget::{button, column, container, mouse_area, row, Space, Widget};
use iced::window::Direction;
use iced::{Alignment, Element, Length};

use crate::message::Message;
use component::{icons, styles, text};
use theme::ThemeConfig;

/// Height of the standalone custom window titlebar in logical pixels
/// (the merged navbar titlebar in `layout_view.rs` uses 32 px).
const STANDALONE_TITLEBAR_HEIGHT: f32 = 30.0;

/// Thickness of the invisible edge resize grab strips in logical pixels.
const RESIZE_EDGE_PX: f32 = 5.0;

/// Size of the invisible corner resize grab targets in logical pixels.
const RESIZE_CORNER_PX: f32 = 8.0;

pub struct TitlebarView;

impl TitlebarView {
    /// Renders the custom themed window titlebar with drag, double-click maximize,
    /// system menu, and window control buttons.
    pub fn view<'a>(
        theme_cfg: &'a ThemeConfig,
        font: iced::Font,
        title: String,
        is_maximized: bool,
        is_focused: bool,
    ) -> Element<'a, Message> {
        let drag_region = mouse_area(
            container(
                text(font, title)
                    .size(11)
                    .line_height(1.0)
                    .color(theme_cfg.text_muted),
            )
            .padding([0, 12])
            .width(Length::Fill)
            .height(Length::Fixed(STANDALONE_TITLEBAR_HEIGHT))
            .align_y(Alignment::Center),
        )
        .on_press(Message::TitlebarPressed)
        .on_right_press(Message::WindowShowSystemMenu);

        let controls = window_controls_row(theme_cfg, is_maximized, is_focused);

        let bar = row![drag_region, controls]
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fixed(STANDALONE_TITLEBAR_HEIGHT));

        container(bar)
            .style(styles::titlebar_container(theme_cfg))
            .width(Length::Fill)
            .height(Length::Fixed(STANDALONE_TITLEBAR_HEIGHT))
            .boxed()
    }
}

/// Renders the Minimize, Maximize/Restore, and Close window control buttons.
pub fn window_controls_row<'a>(
    theme_cfg: &'a ThemeConfig,
    is_maximized: bool,
    is_focused: bool,
) -> Element<'a, Message> {
    let min_btn = window_control_btn(
        icons::WINDOW_MINIMIZE_SVG,
        #[cfg(windows)]
        "\u{E921}",
        theme_cfg,
        false,
        is_focused,
        None,
        Message::WindowMinimize,
    );

    #[cfg(target_os = "windows")]
    let max_override = crate::window_decorations::snap_layouts::maximize_button_status();
    #[cfg(not(target_os = "windows"))]
    let max_override = None;

    let max_btn = window_control_btn(
        if is_maximized {
            icons::WINDOW_RESTORE_SVG
        } else {
            icons::WINDOW_MAXIMIZE_SVG
        },
        #[cfg(windows)]
        if is_maximized { "\u{E923}" } else { "\u{E922}" },
        theme_cfg,
        false,
        is_focused,
        max_override,
        Message::WindowToggleMaximize,
    );

    let close_btn = window_control_btn(
        icons::WINDOW_CLOSE_SVG,
        #[cfg(windows)]
        "\u{E8BB}",
        theme_cfg,
        true,
        is_focused,
        None,
        Message::WindowClose,
    );

    row![min_btn, max_btn, close_btn]
        .align_y(Alignment::Center)
        .height(Length::Fill)
        .boxed()
}

fn window_control_btn<'a>(
    svg_str: &'static str,
    #[cfg(windows)] win_glyph: &'static str,
    theme_cfg: &'a ThemeConfig,
    is_close: bool,
    is_focused: bool,
    override_status: Option<iced::widget::button::Status>,
    on_press: Message,
) -> Element<'a, Message> {
    const ICON_SIZE: f32 = 14.0;
    const BTN_WIDTH: f32 = if cfg!(windows) { 46.0 } else { 42.0 };

    let icon_color = if is_focused {
        theme_cfg.text
    } else {
        theme_cfg.text_muted
    };

    #[cfg(windows)]
    let content: Element<'a, Message> = font::windows_caption_font()
        .map(|font| {
            iced::widget::text(win_glyph)
                .font(font)
                .size(10)
                .line_height(1.0)
                .boxed()
        })
        .unwrap_or_else(|| icons::icon(svg_str, ICON_SIZE, icon_color).boxed());

    #[cfg(not(windows))]
    let content: Element<'a, Message> = icons::icon(svg_str, ICON_SIZE, icon_color).boxed();

    button(
        container(content)
            .width(Length::Fixed(BTN_WIDTH))
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .height(Length::Fill)
    .padding(0)
    .style(styles::window_control_button(
        theme_cfg,
        is_close,
        is_focused,
        override_status,
    ))
    .on_press(on_press)
    .boxed()
}

/// Renders invisible 8-direction resize handles along the window edges and corners.
/// The interior uses `Space` so all normal UI pointer events pass through.
pub fn resize_handles_overlay<'a>() -> Element<'a, Message> {
    let top_row = row![
        resize_handle(
            Length::Fixed(RESIZE_CORNER_PX),
            Length::Fixed(RESIZE_CORNER_PX),
            Direction::NorthWest,
            Interaction::ResizingDiagonallyDown,
        ),
        resize_handle(
            Length::Fill,
            Length::Fixed(RESIZE_EDGE_PX),
            Direction::North,
            Interaction::ResizingVertically,
        ),
        resize_handle(
            Length::Fixed(RESIZE_CORNER_PX),
            Length::Fixed(RESIZE_CORNER_PX),
            Direction::NorthEast,
            Interaction::ResizingDiagonallyUp,
        ),
    ]
    .width(Length::Fill);

    let middle_row = row![
        resize_handle(
            Length::Fixed(RESIZE_EDGE_PX),
            Length::Fill,
            Direction::West,
            Interaction::ResizingHorizontally,
        ),
        Space::new().width(Length::Fill).height(Length::Fill),
        resize_handle(
            Length::Fixed(RESIZE_EDGE_PX),
            Length::Fill,
            Direction::East,
            Interaction::ResizingHorizontally,
        ),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let bottom_row = row![
        resize_handle(
            Length::Fixed(RESIZE_CORNER_PX),
            Length::Fixed(RESIZE_CORNER_PX),
            Direction::SouthWest,
            Interaction::ResizingDiagonallyUp,
        ),
        resize_handle(
            Length::Fill,
            Length::Fixed(RESIZE_EDGE_PX),
            Direction::South,
            Interaction::ResizingVertically,
        ),
        resize_handle(
            Length::Fixed(RESIZE_CORNER_PX),
            Length::Fixed(RESIZE_CORNER_PX),
            Direction::SouthEast,
            Interaction::ResizingDiagonallyDown,
        ),
    ]
    .width(Length::Fill)
    .align_y(Alignment::End);

    column![top_row, middle_row, bottom_row]
        .width(Length::Fill)
        .height(Length::Fill)
        .boxed()
}

fn resize_handle<'a>(
    width: Length,
    height: Length,
    direction: Direction,
    interaction: Interaction,
) -> Element<'a, Message> {
    mouse_area(Space::new().width(width).height(height))
        .on_press(Message::WindowDragResize(direction))
        .interaction(interaction)
        .boxed()
}
