struct CameraUniform {
    view_proj: mat4x4<f32>,
    pixel_to_clip: vec4<f32>, // (2/width, 2/height, _, _) - for screen-space sizing
    pixel_to_world: vec4<f32>, // (world_per_pixel_x, world_per_pixel_y, _, _) - for world-space sizing and patterns
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) prev_position: vec2<f32>,
    @location(2) next_position: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) line_style: u32, // 0=solid, 1=dotted, 2=dashed
    @location(5) distance_along_line: f32, // cumulative distance along the line
    @location(6) style_param: f32, // spacing for dotted, length for dashed
    @location(7) width: f32,
    @location(8) width_mode: u32,
    @location(9) side: f32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @interpolate(flat) @location(1) line_style: u32,
    @location(2) distance_along_line: f32,
    @location(3) style_param: f32,
};

fn safe_normalize(v: vec2<f32>) -> vec2<f32> {
    let len = length(v);
    if len <= 1e-6 {
        return vec2<f32>(0.0, 0.0);
    }
    return v / len;
}

fn perp(v: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(-v.y, v.x);
}

fn join_offset(
    current: vec2<f32>,
    previous: vec2<f32>,
    next: vec2<f32>,
    side: f32,
    half_width: f32,
) -> vec2<f32> {
    let prev_delta = current - previous;
    let next_delta = next - current;
    let has_prev = length(prev_delta) > 1e-6;
    let has_next = length(next_delta) > 1e-6;

    if has_prev && has_next {
        let dir_prev = safe_normalize(prev_delta);
        let dir_next = safe_normalize(next_delta);
        let normal_prev = perp(dir_prev) * side;
        let normal_next = perp(dir_next) * side;
        let miter_sum = normal_prev + normal_next;

        if length(miter_sum) > 1e-4 {
            let miter = normalize(miter_sum);
            let denom = dot(miter, normal_next);
            if abs(denom) > 1e-3 {
                let miter_length = min(half_width / abs(denom), half_width * 4.0);
                return miter * miter_length;
            }
        }

        return normal_next * half_width;
    }

    if has_next {
        return perp(safe_normalize(next_delta)) * side * half_width;
    }

    if has_prev {
        return perp(safe_normalize(prev_delta)) * side * half_width;
    }

    return vec2<f32>(0.0, 0.0);
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let current_clip = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);

    if in.width_mode == 0u {
        let pixel_to_world = vec2<f32>(
            max(camera.pixel_to_world.x, 1e-6),
            max(camera.pixel_to_world.y, 1e-6),
        );
        let current_px = in.position / pixel_to_world;
        let previous_px = in.prev_position / pixel_to_world;
        let next_px = in.next_position / pixel_to_world;
        let offset_px = join_offset(
            current_px,
            previous_px,
            next_px,
            in.side,
            max(in.width, 0.5) * 0.5,
        );
        let offset_ndc = vec2<f32>(
            offset_px.x * camera.pixel_to_clip.x,
            offset_px.y * camera.pixel_to_clip.y,
        );
        out.clip = vec4<f32>(
            current_clip.xy + offset_ndc * current_clip.w,
            current_clip.z,
            current_clip.w,
        );
    } else {
        let offset_world = join_offset(
            in.position,
            in.prev_position,
            in.next_position,
            in.side,
            max(in.width, 1e-6) * 0.5,
        );
        out.clip = camera.view_proj * vec4<f32>(in.position + offset_world, 0.0, 1.0);
    }

    out.color = in.color;
    out.line_style = in.line_style;
    out.distance_along_line = in.distance_along_line;
    out.style_param = in.style_param;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var alpha = 1.0;

    // Convert style parameter from logical pixels to world coordinates.
    // camera.pixel_to_world.x contains the world size of one pixel.
    let pixel_to_world = max(camera.pixel_to_world.x, 1e-6);

    if in.line_style == 1u {
        // Convert logical pixels to world units.
        let spacing_world = in.style_param * pixel_to_world;
        let pattern_length = spacing_world * 2.0;
        let t = fract(in.distance_along_line / pattern_length);
        // Create dots: visible for first half of pattern, invisible for second half.
        if t > 0.5 {
            alpha = 0.0;
        }
    } else if in.line_style == 2u {
        // Convert logical pixels to world units.
        let dash_length_world = in.style_param * pixel_to_world;
        let gap_length_world = dash_length_world * 0.5;
        let pattern_length = dash_length_world + gap_length_world;
        let t = fract(in.distance_along_line / pattern_length);
        // Create dashes: visible for first part of pattern, invisible for the gap.
        if t > (dash_length_world / pattern_length) {
            alpha = 0.0;
        }
    }
    // else: solid line (line_style == 0u), alpha remains 1.0

    if alpha < 0.1 {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
