use iced::advanced::{
    layout, mouse, overlay, renderer,
    widget::{Meta, Operation, Tree},
    Layout, Shell, Widget,
};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// A wrapper widget that shifts its child vertically by `offset_y` pixels
/// during layout (supporting negative offsets).
pub struct VerticalOffset<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    offset_y: f32,
}

impl<'a, Message, Theme, Renderer> VerticalOffset<'a, Message, Theme, Renderer> {
    /// Creates a new [`VerticalOffset`] shifting `content` vertically by `offset_y`.
    pub fn new(offset_y: f32, content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            offset_y,
        }
    }
}

/// Shifts `content` vertically by `offset_y` pixels during layout.
pub fn offset_y<'a, Message, Theme, Renderer>(
    offset_y: f32,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> VerticalOffset<'a, Message, Theme, Renderer> {
    VerticalOffset::new(offset_y, content)
}

impl<'a, Message, Theme, Renderer> Meta for VerticalOffset<'a, Message, Theme, Renderer> {}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for VerticalOffset<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) {
        self.content.layout(&mut tree.children[0], renderer, limits);

        tree.children[0].translation.y = self.offset_y;
        tree.size = Size::new(
            tree.children[0].size.width,
            (tree.children[0].size.height + self.offset_y).max(0.0),
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
        let (child_layout, child_tree) = layout.iter_mut(&mut tree.children).next().unwrap();
        self.content
            .operate(child_tree, child_layout, viewport, renderer, operation);
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
        let (child_layout, child_tree) = layout.iter_mut(&mut tree.children).next().unwrap();
        self.content.update(
            child_tree,
            event,
            child_layout,
            cursor,
            renderer,
            shell,
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
        let (child_layout, child_tree) = layout.iter(&tree.children).next().unwrap();
        self.content
            .mouse_interaction(child_tree, child_layout, cursor, viewport, renderer)
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
        let (child_layout, child_tree) = layout.iter(&tree.children).next().unwrap();
        self.content.draw(
            child_tree,
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );
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
        let (child_layout, child_tree) = layout.iter_mut(&mut tree.children).next().unwrap();
        self.content.overlay(
            child_tree,
            child_layout,
            renderer,
            viewport,
            translation,
            window,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<VerticalOffset<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(widget: VerticalOffset<'a, Message, Theme, Renderer>) -> Self {
        iced::widget::Widget::boxed(widget)
    }
}
