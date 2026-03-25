struct CameraUniform {
    view_proj: mat4x4<f32>,
    pixel_to_clip: vec4<f32>, // (2/width, 2/height, _, _) - for screen-space sizing
    pixel_to_world: vec4<f32>, // (world_per_pixel_x, world_per_pixel_y, _, _) - for world-space sizing and patterns
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VsIn {
    @location(0) segment_start: vec2<f32>,
    @location(1) segment_end: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) line_style: u32, // 0=solid, 1=dotted, 2=dashed
    @location(4) distance_start: f32, // cumulative distance at the segment start
    @location(5) segment_length_world: f32,
    @location(6) style_param: f32, // spacing for dotted, length for dashed
    @location(7) width: f32,
    @location(8) width_mode: u32,
    @location(9) along: f32, // 0=start edge, 1=end edge
    @location(10) side: f32, // -1 or +1
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @interpolate(flat) @location(1) line_style: u32,
    @location(2) distance_start: f32,
    @location(3) segment_length_world: f32,
    @location(4) style_param: f32,
    @location(5) local_x_px: f32,
    @location(6) local_y_px: f32,
    @location(7) half_width_px: f32,
    @location(8) segment_length_px: f32,
};

const LINE_AA_RADIUS_PX: f32 = 1.0;
const PIXEL_TO_WORLD_MIN: f32 = 1e-12;

fn perp(v: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(-v.y, v.x);
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;

    let pixel_to_world = vec2<f32>(
        max(camera.pixel_to_world.x, PIXEL_TO_WORLD_MIN),
        max(camera.pixel_to_world.y, PIXEL_TO_WORLD_MIN),
    );
    let start_px = in.segment_start / pixel_to_world;
    let end_px = in.segment_end / pixel_to_world;
    let delta_px = end_px - start_px;
    let segment_length_px = max(length(delta_px), 1e-6);
    let tangent_px = delta_px / segment_length_px;
    let normal_px = perp(tangent_px);

    var half_width_px: f32;
    if in.width_mode == 0u {
        half_width_px = max(in.width, 0.5) * 0.5;
    } else {
        half_width_px = max(in.width / max(pixel_to_world.x, pixel_to_world.y), 0.5) * 0.5;
    }
    let outer_half_width_px = half_width_px + LINE_AA_RADIUS_PX;

    let local_x_px = select(
        -outer_half_width_px,
        segment_length_px + outer_half_width_px,
        in.along > 0.5,
    );
    let local_y_px = in.side * outer_half_width_px;

    let offset_px = tangent_px * local_x_px + normal_px * local_y_px;
    let start_clip = camera.view_proj * vec4<f32>(in.segment_start, 0.0, 1.0);
    let offset_ndc = vec2<f32>(
        offset_px.x * camera.pixel_to_clip.x,
        offset_px.y * camera.pixel_to_clip.y,
    );

    out.clip = vec4<f32>(
        start_clip.xy + offset_ndc * start_clip.w,
        start_clip.z,
        start_clip.w,
    );
    out.color = in.color;
    out.line_style = in.line_style;
    out.distance_start = in.distance_start;
    out.segment_length_world = in.segment_length_world;
    out.style_param = in.style_param;
    out.local_x_px = local_x_px;
    out.local_y_px = local_y_px;
    out.half_width_px = half_width_px;
    out.segment_length_px = segment_length_px;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let clamped_x_px = clamp(in.local_x_px, 0.0, in.segment_length_px);
    let cap_dx_px = in.local_x_px - clamped_x_px;
    let distance_to_stroke_px = length(vec2<f32>(cap_dx_px, in.local_y_px));
    let edge_alpha = clamp(in.half_width_px + 0.5 - distance_to_stroke_px, 0.0, 1.0);

    let pixel_to_world = max(camera.pixel_to_world.x, PIXEL_TO_WORLD_MIN);
    let distance_along_line = in.distance_start
        + clamped_x_px * (in.segment_length_world / max(in.segment_length_px, 1e-6));

    var alpha = 1.0;

    if in.line_style == 1u {
        let spacing_world = in.style_param * pixel_to_world;
        let pattern_length = spacing_world * 2.0;
        let t = fract(distance_along_line / pattern_length);
        if t > 0.5 {
            alpha = 0.0;
        }
    } else if in.line_style == 2u {
        let dash_length_world = in.style_param * pixel_to_world;
        let gap_length_world = dash_length_world * 0.5;
        let pattern_length = dash_length_world + gap_length_world;
        let t = fract(distance_along_line / pattern_length);
        if t > (dash_length_world / pattern_length) {
            alpha = 0.0;
        }
    }

    alpha *= edge_alpha;

    if alpha < 1e-3 {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
