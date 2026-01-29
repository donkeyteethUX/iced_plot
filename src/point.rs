#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Marker types for series points.
///
/// Determines the shape drawn for each data point in a series.
pub enum MarkerType {
    /// A filled circle.
    FilledCircle = 0,
    /// An empty circle (ring).
    EmptyCircle = 1,
    /// A square.
    Square = 2,
    /// A star shape.
    Star = 3,
    /// A triangle.
    Triangle = 4,
}

pub(crate) const MARKER_SIZE_PIXELS: u32 = 0;
pub(crate) const MARKER_SIZE_WORLD: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// A point in data-space with a visual size.
///
/// Represents a single data point to be rendered, with position and visual sizing.
pub struct Point {
    /// Position in data coordinates [x, y].
    pub position: [f64; 2],
    /// Visual size value (pixels or world units depending on mode).
    pub size: f32,
    /// 0 = pixels, 1 = world units.
    pub size_mode: u32,
}

impl Point {
    pub fn new(x: f64, y: f64, size: f32) -> Self {
        Self {
            position: [x, y],
            size,
            size_mode: MARKER_SIZE_PIXELS,
        }
    }

    pub fn new_world(x: f64, y: f64, size: f64) -> Self {
        Self {
            position: [x, y],
            size: size as f32,
            size_mode: MARKER_SIZE_WORLD,
        }
    }

    pub fn filled_circle(x: f64, y: f64, size: f32) -> Self {
        Self::new(x, y, size)
    }

    pub fn empty_circle(x: f64, y: f64, size: f32) -> Self {
        Self::new(x, y, size)
    }

    pub fn square(x: f64, y: f64, size: f32) -> Self {
        Self::new(x, y, size)
    }

    pub fn star(x: f64, y: f64, size: f32) -> Self {
        Self::new(x, y, size)
    }

    pub fn triangle(x: f64, y: f64, size: f32) -> Self {
        Self::new(x, y, size)
    }

    pub fn filled_circle_world(x: f64, y: f64, size: f64) -> Self {
        Self::new_world(x, y, size)
    }

    pub fn empty_circle_world(x: f64, y: f64, size: f64) -> Self {
        Self::new_world(x, y, size)
    }

    pub fn square_world(x: f64, y: f64, size: f64) -> Self {
        Self::new_world(x, y, size)
    }

    pub fn star_world(x: f64, y: f64, size: f64) -> Self {
        Self::new_world(x, y, size)
    }

    pub fn triangle_world(x: f64, y: f64, size: f64) -> Self {
        Self::new_world(x, y, size)
    }
}
