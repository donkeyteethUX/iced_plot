use glam::{DVec2, Mat4, Vec3};

const EPSILON_SMALL: f64 = 1e-6;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    // (2/width, 2/height, reserved0, reserved1) - for screen-space sizing (markers)
    pub pixel_to_clip: [f32; 4],
    // (world_units_per_pixel_x, world_units_per_pixel_y, reserved0, reserved1) - for world-space patterns (lines)
    pub pixel_to_world: [f32; 4],
}

impl CameraUniform {
    pub(crate) fn update(&mut self, camera: &Camera, viewport_width: u32, viewport_height: u32) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();

        // For screen-space sizing (markers): convert pixels to clip space
        let pixel_to_clip_x = 2.0 / viewport_width as f32;
        let pixel_to_clip_y = 2.0 / viewport_height as f32;
        self.pixel_to_clip = [pixel_to_clip_x, pixel_to_clip_y, 0.0, 0.0];

        // For world-space patterns (lines): convert pixels to world units
        let world_units_per_pixel_x = (2.0 * camera.half_extents.x) / viewport_width as f64;
        let world_units_per_pixel_y = (2.0 * camera.half_extents.y) / viewport_height as f64;
        self.pixel_to_world = [
            world_units_per_pixel_x as f32,
            world_units_per_pixel_y as f32,
            0.0,
            0.0,
        ];
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            pixel_to_clip: [0.0; 4],
            pixel_to_world: [0.0; 4],
        }
    }
}

#[derive(Debug, Clone, Default, Copy, PartialEq)]
/// Camera for viewing the plot in world-space.
///
/// Manages the viewport transformation between world coordinates (data space),
/// render coordinates, and screen coordinates. Supports panning and zooming.
pub(crate) struct Camera {
    /// Center position (world units)
    pub position: DVec2,
    /// Half extents in world units.
    pub half_extents: DVec2,
    /// Offset subtracted from world coordinates before rendering (for precision)
    pub render_offset: DVec2,
}

impl Camera {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        const INITIAL_ZOOM: f64 = 1.0;
        let aspect = width as f64 / height as f64;
        let half_height = INITIAL_ZOOM;
        let half_width = aspect * half_height;
        Self {
            position: DVec2::ZERO,
            half_extents: DVec2::new(half_width, half_height),
            render_offset: DVec2::ZERO,
        }
    }

    pub(crate) fn build_view_projection_matrix(&self) -> Mat4 {
        let proj = Mat4::orthographic_rh(
            -self.half_extents.x as f32,
            self.half_extents.x as f32,
            -self.half_extents.y as f32,
            self.half_extents.y as f32,
            -1.0,
            1.0,
        );
        // Subtract render_offset from camera position for high-precision rendering
        let effective_position = self.position - self.render_offset;
        let view = Mat4::from_translation(-Vec3::new(
            effective_position.x as f32,
            effective_position.y as f32,
            0.0,
        ));
        proj * view
    }

    // Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: DVec2, screen_size: DVec2) -> DVec2 {
        let ndc_x = (screen_pos.x / screen_size.x) * 2.0 - 1.0;
        let ndc_y = -((screen_pos.y / screen_size.y) * 2.0 - 1.0); // Flip Y
        // Calculate position in render space first, then convert to world space
        let render_pos = DVec2::new(
            self.effective_position().x + ndc_x * self.half_extents.x,
            self.effective_position().y + ndc_y * self.half_extents.y,
        );
        render_pos + self.render_offset
    }

    /// Get the effective camera position relative to the render offset
    pub fn effective_position(&self) -> DVec2 {
        self.position - self.render_offset
    }

    /// Convert screen coordinates to render coordinates (without offset)
    pub fn screen_to_render(&self, screen_pos: DVec2, screen_size: DVec2) -> DVec2 {
        let ndc_x = (screen_pos.x / screen_size.x) * 2.0 - 1.0;
        let ndc_y = -((screen_pos.y / screen_size.y) * 2.0 - 1.0); // Flip Y
        DVec2::new(
            self.effective_position().x + ndc_x * self.half_extents.x,
            self.effective_position().y + ndc_y * self.half_extents.y,
        )
    }

    /// Set camera bounds without changing the render offset
    pub fn set_bounds_preserve_offset(
        &mut self,
        bounds_min: DVec2,
        bounds_max: DVec2,
        padding_frac: f64,
    ) {
        let size = (bounds_max - bounds_min).max(DVec2::splat(EPSILON_SMALL));
        let size_padded = size + size * padding_frac;
        self.half_extents = size_padded / 2.0;
        let center = (bounds_min + bounds_max) / 2.0;
        self.position = center;
        // Keep the existing render_offset
    }

    pub(crate) fn set_bounds(&mut self, bounds_min: DVec2, bounds_max: DVec2, padding_frac: f64) {
        let size = (bounds_max - bounds_min).max(DVec2::splat(EPSILON_SMALL));
        let size_padded = size + size * padding_frac;
        self.half_extents = size_padded / 2.0;
        let center = (bounds_min + bounds_max) / 2.0;
        self.position = center;
        // Set render_offset to center for high-precision rendering near zero
        self.render_offset = center;
    }
}
