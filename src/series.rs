use core::fmt;

use iced::Rectangle;

use crate::{
    Color,
    camera::Camera,
    point::MarkerType,
    transform::{PositionTransform, Transform},
};

/// Line styling options for series connections.
///
/// Determines how points in a series are connected.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LineType {
    /// Solid continuous line.
    #[default]
    Solid,
    /// Dotted line with configurable spacing.
    Dotted { spacing: f32 },
    /// Dashed line with configurable dash length.
    Dashed { length: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Line styling options for series lines.
///
/// Defines how individual lines are rendered.
pub struct LineStyle {
    /// Width of the line in pixels or world units.
    pub width: Size,
    /// Shape of the line.
    pub line_type: LineType,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            width: Size::Pixels(1.0),
            line_type: LineType::Solid,
        }
    }
}

impl LineStyle {
    /// Create a line style from an explicit width and line type.
    pub fn new(width: Size, line_type: LineType) -> Self {
        Self { width, line_type }
    }

    /// Create a solid line style with the default width of 1 logical pixel.
    pub fn solid() -> Self {
        Self::default()
    }

    /// Create a dotted line style with the given dot spacing in logical pixels.
    pub fn dotted(spacing: f32) -> Self {
        Self {
            line_type: LineType::Dotted { spacing },
            ..Self::default()
        }
    }

    /// Create a dashed line style with the given dash length in logical pixels.
    pub fn dashed(length: f32) -> Self {
        Self {
            line_type: LineType::Dashed { length },
            ..Self::default()
        }
    }

    /// Set the line width in either logical pixels or world units.
    pub fn with_width(mut self, width: impl Into<Size>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the line width in logical pixels.
    pub fn with_pixel_width(mut self, width: f32) -> Self {
        self.width = Size::Pixels(width);
        self
    }

    /// Set the line width in world units.
    pub fn with_world_width(mut self, width: f64) -> Self {
        self.width = Size::World(width);
        self
    }

    /// Set the line type while preserving the current width.
    pub fn with_line_type(mut self, line_type: LineType) -> Self {
        self.line_type = line_type;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Shared size modes for markers and line widths.
pub enum Size {
    /// Size in logical pixels.
    ///
    /// Markers are centered on the data position. Lines use this as a screen-space width.
    ///
    /// This is usually the right default.
    Pixels(f32),

    /// Size in world units.
    ///
    /// Markers are painted such that the lower-left corner is at the data position.
    /// Lines use this as a width measured directly in plot units.
    ///
    /// This is useful for implementing heatmaps and similar applications where markers
    /// need to paint an area of the plot.
    World(f64),
}

impl From<f32> for Size {
    fn from(size: f32) -> Self {
        Self::Pixels(size)
    }
}

impl Size {
    pub(crate) fn to_raw(self) -> (f32, u32) {
        match self {
            Self::Pixels(size) => (size, 0),
            Self::World(size) => (size as f32, 1),
        }
    }
    pub(crate) fn size_px(size: f32, size_mode: u32, camera: &Camera, bounds: &Rectangle) -> f32 {
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
        Self::size_px(size, size_mode, camera, bounds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Marker styling options for series points.
///
/// Defines how individual data points are rendered.
pub struct MarkerStyle {
    /// Size of the marker in pixels or world units.
    pub size: Size,
    /// Shape of the marker.
    pub marker_type: MarkerType,
}

impl Default for MarkerStyle {
    fn default() -> Self {
        Self {
            size: Size::Pixels(5.0),
            marker_type: MarkerType::FilledCircle,
        }
    }
}

impl MarkerStyle {
    pub fn new(size: f32, marker_type: MarkerType) -> Self {
        Self {
            size: Size::Pixels(size),
            marker_type,
        }
    }

    pub fn new_world(size: f64, marker_type: MarkerType) -> Self {
        Self {
            size: Size::World(size),
            marker_type,
        }
    }

    pub fn circle(size: f32) -> Self {
        Self {
            size: Size::Pixels(size),
            marker_type: MarkerType::FilledCircle,
        }
    }

    pub fn ring(size: f32) -> Self {
        Self {
            size: Size::Pixels(size),
            marker_type: MarkerType::EmptyCircle,
        }
    }

    pub fn square(size: f32) -> Self {
        Self {
            size: Size::Pixels(size),
            marker_type: MarkerType::Square,
        }
    }

    pub fn star(size: f32) -> Self {
        Self {
            size: Size::Pixels(size),
            marker_type: MarkerType::Star,
        }
    }

    pub fn triangle(size: f32) -> Self {
        Self {
            size: Size::Pixels(size),
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
    /// Invalid AxisScale configuration (e.g. log scale with non-positive base).
    InvalidAxisScale,
    /// Per-point colors length does not match positions length.
    InvalidPointColorsLength,
    /// Fill begin/end must reference different shapes.
    InvalidFillEndpoints,
    /// Fill endpoint references a shape that does not exist in the widget.
    FillEndpointNotFound(ShapeId),
}

/// Unique identifier for a shape in the plot.
///
/// You can obtain the [ShapeId] of [Series], [VLine](crate::VLine), [HLine](crate::HLine),
/// or [Fill](crate::Fill) by `id` field:
/// ```rust
/// use iced_plot::{Series, VLine, HLine, MarkerStyle, LineStyle};
/// let series = Series::new(
///     vec![[0.0, 0.0], [1.0, 1.0]],
///     MarkerStyle::circle(5.0),
///     LineStyle::solid(),
/// );
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

    /// How this series interprets or converts point positions before drawing.
    pub transform: PositionTransform,

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

    /// Can be hovered or picked. Defaults to `true`.
    pub pickable: bool,
}

impl Series {
    /// Create a new series with both markers and lines.
    pub fn new(positions: Vec<[f64; 2]>, marker_style: MarkerStyle, line_style: LineStyle) -> Self {
        Self {
            id: ShapeId::new(),
            positions,
            transform: PositionTransform::default(),
            point_colors: None,
            label: None,
            color: Color::from_rgb(0.3, 0.3, 0.9),
            marker_style: Some(marker_style),
            line_style: Some(line_style),
            pickable: true,
        }
    }

    /// Create a new line-only series.
    pub fn line_only(positions: Vec<[f64; 2]>, line_style: LineStyle) -> Self {
        Self {
            id: ShapeId::new(),
            positions,
            transform: PositionTransform::default(),
            point_colors: None,
            label: None,
            color: Color::from_rgb(0.3, 0.3, 0.9),
            marker_style: None,
            line_style: Some(line_style),
            pickable: true,
        }
    }

    /// Create a new marker-only series.
    pub fn markers_only(positions: Vec<[f64; 2]>, marker_style: MarkerStyle) -> Self {
        Self {
            id: ShapeId::new(),
            positions,
            transform: PositionTransform::default(),
            point_colors: None,
            label: None,
            color: Color::from_rgb(0.3, 0.3, 0.9),
            marker_style: Some(marker_style),
            line_style: None,
            pickable: true,
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

    /// Set how this series interprets or converts x/y values before drawing.
    ///
    /// For normal data values, conversion runs before the plot's axis scale.
    /// `Transform::axes()` uses normalized plot positions instead.
    pub fn with_transform(mut self, transform: PositionTransform) -> Self {
        self.transform = transform;
        self
    }

    /// Set how this series interprets or converts x values before drawing.
    ///
    /// For normal data values, conversion runs before the plot's x-axis scale.
    /// `Transform::axes()` uses normalized plot positions instead.
    pub fn with_transform_x(mut self, transform: Transform) -> Self {
        self.transform.x = Some(transform);
        self
    }

    /// Set how this series interprets or converts y values before drawing.
    ///
    /// For normal data values, conversion runs before the plot's y-axis scale.
    /// `Transform::axes()` uses normalized plot positions instead.
    pub fn with_transform_y(mut self, transform: Transform) -> Self {
        self.transform.y = Some(transform);
        self
    }

    /// Interpret all positions as normalized plot coordinates.
    ///
    /// `(0.0, 0.0)` is the lower-left of the plot area and `(1.0, 1.0)` is
    /// the upper-right.
    pub fn with_axes_transform(mut self) -> Self {
        self.transform = PositionTransform::axes();
        self
    }

    /// Set how this series interprets or converts x values before drawing.
    pub fn with_x_transform(mut self, transform: Transform) -> Self {
        self.transform.x = Some(transform);
        self
    }

    /// Set how this series interprets or converts y values before drawing.
    pub fn with_y_transform(mut self, transform: Transform) -> Self {
        self.transform.y = Some(transform);
        self
    }

    /// Enable or disable interactive hover/pick behavior for this series.
    pub fn with_pickable(mut self, pickable: bool) -> Self {
        self.pickable = pickable;
        self
    }

    /// Set or change the line style for the series.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = Some(style);
        self
    }

    /// Set or change the line width for the series.
    pub fn line_width(mut self, width: impl Into<Size>) -> Self {
        let width = width.into();
        self.line_style = Some(self.line_style.unwrap_or_default().with_width(width));
        self
    }

    /// Set or change the line width for the series in world units.
    pub fn line_width_world(mut self, width: f64) -> Self {
        self.line_style = Some(self.line_style.unwrap_or_default().with_world_width(width));
        self
    }

    /// Set or change only the line type while preserving width if it already exists.
    pub fn line_type(mut self, line_type: LineType) -> Self {
        self.line_style = Some(
            self.line_style
                .unwrap_or_default()
                .with_line_type(line_type),
        );
        self
    }

    /// Set solid line style.
    pub fn line_solid(self) -> Self {
        self.line_type(LineType::Solid)
    }

    /// Set dotted line style with given spacing.
    pub fn line_dotted(self, spacing: f32) -> Self {
        self.line_type(LineType::Dotted { spacing })
    }

    /// Set dashed line style with given dash length.
    pub fn line_dashed(self, length: f32) -> Self {
        self.line_type(LineType::Dashed { length })
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
