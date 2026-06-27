// Vertex shader
const QUAD_POS: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),  // bottom-left
    vec2<f32>(1.0, -1.0),   // bottom-right
    vec2<f32>(-1.0, 1.0),   // top-left
    vec2<f32>(1.0, 1.0),    // top-right
);
const CIRCLE_RADIUS: f32 = 1.0;
const EMPTY_CIRCLE_INNER: f32 = 0.7;
const STAR_POINTS: array<vec2<f32>, 10> = array<vec2<f32>, 10>(
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.264503, 0.364057),
    vec2<f32>(0.951057, 0.309017),
    vec2<f32>(0.427975, -0.139058),
    vec2<f32>(0.587785, -0.809017),
    vec2<f32>(0.0, -0.45),
    vec2<f32>(-0.587785, -0.809017),
    vec2<f32>(-0.427975, -0.139058),
    vec2<f32>(-0.951057, 0.309017),
    vec2<f32>(-0.264503, 0.364057),
);
const MARKER_SIZE_MODE_MASK: u32 = 1u;
const MARKER_SIZE_WORLD: u32 = 1u;

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
    @location(4) marker_flags: u32,
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
    let size_mode = model.marker_flags & MARKER_SIZE_MODE_MASK;

    var center_pos = model.position;
    var half_world = 0.0;
    if (size_mode == MARKER_SIZE_WORLD) {
        half_world = model.size * 0.5;
        center_pos = center_pos + vec2<f32>(half_world, half_world);
    }
    // Center in clip space
    let center_clip = camera.view_proj * vec4<f32>(center_pos, 0.0, 1.0);

    // Interpret model.size as pixels or world units depending on size_mode
    var half_size_px_x = model.size;
    var half_size_px_y = model.size;
    if (size_mode == MARKER_SIZE_WORLD) {
        half_size_px_x = half_world / camera.pixel_to_world.x;
        half_size_px_y = half_world / camera.pixel_to_world.y;
    }
    let offset_clip = vec4<f32>(local_pos.x * half_size_px_x * camera.pixel_to_clip.x * center_clip.w,
                                local_pos.y * half_size_px_y * camera.pixel_to_clip.y * center_clip.w,
                                0.0, 0.0);
    out.clip_position = center_clip + offset_clip;
    out.color = model.color;
    out.marker_type = model.marker_type;
    out.size = model.size;
    out.local_pos = local_pos;

    return out;
}

fn segment_distance(point: vec2<f32>, start: vec2<f32>, end: vec2<f32>) -> f32 {
    let segment = end - start;
    let t = clamp(dot(point - start, segment) / dot(segment, segment), 0.0, 1.0);
    return length(point - (start + segment * t));
}

fn star_signed_distance(point: vec2<f32>) -> f32 {
    var inside = false;
    var edge_distance = 1e6;
    var previous = STAR_POINTS[9];

    for (var i = 0u; i < 10u; i = i + 1u) {
        let current = STAR_POINTS[i];
        if ((current.y > point.y) != (previous.y > point.y)) {
            let intersection_x =
                (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y)
                + current.x;
            if (point.x < intersection_x) {
                inside = !inside;
            }
        }
        edge_distance = min(edge_distance, segment_distance(point, previous, current));
        previous = current;
    }

    if inside {
        return -edge_distance;
    }
    return edge_distance;
}

fn marker_alpha(sdf: f32, width: f32) -> f32 {
    return 1.0 - smoothstep(-width, width, sdf);
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.local_pos);
    let edge_width = max(length(fwidth(in.local_pos)), 1e-4);
    var alpha = 0.0;

    // Different marker shapes
    switch in.marker_type {
        case 0u { // Filled Circle
            alpha = marker_alpha(dist - CIRCLE_RADIUS, edge_width);
        }
        case 1u { // Empty Circle (ring)
            let sdf = max(EMPTY_CIRCLE_INNER - dist, dist - CIRCLE_RADIUS);
            alpha = marker_alpha(sdf, edge_width);
        }
        case 2u { // Square
            let sdf = max(abs(in.local_pos.x), abs(in.local_pos.y)) - CIRCLE_RADIUS;
            alpha = marker_alpha(sdf, edge_width);
        }
        case 3u { // Star
            alpha = marker_alpha(star_signed_distance(in.local_pos), edge_width);
        }
        case 4u { // Triangle
            let x = in.local_pos.x;
            let y = in.local_pos.y;
            // Equilateral triangle pointing up: base from (-1, -0.866) to (1, -0.866), apex at (0, 0.866)
            // Height/base ratio of √3/2 ≈ 0.866 (truly equilateral), centered at y=0
            // Calculate the fraction from base to apex (0 at base, 1 at apex)
            let fraction = (y + 0.866) / 1.732;
            // Width decreases linearly from 2 at base to 0 at apex
            let half_width = 1.0 * (1.0 - fraction);
            let left_sdf = -half_width - x;
            let right_sdf = x - half_width;
            let bottom_sdf = -0.866 - y;
            alpha = marker_alpha(max(max(left_sdf, right_sdf), bottom_sdf), edge_width);
        }
        default {
            return vec4<f32>(in.color.rgb, 1.0);
        }
    }

    if alpha <= 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return vec4<f32>(in.color.rgb, alpha);
}
