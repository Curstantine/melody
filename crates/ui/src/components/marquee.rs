use std::time::{SystemTime, UNIX_EPOCH};

use iced::advanced::text::Paragraph;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{layout, mouse, renderer, text, Clipboard, Layout, Shell, Widget};
use iced::{
    alignment, event, window, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size,
};

/// An auto-scrolling single-line marquee widget.
/// When the text width exceeds the available container width, it smoothly scrolls horizontally
/// with a seamless loop, pausing briefly at the starting position.
pub struct Marquee {
    content: String,
    size: Pixels,
    color: Color,
    font: Font,
    speed_px_per_sec: f32,
    gap: f32,
    pause_start_ms: u64,
    pause_end_ms: u64,
}

impl Marquee {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: Pixels(13.0),
            color: Color::WHITE,
            font: Font::DEFAULT,
            speed_px_per_sec: 35.0,
            gap: 50.0,
            pause_start_ms: 2500,
            pause_end_ms: 3000,
        }
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = size.into();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    pub fn speed(mut self, speed_px_per_sec: f32) -> Self {
        self.speed_px_per_sec = speed_px_per_sec;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn pause_ms(mut self, pause_ms: u64) -> Self {
        self.pause_start_ms = pause_ms;
        self.pause_end_ms = pause_ms;
        self
    }

    pub fn pause_start_ms(mut self, pause_start_ms: u64) -> Self {
        self.pause_start_ms = pause_start_ms;
        self
    }

    pub fn pause_end_ms(mut self, pause_end_ms: u64) -> Self {
        self.pause_end_ms = pause_end_ms;
        self
    }
}

pub struct State<P: Paragraph> {
    paragraph: text::paragraph::Plain<P>,
}

impl<P: Paragraph> Default for State<P> {
    fn default() -> Self {
        Self {
            paragraph: text::paragraph::Plain::default(),
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Marquee
where
    Renderer: text::Renderer<Font = Font>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        let shaping = if self.content.is_ascii() {
            iced::widget::text::Shaping::Basic
        } else {
            iced::widget::text::Shaping::Advanced
        };

        state.paragraph.update(iced::advanced::text::Text {
            content: self.content.as_str(),
            bounds: Size::new(f32::INFINITY, f32::INFINITY),
            size: self.size,
            line_height: iced::widget::text::LineHeight::default(),
            font: self.font,
            horizontal_alignment: alignment::Horizontal::Left,
            vertical_alignment: alignment::Vertical::Top,
            shaping,
            wrapping: iced::widget::text::Wrapping::None,
        });

        let height = state.paragraph.raw().min_height().max(self.size.0);

        let node_size = limits.width(Length::Fill).height(Length::Shrink).resolve(
            Length::Fill,
            Length::Shrink,
            Size::new(0.0, height),
        );

        layout::Node::new(node_size)
    }

    fn on_event(
        &mut self,
        _tree: &mut Tree,
        event: Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            shell.request_redraw(window::RedrawRequest::NextFrame);
        }
        event::Status::Ignored
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let paragraph = state.paragraph.raw();

        let bounds = layout.bounds();
        let clip_rect = bounds.intersection(viewport).unwrap_or(bounds);

        let text_w = paragraph.min_width();
        let text_h = paragraph.min_height().max(self.size.0);
        let y_pos = bounds.y + (bounds.height - text_h) / 2.0;

        if text_w <= bounds.width {
            // Text fits cleanly without overflowing
            renderer.fill_paragraph(
                paragraph,
                Point::new(bounds.x, y_pos),
                self.color,
                *viewport,
            );
            return;
        }

        // Text overflows: compute marquee scroll translation
        let cycle_distance = text_w + self.gap;
        let scroll_duration_ms = (cycle_distance / self.speed_px_per_sec * 1000.0) as u64;
        let total_cycle_ms = self.pause_start_ms + scroll_duration_ms + self.pause_end_ms;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let cycle_pos_ms = if total_cycle_ms > 0 {
            now_ms % total_cycle_ms
        } else {
            0
        };

        let offset_x = if cycle_pos_ms <= self.pause_start_ms {
            0.0
        } else if cycle_pos_ms < self.pause_start_ms + scroll_duration_ms {
            let active_ms = cycle_pos_ms - self.pause_start_ms;
            (active_ms as f32 / 1000.0) * self.speed_px_per_sec
        } else {
            0.0
        };

        // Clip to the bounds of the marquee widget
        renderer.with_layer(clip_rect, |renderer| {
            let x1 = bounds.x - offset_x;
            renderer.fill_paragraph(paragraph, Point::new(x1, y_pos), self.color, clip_rect);

            if offset_x > 0.0 {
                let x2 = x1 + cycle_distance;
                renderer.fill_paragraph(paragraph, Point::new(x2, y_pos), self.color, clip_rect);
            }
        });
    }
}

impl<'a, Message: 'a, Theme: 'a, Renderer: text::Renderer<Font = Font> + 'a> From<Marquee>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(m: Marquee) -> Self {
        Self::new(m)
    }
}
