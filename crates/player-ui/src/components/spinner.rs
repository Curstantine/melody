use std::time::{SystemTime, UNIX_EPOCH};

use iced::advanced::{layout, mouse, renderer, svg, widget::Tree, Clipboard, Layout, Shell, Widget};
use iced::{event, window, Color, Element, Event, Length, Radians, Rectangle, Size};

pub struct Spinner {
    handle: svg::Handle,
    size: f32,
    color: Option<Color>,
}

impl Spinner {
    pub fn new(handle: svg::Handle) -> Self {
        Self {
            handle,
            size: 32.0,
            color: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Spinner
where
    Renderer: svg::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(&self, _tree: &mut Tree, _r: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout::atomic(limits, self.size, self.size)
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
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // One full turn per second, derived from the clock.
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() % 1000)
            .unwrap_or(0);
        let angle = Radians(ms as f32 / 1000.0 * std::f32::consts::TAU);

        let mut svg_item = svg::Svg::new(self.handle.clone()).rotation(angle);
        if let Some(color) = self.color {
            svg_item = svg_item.color(color);
        }

        renderer.draw_svg(svg_item, layout.bounds());
    }
}

impl<'a, Message: 'a, Theme: 'a, Renderer: svg::Renderer + 'a> From<Spinner>
    for Element<'a, Message, Theme, Renderer>
{
    fn from(s: Spinner) -> Self {
        Self::new(s)
    }
}
