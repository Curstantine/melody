use iced::advanced::{
    layout, mouse, overlay, renderer,
    widget::{tree, Meta, Operation, Tree},
    Layout, Shell, Widget,
};
pub use iced::widget::button::{Catalog, Status, Style, StyleFn};
use iced::{Element, Event, Length, Padding, Rectangle, Size, Vector};

/// A button widget that keeps the default arrow cursor unless explicitly
/// configured otherwise (e.g., for hyperlinks).
pub struct Button<'a, Message, W = iced::Element<'a, Message>, Theme = iced::Theme>
where
    Theme: Catalog,
{
    inner: iced::widget::Button<'a, Message, W, Theme>,
    interaction: Option<mouse::Interaction>,
}

impl<'a, Message, W, Theme> Button<'a, Message, W, Theme>
where
    Theme: Catalog,
{
    /// Creates a new [`Button`] with the given content.
    pub fn new(content: W) -> Self {
        Self {
            inner: iced::widget::Button::new(content),
            interaction: None,
        }
    }

    /// Sets the width of the [`Button`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.inner = self.inner.width(width);
        self
    }

    /// Sets the height of the [`Button`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.inner = self.inner.height(height);
        self
    }

    /// Sets the [`Padding`] of the [`Button`].
    pub fn padding<P: Into<Padding>>(mut self, padding: P) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    pub fn on_press(mut self, on_press: Message) -> Self {
        self.inner = self.inner.on_press(on_press);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed.
    pub fn on_press_with(mut self, on_press: impl Fn() -> Message + 'a) -> Self {
        self.inner = self.inner.on_press_with(on_press);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed,
    /// if `Some`.
    pub fn on_press_maybe(mut self, on_press: Option<Message>) -> Self {
        self.inner = self.inner.on_press_maybe(on_press);
        self
    }

    /// Sets the message that will be produced when the [`Button`] is pressed,
    /// if `Some`.
    pub fn on_press_maybe_with(mut self, on_press: Option<impl Fn() -> Message + 'a>) -> Self {
        self.inner = self.inner.on_press_maybe_with(on_press);
        self
    }

    /// Sets whether the contents of the [`Button`] should be clipped on overflow.
    pub fn clip(mut self, clip: bool) -> Self {
        self.inner = self.inner.clip(clip);
        self
    }

    /// Sets the style of the [`Button`].
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.inner = self.inner.style(style);
        self
    }

    /// Sets the mouse cursor interaction when hovering the [`Button`].
    ///
    /// By default, buttons use the standard arrow cursor (`Interaction::default()`).
    pub fn interaction(mut self, interaction: mouse::Interaction) -> Self {
        self.interaction = Some(interaction);
        self
    }
}

/// Creates a new [`Button`] with the given content.
pub fn button<'a, Message, W, Theme>(content: W) -> Button<'a, Message, W, Theme>
where
    Theme: Catalog,
{
    Button::new(content)
}

impl<'a, Message, W, Theme> Meta for Button<'a, Message, W, Theme> where Theme: Catalog {}

impl<'a, Message, W, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Button<'a, Message, W, Theme>
where
    Message: 'a + Clone,
    W: Widget<Message, Theme, Renderer>,
    Renderer: 'a + iced::advanced::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::tag(
            &self.inner,
        )
    }

    fn state(&self) -> tree::State {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::state(
            &self.inner,
        )
    }

    fn diff(&mut self, tree: &mut Tree) {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::diff(
            &mut self.inner,
            tree,
        );
    }

    fn size(&self) -> Size<Length> {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::size(
            &self.inner,
        )
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::layout(
            &mut self.inner,
            tree,
            renderer,
            limits,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::operate(
            &mut self.inner,
            tree,
            layout,
            viewport,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::update(
            &mut self.inner,
            tree,
            event,
            layout,
            cursor,
            renderer,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::draw(
            &self.inner,
            tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        match self.interaction {
            Some(interaction) if cursor.is_over(layout.bounds()) => {
                // Delegate to inner first so disabled buttons still report default cursor.
                let inner_interaction = <iced::widget::Button<'a, Message, W, Theme> as Widget<
                    Message,
                    Theme,
                    Renderer,
                >>::mouse_interaction(
                    &self.inner, tree, layout, cursor, viewport, renderer
                );
                if inner_interaction == mouse::Interaction::default() {
                    inner_interaction
                } else {
                    interaction
                }
            }
            _ => mouse::Interaction::default(),
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
        window: Size,
    ) -> Vec<overlay::Element<'b, Message, Theme, Renderer>> {
        <iced::widget::Button<'a, Message, W, Theme> as Widget<Message, Theme, Renderer>>::overlay(
            &mut self.inner,
            tree,
            layout,
            renderer,
            viewport,
            translation,
            window,
        )
    }
}

impl<'a, Message, W, Theme, Renderer> From<Button<'a, Message, W, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    W: Widget<Message, Theme, Renderer> + 'a,
    Theme: Catalog + 'a,
    Renderer: 'a + iced::advanced::Renderer,
{
    fn from(button: Button<'a, Message, W, Theme>) -> Self {
        iced::widget::Widget::boxed(button)
    }
}
