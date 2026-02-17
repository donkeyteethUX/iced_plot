use crate::{Color, series::ShapeId};

/// A filled region between two existing shapes.
///
/// `begin` and `end` must reference existing [`Series`](crate::Series),
/// [`HLine`](crate::HLine), or [`VLine`](crate::VLine) shapes in the same plot.
#[derive(Debug, Clone)]
pub struct Fill {
    /// Unique identifier for this fill region.
    pub id: ShapeId,
    /// Starting boundary shape.
    pub begin: ShapeId,
    /// Ending boundary shape.
    pub end: ShapeId,
    /// Optional label for legends.
    pub label: Option<String>,
    /// Fill color (and opacity).
    pub color: Color,
}

impl Fill {
    /// Create a new [`Fill`] between two shapes.
    pub fn new(begin: ShapeId, end: ShapeId) -> Self {
        Self {
            id: ShapeId::new(),
            begin,
            end,
            label: None,
            color: Color::from_rgba(0.2, 0.6, 1.0, 0.25),
        }
    }

    /// Set a label for this fill (shown in legend when non-empty).
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        if !label.is_empty() {
            self.label = Some(label);
        }
        self
    }

    /// Set fill color, including opacity.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}
