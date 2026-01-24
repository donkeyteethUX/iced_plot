// Vertex shader
const QUAD_POS: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),  // bottom-left
    vec2<f32>(1.0, -1.0),   // bottom-right
    vec2<f32>(-1.0, 1.0),   // top-left
    vec2<f32>(1.0, 1.0),    // top-right
);
const CIRCLE_RADIUS: f32 = 1.0;
const EMPTY_CIRCLE_INNER: f32 = 0.7;
const STAR_ANGLE_MULT: f32 = 5.0;
const STAR_INNER_SCALE: f32 = 0.3;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pixel_to_clip: vec4<f32>, // (2/width, 2/height, _, _) - for screen-space sizing
    pixel_to_world: vec4<f32>, // (world_per_pixel_x, world_per_pixel_y, _, _) - for world-space patterns
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) marker_type: u32,
    @location(3) size: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @interpolate(flat) @location(1) marker_type: u32,
    @location(2) size: f32,
    @location(3) local_pos: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Generate quad vertices for each marker
    let local_pos = QUAD_POS[vertex_index];

    // Center in clip space
    let center_clip = camera.view_proj * vec4<f32>(model.position, 0.0, 1.0);
    // Perspective divide later handled by hardware; apply pixel offsets in clip units scaled by w.
    // Interpret model.size as pixel radius directly
    let half_size_px = model.size;
    let offset_clip = vec4<f32>(local_pos.x * half_size_px * camera.pixel_to_clip.x * center_clip.w,
                                local_pos.y * half_size_px * camera.pixel_to_clip.y * center_clip.w,
                                0.0, 0.0);
    out.clip_position = center_clip + offset_clip;
    out.color = model.color;
    out.marker_type = model.marker_type;
    out.size = model.size;
    out.local_pos = local_pos;

    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.local_pos);

    // Different marker shapes
    switch in.marker_type {
        case 0u { // Filled Circle
            if dist <= CIRCLE_RADIUS {
                return vec4<f32>(in.color.rgb, 1.0);
            }
        }
        case 1u { // Empty Circle (ring)
            if dist >= EMPTY_CIRCLE_INNER && dist <= CIRCLE_RADIUS {
                return vec4<f32>(in.color.rgb, 1.0);
            }
        }
        case 2u { // Square
            if abs(in.local_pos.x) <= CIRCLE_RADIUS && abs(in.local_pos.y) <= CIRCLE_RADIUS {
                return vec4<f32>(in.color.rgb, 1.0);
            }
        }
        case 3u { // Star
            let angle = atan2(in.local_pos.y, in.local_pos.x);
            let star_dist = CIRCLE_RADIUS - STAR_INNER_SCALE * abs(sin(angle * STAR_ANGLE_MULT));
            if dist <= star_dist {
                return vec4<f32>(in.color.rgb, 1.0);
            }
        }
        case 4u { // Triangle
            let x = in.local_pos.x;
            let y = in.local_pos.y;
            // Equilateral triangle pointing up: base from (-1, -0.866) to (1, -0.866), apex at (0, 0.866)
            // Height/base ratio of √3/2 ≈ 0.866 (truly equilateral), centered at y=0
            if y >= -0.866 && y <= 0.866 {
                // Calculate the fraction from base to apex (0 at base, 1 at apex)
                let fraction = (y + 0.866) / 1.732;
                // Width decreases linearly from 2 at base to 0 at apex
                let half_width = 1.0 * (1.0 - fraction);
                let left_bound = -half_width;
                let right_bound = half_width;
                if x >= left_bound && x <= right_bound {
                    return vec4<f32>(in.color.rgb, 1.0);
                }
            }
        }
        default {
            return vec4<f32>(in.color.rgb, 1.0);
        }
    }

    // Transparent for areas outside the marker
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
