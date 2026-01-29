// Render markers to a uint id target. Each instance writes unique id = instance_index + 1.

const QUAD_POS: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),  // bottom-left
    vec2<f32>(1.0, -1.0),   // bottom-right
    vec2<f32>(-1.0, 1.0),   // top-left
    vec2<f32>(1.0, 1.0),    // top-right
);

struct CameraUniform {
    view_proj: mat4x4<f32>,
    pixel_to_clip: vec4<f32>,
    pixel_to_world: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    // We don't care about the marker type which is at location 2.
    @location(3) size: f32,
    @location(4) size_mode: u32,
};

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @interpolate(flat) @location(1) instance_id: u32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vid: u32,
    @builtin(instance_index) iid: u32,
    model: VertexInput,
) -> VsOut {
    var out: VsOut;
    let local = QUAD_POS[vid];
    var center_pos = model.position;
    var half_world = 0.0;
    if (model.size_mode == 1u) {
        half_world = model.size * 0.5;
        center_pos = center_pos + vec2<f32>(half_world, half_world);
    }
    let center = camera.view_proj * vec4<f32>(center_pos, 0.0, 1.0);
    var half_size_px_x = model.size;
    var half_size_px_y = model.size;
    if (model.size_mode == 1u) {
        half_size_px_x = half_world / camera.pixel_to_world.x;
        half_size_px_y = half_world / camera.pixel_to_world.y;
    }
    let offset = vec4<f32>(
        local.x * half_size_px_x * camera.pixel_to_clip.x * center.w,
        local.y * half_size_px_y * camera.pixel_to_clip.y * center.w,
        0.0, 0.0);
    out.clip_position = center + offset;
    out.local_pos = local;
    out.instance_id = iid;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) u32 {
    if length(in.local_pos) <= 1.0 {     
        // 1-based id so 0 means background
        return in.instance_id + 1u;
    }
    return 0u;
}
