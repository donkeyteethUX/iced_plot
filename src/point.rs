#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerType {
    FilledCircle = 0,
    EmptyCircle = 1,
    Square = 2,
    Star = 3,
    Triangle = 4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub position: [f64; 2],
    /// Marker size interpreted as PIXEL RADIUS (screen-space, invariant to zoom)
    pub size: f32,
}

impl Point {
    pub fn new(x: f64, y: f64, size: f32) -> Self {
        Self {
            position: [x, y],
            size,
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
}
