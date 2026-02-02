use core::fmt;

use iced::Rectangle;

use crate::{Color, camera::Camera, point::MarkerType};

/// Line styling options for series connections.
///
/// Determines how points in a series are connected.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    /// Solid continuous line.
    Solid,
    /// Dotted line with configurable spacing.
    Dotted { spacing: f32 },
    /// Dashed line with configurable dash length.
    Dashed { length: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Marker size modes.
pub enum MarkerSize {
    /// Marker size in logical pixels. The marker will be centered on the data position.
    ///
    /// This is usually the right default.
    Pixels(f32),

    /// Marker size in world units. The marker will be painted at the data position such that
    /// the lower-left corner of the marker is at the data position.
    ///
    /// This is useful for implementing heatmaps and similar applications where markers
    /// need to paint an area of the plot.
    World(f64),
}

impl From<f32> for MarkerSize {
    fn from(size: f32) -> Self {
        Self::Pixels(size)
    }
}

impl MarkerSize {
    pub(crate) fn to_raw(self) -> (f32, u32) {
        match self {
            Self::Pixels(size) => (size, 0),
            Self::World(size) => (size as f32, 1),
        }
    }
    pub(crate) fn marker_size_px(
        size: f32,
        size_mode: u32,
        camera: &Camera,
        bounds: &Rectangle,
    ) -> f32 {
        if size_mode != crate::point::MARKER_SIZE_WORLD {
            return size;
        }
        let width = bounds.width.max(1.0) as f64;
        let height = bounds.height.max(1.0) as f64;
        let world_per_px_x = (2.0 * camera.half_extents.x) / width;
        let world_per_px_y = (2.0 * camera.half_extents.y) / height;
        let world_per_px_x = world_per_px_x.max(1e-12);
        let world_per_px_y = world_per_px_y.max(1e-12);
        let px_x = size as f64 / world_per_px_x;
        let px_y = size as f64 / world_per_px_y;
        px_x.max(px_y) as f32
    }
    pub(crate) fn to_px(self, camera: &Camera, bounds: &Rectangle) -> f32 {
        let (size, size_mode) = self.to_raw();
        Self::marker_size_px(size, size_mode, camera, bounds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Marker styling options for series points.
///
/// Defines how individual data points are rendered.
pub struct MarkerStyle {
    /// Size of the marker in pixels or world units.
    pub size: MarkerSize,
    /// Shape of the marker.
    pub marker_type: MarkerType,
}

impl Default for MarkerStyle {
    fn default() -> Self {
        Self {
            size: MarkerSize::Pixels(5.0),
            marker_type: MarkerType::FilledCircle,
        }
    }
}

impl MarkerStyle {
    pub fn new(size: f32, marker_type: MarkerType) -> Self {
        Self {
            size: MarkerSize::Pixels(size),
            marker_type,
        }
    }

    pub fn new_world(size: f64, marker_type: MarkerType) -> Self {
        Self {
            size: MarkerSize::World(size),
            marker_type,
        }
    }

    pub fn circle(size: f32) -> Self {
        Self {
            size: MarkerSize::Pixels(size),
            marker_type: MarkerType::FilledCircle,
        }
    }

    pub fn ring(size: f32) -> Self {
        Self {
            size: MarkerSize::Pixels(size),
            marker_type: MarkerType::EmptyCircle,
        }
    }

    pub fn square(size: f32) -> Self {
        Self {
            size: MarkerSize::Pixels(size),
            marker_type: MarkerType::Square,
        }
    }

    pub fn star(size: f32) -> Self {
        Self {
            size: MarkerSize::Pixels(size),
            marker_type: MarkerType::Star,
        }
    }

    pub fn triangle(size: f32) -> Self {
        Self {
            size: MarkerSize::Pixels(size),
            marker_type: MarkerType::Triangle,
        }
    }
}

/// Errors that can occur when constructing or adding a series.
#[derive(Debug, Clone, PartialEq)]
pub enum SeriesError {
    /// No points provided to the series.
    Empty,
    /// Series has neither markers nor lines enabled.
    NoMarkersAndNoLines,
    /// A series with the given ID does not exist.
    NotFound(ShapeId),
    /// Axis limits are not properly set (min >= max).
    InvalidAxisLimits,
    /// Per-point colors length does not match positions length.
    InvalidPointColorsLength,
}

/// Unique identifier for a shape in the plot.
///
/// You can obtain the [ShapeId] of [Series], [VLine](crate::VLine), or [HLine](crate::HLine) by `id` field:
/// ```rust
/// use iced_plot::{Series, VLine, HLine, MarkerStyle, LineStyle};
/// let series = Series::new(vec![[0.0, 0.0], [1.0, 1.0]], MarkerStyle::circle(5.0), LineStyle::Solid);
/// let id1 = series.id;
///
/// let vline = VLine::new(0.0);
/// let id2 = vline.id;
///
/// let hline = HLine::new(0.0);
/// let id3 = hline.id;
///
/// assert_ne!(id1, id2);
/// assert_ne!(id1, id3);
/// assert_ne!(id2, id3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub(crate) u64);
impl ShapeId {
    /// Create a new unique shape ID (0, 1, 2, ...).
    ///
    /// Used internally by the plot widget to create unique IDs for series, vlines, and hlines.
    pub(crate) fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}
impl fmt::Display for ShapeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shape({})", self.0)
    }
}

/// A collection of per-point styled data to be plotted.
///
/// Represents a single series of data points, which may be rendered with markers,
/// lines, or both. The same series can contain any number of points.
#[derive(Debug, Clone)]
pub struct Series {
    /// Unique identifier for the series.
    pub id: ShapeId,

    /// Series point positions.
    pub positions: Vec<[f64; 2]>,

    /// Optional per-point colors. Must match the length of `positions` if set.
    pub point_colors: Option<Vec<Color>>,

    /// Optional label for the entire series.
    pub label: Option<String>,

    /// Color of the entire series.
    ///
    /// Overridden by per-point colors if they are set.
    pub color: Color,

    /// Optional marker style for the series. If none, no markers are drawn.
    pub marker_style: Option<MarkerStyle>,

    /// Line style for connecting markers. If None, no line is drawn.
    pub line_style: Option<LineStyle>,
}

impl Series {
    /// Create a new series with both markers and lines.
    pub fn new(positions: Vec<[f64; 2]>, marker_style: MarkerStyle, line_style: LineStyle) -> Self {
        Self {
            id: ShapeId::new(),
            positions,
            point_colors: None,
            label: None,
            color: Color::from_rgb(0.3, 0.3, 0.9),
            marker_style: Some(marker_style),
            line_style: Some(line_style),
        }
    }

    /// Create a new line-only series.
    pub fn line_only(positions: Vec<[f64; 2]>, line_style: LineStyle) -> Self {
        Self {
            id: ShapeId::new(),
            positions,
            point_colors: None,
            label: None,
            color: Color::from_rgb(0.3, 0.3, 0.9),
            marker_style: None,
            line_style: Some(line_style),
        }
    }

    /// Create a new marker-only series.
    pub fn markers_only(positions: Vec<[f64; 2]>, marker_style: MarkerStyle) -> Self {
        Self {
            id: ShapeId::new(),
            positions,
            point_colors: None,
            label: None,
            color: Color::from_rgb(0.3, 0.3, 0.9),
            marker_style: Some(marker_style),
            line_style: None,
        }
    }

    /// Create a new series with circle markers.
    pub fn circles(positions: Vec<[f64; 2]>, size: f32) -> Self {
        Self::markers_only(positions, MarkerStyle::circle(size))
    }

    /// Create a new series with square markers.
    pub fn squares(positions: Vec<[f64; 2]>, size: f32) -> Self {
        Self::markers_only(positions, MarkerStyle::square(size))
    }

    /// Create a new series with star markers.
    pub fn stars(positions: Vec<[f64; 2]>, size: f32) -> Self {
        Self::markers_only(positions, MarkerStyle::star(size))
    }

    /// Create a new series with triangle markers.
    pub fn triangles(positions: Vec<[f64; 2]>, size: f32) -> Self {
        Self::markers_only(positions, MarkerStyle::triangle(size))
    }

    /// Set an label for the series.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.label = Some(l);
        }
        self
    }

    /// Set the marker style for the series.
    pub fn with_marker_style(mut self, style: MarkerStyle) -> Self {
        self.marker_style = Some(style);
        self
    }

    /// Set the color of the entire series. Overridden by per-point colors if they are set.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }

    /// Set per-point colors for the series. Length must match the number of positions.
    pub fn with_point_colors(mut self, colors: Vec<Color>) -> Self {
        self.point_colors = Some(colors);
        self
    }

    /// Set or change the line style for the series.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = Some(style);
        self
    }

    /// Set solid line style.
    pub fn line_solid(self) -> Self {
        self.line_style(LineStyle::Solid)
    }

    /// Set dotted line style with given spacing.
    pub fn line_dotted(self, spacing: f32) -> Self {
        self.line_style(LineStyle::Dotted { spacing })
    }

    /// Set dashed line style with given dash length.
    pub fn line_dashed(self, length: f32) -> Self {
        self.line_style(LineStyle::Dashed { length })
    }

    pub(super) fn validate(&self) -> Result<(), SeriesError> {
        if self.positions.is_empty() {
            return Err(SeriesError::Empty);
        }
        if self.marker_style.is_none() && self.line_style.is_none() {
            return Err(SeriesError::NoMarkersAndNoLines);
        }
        if let Some(colors) = &self.point_colors
            && colors.len() != self.positions.len()
        {
            return Err(SeriesError::InvalidPointColorsLength);
        }
        Ok(())
    }
}
