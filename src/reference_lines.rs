use crate::{Color, LineStyle, LineType, Size, series::ShapeId};

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
    /// Line styling options, including width and pattern (solid, dashed, dotted).
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
            line_style: LineStyle::default(),
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
        self.line_style.width = Size::Pixels(width.max(0.5));
        self
    }

    /// Set the line width in world units.
    pub fn with_width_world(mut self, width: f64) -> Self {
        self.line_style.width = Size::World(width.max(f64::EPSILON));
        self
    }

    /// Set the line style.
    pub fn with_style(mut self, style: LineStyle) -> Self {
        let old_width = self.line_style.width;
        let preserve_width = style.width == LineStyle::default().width;
        self.line_style = style;
        if preserve_width {
            self.line_style.width = old_width;
        }
        self
    }

    /// Set only the line type while preserving the current width.
    pub fn with_line_type(mut self, line_type: LineType) -> Self {
        self.line_style.line_type = line_type;
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
    /// Line styling options, including width and pattern (solid, dashed, dotted).
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
            line_style: LineStyle::default(),
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
        self.line_style.width = Size::Pixels(width.max(0.5));
        self
    }

    /// Set the line width in world units.
    pub fn with_width_world(mut self, width: f64) -> Self {
        self.line_style.width = Size::World(width.max(f64::EPSILON));
        self
    }

    /// Set the line style.
    pub fn with_style(mut self, style: LineStyle) -> Self {
        let old_width = self.line_style.width;
        let preserve_width = style.width == LineStyle::default().width;
        self.line_style = style;
        if preserve_width {
            self.line_style.width = old_width;
        }
        self
    }

    /// Set only the line type while preserving the current width.
    pub fn with_line_type(mut self, line_type: LineType) -> Self {
        self.line_style.line_type = line_type;
        self
    }
}
