use iced::advanced::{
    layout, mouse, overlay, renderer,
    widget::{Meta, Operation, Tree},
    Layout, Shell, Widget,
};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// A two-layer stack that preserves hover state on the underlay even when
/// the cursor is over an interactive widget in the overlay.
///
/// Unlike [`iced::widget::Stack`], which masks the cursor for lower layers
/// whenever an upper layer reports a non-`None` mouse interaction,
/// `HoverStack` forwards the real cursor to both layers during `update`
/// (unless the overlay captures the event) and `draw`.
pub struct HoverStack<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    children: [Element<'a, Message, Theme, Renderer>; 2],
    width: Length,
    height: Length,
}

impl<'a, Message, Theme, Renderer> HoverStack<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    /// Creates a new [`HoverStack`] with `underlay` below `overlay`.
    pub fn new(
        underlay: impl Into<Element<'a, Message, Theme, Renderer>>,
        overlay: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        let underlay = underlay.into();
        let overlay = overlay.into();
        let size = underlay.size();

        Self {
            children: [underlay, overlay],
            width: size.width,
            height: size.height,
        }
    }
}

/// Creates a new [`HoverStack`] with `underlay` below `overlay`.
pub fn hover_stack<'a, Message, Theme, Renderer>(
    underlay: impl Into<Element<'a, Message, Theme, Renderer>>,
    overlay: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> HoverStack<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    HoverStack::new(underlay, overlay)
}

impl<'a, Message, Theme, Renderer> Meta for HoverStack<'a, Message, Theme, Renderer> {}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for HoverStack<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.children);

        let size = self.children[0].size();
        self.width = size.width;
        self.height = size.height;
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) {
        let limits = limits.width(self.width).height(self.height);

        self.children[0].layout(&mut tree.children[0], renderer, &limits);

        let size = limits.resolve(self.width, self.height, tree.children[0].size);
        let overlay_limits = layout::Limits::new(Size::ZERO, size);

        self.children[1].layout(&mut tree.children[1], renderer, &overlay_limits);

        tree.size = size;
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout,
        viewport: &Rectangle,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds(), viewport);
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(layout.iter_mut(&mut tree.children))
                .for_each(|(child, (layout, state))| {
                    child.operate(state, layout, viewport, renderer, operation);
                });
        });
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
        let mut iter = layout.iter_mut(&mut tree.children);
        let (underlay_layout, underlay_tree) = iter.next().unwrap();
        let (overlay_layout, overlay_tree) = iter.next().unwrap();

        self.children[1].update(
            overlay_tree,
            event,
            overlay_layout,
            cursor,
            renderer,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        self.children[0].update(
            underlay_tree,
            event,
            underlay_layout,
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
        let mut iter = layout.iter(&tree.children);
        let (underlay_layout, underlay_tree) = iter.next().unwrap();
        let (overlay_layout, overlay_tree) = iter.next().unwrap();

        let overlay_interaction = self.children[1].mouse_interaction(
            overlay_tree,
            overlay_layout,
            cursor,
            viewport,
            renderer,
        );

        if overlay_interaction != mouse::Interaction::None {
            return overlay_interaction;
        }

        self.children[0].mouse_interaction(
            underlay_tree,
            underlay_layout,
            cursor,
            viewport,
            renderer,
        )
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
        if let Some(viewport) = layout.bounds().intersection(viewport) {
            let mut iter = layout.iter(&tree.children);
            let (underlay_layout, underlay_tree) = iter.next().unwrap();
            let (overlay_layout, overlay_tree) = iter.next().unwrap();

            self.children[0].draw(
                underlay_tree,
                renderer,
                theme,
                style,
                underlay_layout,
                cursor,
                &viewport,
            );

            renderer.with_layer(viewport, |renderer| {
                self.children[1].draw(
                    overlay_tree,
                    renderer,
                    theme,
                    style,
                    overlay_layout,
                    cursor,
                    &viewport,
                );
            });
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
        overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
            window,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<HoverStack<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(stack: HoverStack<'a, Message, Theme, Renderer>) -> Self {
        iced::widget::Widget::boxed(stack)
    }
}
