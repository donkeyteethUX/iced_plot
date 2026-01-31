use crate::{Color, LineStyle, series::ShapeId};

/// A vertical line at a fixed x-coordinate.
#[derive(Debug, Clone)]
pub struct VLine {
    /// Unique identifier for the line.
    pub id: ShapeId,
    /// The x-coordinate where the vertical line is drawn.
    pub x: f64,
    /// Optional label for the line (appears in legend if provided).
    pub label: Option<String>,
    /// Color of the line.
    pub color: Color,
    /// Line width in pixels.
    pub width: f32,
    /// Line style (solid, dashed, dotted).
    pub line_style: LineStyle,
}

impl VLine {
    /// Create a new vertical line at the given x-coordinate.
    pub fn new(x: f64) -> Self {
        Self {
            id: ShapeId::new(),
            x,
            label: None,
            color: Color::from_rgb(0.5, 0.5, 0.5),
            width: 1.0,
            line_style: LineStyle::Solid,
        }
    }

    /// Set the label for this line (will appear in legend).
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.label = Some(l);
        }
        self
    }

    /// Set the color of the line.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the line width in pixels.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width.max(0.5);
        self
    }

    /// Set the line style.
    pub fn with_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }
}

/// A horizontal line at a fixed y-coordinate.
#[derive(Debug, Clone)]
pub struct HLine {
    /// Unique identifier for the line.
    pub id: ShapeId,
    /// The y-coordinate where the horizontal line is drawn.
    pub y: f64,
    /// Optional label for the line (appears in legend if provided).
    pub label: Option<String>,
    /// Color of the line.
    pub color: Color,
    /// Line width in pixels.
    pub width: f32,
    /// Line style (solid, dashed, dotted).
    pub line_style: LineStyle,
}

impl HLine {
    /// Create a new horizontal line at the given y-coordinate.
    pub fn new(y: f64) -> Self {
        Self {
            id: ShapeId::new(),
            y,
            label: None,
            color: Color::from_rgb(0.5, 0.5, 0.5),
            width: 1.0,
            line_style: LineStyle::Solid,
        }
    }

    /// Set the label for this line (will appear in legend).
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.label = Some(l);
        }
        self
    }

    /// Set the color of the line.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the line width in pixels.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width.max(0.5);
        self
    }

    /// Set the line style.
    pub fn with_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }
}
