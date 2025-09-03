use glam::vec2;
use glam::{Mat4, Vec2};

const EPSILON_SMALL: f32 = 1e-6;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    // (2/width, 2/height, reserved0, reserved1) - for screen-space sizing (markers)
    pub pixel_to_clip: [f32; 4],
    // (world_units_per_pixel_x, world_units_per_pixel_y, reserved0, reserved1) - for world-space patterns (lines)
    pub pixel_to_world: [f32; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            pixel_to_clip: [0.0; 4],
            pixel_to_world: [0.0; 4],
        }
    }

    pub fn update(&mut self, camera: &Camera, viewport_width: u32, viewport_height: u32) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();

        // For screen-space sizing (markers): convert pixels to clip space
        let pixel_to_clip_x = 2.0 / viewport_width as f32;
        let pixel_to_clip_y = 2.0 / viewport_height as f32;
        self.pixel_to_clip = [pixel_to_clip_x, pixel_to_clip_y, 0.0, 0.0];

        // For world-space patterns (lines): convert pixels to world units
        let world_units_per_pixel_x = (2.0 * camera.half_extents.x) / viewport_width as f32;
        let world_units_per_pixel_y = (2.0 * camera.half_extents.y) / viewport_height as f32;
        self.pixel_to_world = [world_units_per_pixel_x, world_units_per_pixel_y, 0.0, 0.0];
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Camera {
    /// Center position (world units)
    pub position: Vec2,
    /// Half extents in world units.
    pub half_extents: Vec2,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        const INITIAL_ZOOM: f32 = 1.0;
        let aspect = width as f32 / height as f32;
        let half_height = INITIAL_ZOOM; // initial zoom = 1.0
        let half_width = aspect * half_height;
        Self {
            position: Vec2::ZERO,
            half_extents: Vec2::new(half_width, half_height),
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let proj = Mat4::orthographic_rh(
            -self.half_extents.x,
            self.half_extents.x,
            -self.half_extents.y,
            self.half_extents.y,
            -1.0,
            1.0,
        );
        let view = Mat4::from_translation(-self.position.extend(0.0));
        proj * view
    }

    // Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: Vec2, screen_size: Vec2) -> Vec2 {
        let ndc_x = (screen_pos.x / screen_size.x) * 2.0 - 1.0;
        let ndc_y = -((screen_pos.y / screen_size.y) * 2.0 - 1.0); // Flip Y
        Vec2::new(
            self.position.x + ndc_x * self.half_extents.x,
            self.position.y + ndc_y * self.half_extents.y,
        )
    }
}

impl Camera {
    pub fn set_bounds(&mut self, bounds_min: Vec2, bounds_max: Vec2, padding_frac: f32) {
        let size = (bounds_max - bounds_min).max(vec2(EPSILON_SMALL, EPSILON_SMALL));
        let size_padded = size + size * padding_frac;
        self.half_extents = size_padded / 2.0;
        self.position = (bounds_min + bounds_max) / 2.0;
    }
}
