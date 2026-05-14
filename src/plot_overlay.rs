use iced::{
    Element, Length, Point, Rectangle, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, overlay, renderer,
        widget::{Tree, tree},
    },
    alignment::{Horizontal, Vertical},
    mouse,
};

use crate::{PositionTransform, Transform};

/// An Iced element anchored to a plot position and overlaid on the plot.
pub struct PlotOverlay<'a, Message> {
    pub(crate) element: Element<'a, Message>,
    pub(crate) anchor_position: [f64; 2],
    pub(crate) anchor_position_transform: PositionTransform,
    /// Pixel offset applied after anchor projection. Positive x moves right; positive y moves up.
    pub(crate) anchor_offset: [f32; 2],
    pub(crate) align_to_anchor_vertical: Vertical,
    pub(crate) align_to_anchor_horizontal: Horizontal,
}

impl<'a, Message> PlotOverlay<'a, Message> {
    /// Create an external Iced element anchored at a plot position.
    ///
    /// The position is interpreted as normal data coordinates by default. Use
    /// [`PlotOverlay::with_axes_transform`] or [`PlotOverlay::with_transform`] for other
    /// coordinate systems.
    pub fn new(element: impl Into<Element<'a, Message>>, anchor_position: [f64; 2]) -> Self {
        Self {
            element: element.into(),
            anchor_position,
            anchor_position_transform: PositionTransform::identity(),
            anchor_offset: [0.0, 0.0],
            align_to_anchor_vertical: Vertical::Center,
            align_to_anchor_horizontal: Horizontal::Center,
        }
    }

    /// Set the plot position (in `[x,y]` coordinates) used as the overlay anchor.
    pub fn with_anchor_position(mut self, anchor_position: [f64; 2]) -> Self {
        self.anchor_position = anchor_position;
        self
    }

    /// Set a pixel offset applied after anchor projection.
    ///
    /// Positive x moves the overlay right; positive y moves it up.
    pub fn with_anchor_offset(mut self, anchor_offset: [f32; 2]) -> Self {
        self.anchor_offset = anchor_offset;
        self
    }

    /// Set how the overlay anchor position is transformed before placement.
    pub fn with_transform(mut self, transform: PositionTransform) -> Self {
        self.anchor_position_transform = transform;
        self
    }

    /// Transform the x coordinate of the overlay anchor.
    pub fn with_transform_x(mut self, transform: Transform) -> Self {
        self.anchor_position_transform.x = Some(transform);
        self
    }

    /// Transform the y coordinate of the overlay anchor.
    pub fn with_transform_y(mut self, transform: Transform) -> Self {
        self.anchor_position_transform.y = Some(transform);
        self
    }

    /// Interpret the overlay anchor as normalized plot coordinates.
    pub fn with_axes_transform(mut self) -> Self {
        self.anchor_position_transform = PositionTransform::axes();
        self
    }

    /// Set the overlay position relative to its anchor.
    ///
    /// For example, `(Horizontal::Center, Vertical::Top)` places the overlay
    /// above the anchor with its horizontal center aligned to it.
    pub fn align_to_anchor(mut self, horizontal: Horizontal, vertical: Vertical) -> Self {
        self.align_to_anchor_horizontal = horizontal;
        self.align_to_anchor_vertical = vertical;
        self
    }

    /// Set the overlay's horizontal position relative to its anchor.
    pub fn align_to_anchor_horizontal(mut self, horizontal: Horizontal) -> Self {
        self.align_to_anchor_horizontal = horizontal;
        self
    }

    /// Set the overlay's vertical position relative to its anchor.
    pub fn align_to_anchor_vertical(mut self, vertical: Vertical) -> Self {
        self.align_to_anchor_vertical = vertical;
        self
    }

    /// Map the overlay's message type into another message type.
    pub fn map<B>(self, f: impl Fn(Message) -> B + 'a) -> PlotOverlay<'a, B>
    where
        B: 'a,
        Message: 'a,
    {
        PlotOverlay {
            element: self.element.map(f),
            anchor_position: self.anchor_position,
            anchor_position_transform: self.anchor_position_transform,
            anchor_offset: self.anchor_offset,
            align_to_anchor_vertical: self.align_to_anchor_vertical,
            align_to_anchor_horizontal: self.align_to_anchor_horizontal,
        }
    }
}

pub(crate) fn positioned_overlay<'a, Message>(
    element: Element<'a, Message>,
    anchor: [f32; 2],
    horizontal_position: Horizontal,
    vertical_position: Vertical,
) -> Element<'a, Message>
where
    Message: 'a,
{
    Element::new(PositionedOverlay {
        content: element,
        anchor,
        horizontal_position,
        vertical_position,
    })
}

struct PositionedOverlay<'a, Message> {
    content: Element<'a, Message>,
    anchor: [f32; 2],
    horizontal_position: Horizontal,
    vertical_position: Vertical,
}

impl<Message> PositionedOverlay<'_, Message> {
    fn content_position(&self, size: iced::Size) -> Point {
        let x = match self.horizontal_position {
            Horizontal::Left => self.anchor[0] - size.width,
            Horizontal::Center => self.anchor[0] - size.width * 0.5,
            Horizontal::Right => self.anchor[0],
        };
        let y = match self.vertical_position {
            Vertical::Top => self.anchor[1] - size.height,
            Vertical::Center => self.anchor[1] - size.height * 0.5,
            Vertical::Bottom => self.anchor[1],
        };

        Point::new(x, y)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for PositionedOverlay<'_, Message> {
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> iced::Size<Length> {
        iced::Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);
        let size = limits.resolve(Length::Fill, Length::Fill, iced::Size::ZERO);
        let child_limits = layout::Limits::new(iced::Size::ZERO, size);
        let child = self
            .content
            .as_widget_mut()
            .layout(tree, renderer, &child_limits);
        let position = self.content_position(child.size());

        layout::Node::with_children(size, vec![child.move_to(position)])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let Some(layout) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(tree, layout, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let cursor = if cursor.is_over(layout.bounds()) {
            cursor
        } else {
            mouse::Cursor::Unavailable
        };
        if let Some(layout) = layout.children().next() {
            self.content.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if !cursor.is_over(layout.bounds()) {
            return mouse::Interaction::None;
        }

        layout
            .children()
            .next()
            .map(|layout| {
                self.content
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let Some(viewport) = bounds.intersection(viewport) else {
            return;
        };
        if let Some(layout) = layout.children().next() {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, &viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let layout = layout.children().next()?;
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}
