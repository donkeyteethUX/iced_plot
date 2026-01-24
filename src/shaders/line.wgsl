struct CameraUniform {
    view_proj: mat4x4<f32>,
    pixel_to_clip: vec4<f32>, // (2/width, 2/height, _, _) - for screen-space sizing
    pixel_to_world: vec4<f32>, // (world_per_pixel_x, world_per_pixel_y, _, _) - for world-space patterns
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) line_style: u32, // 0=solid, 1=dotted, 2=dashed
    @location(3) distance_along_line: f32, // cumulative distance along the line
    @location(4) style_param: f32, // spacing for dotted, length for dashed
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @interpolate(flat) @location(1) line_style: u32,
    @location(2) distance_along_line: f32,
    @location(3) style_param: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = camera.view_proj * vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    out.line_style = in.line_style;
    out.distance_along_line = in.distance_along_line;
    out.style_param = in.style_param;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var alpha = 1.0;
    
    // Convert style parameter from logical pixels to world coordinates
    // camera.pixel_to_world.x contains the world size of one pixel
    let pixel_to_world = camera.pixel_to_world.x;
    
    if in.line_style == 1u { // Dotted
        // Convert logical pixels to world units
        let spacing_world = in.style_param * camera.pixel_to_world.x;
        let pattern_length = spacing_world * 2.0;
        let t = fract(in.distance_along_line / pattern_length);
        // Create dots: visible for first half of pattern, invisible for second half
        if t > 0.5 {
            alpha = 0.0;
        }
    } else if in.line_style == 2u { // Dashed
        // Convert logical pixels to world units
        let dash_length_world = in.style_param * camera.pixel_to_world.x;
        let gap_length_world = dash_length_world * 0.5; // Gap is half the dash length
        let pattern_length = dash_length_world + gap_length_world;
        let t = fract(in.distance_along_line / pattern_length);
        // Create dashes: visible for first part of pattern (dash), invisible for gap
        if t > (dash_length_world / pattern_length) {
            alpha = 0.0;
        }
    }
    // else: solid line (line_style == 0u), alpha remains 1.0
    
    if alpha < 0.1 {
        discard;
    }
    
    return vec4<f32>(in.color.rgb, alpha);
}
