use glam::Vec2;
use iced::wgpu::{util::DeviceExt, *};

use crate::camera::Camera;

pub struct Grid {
    pipeline: Option<RenderPipeline>,
    vertex_buffer: Option<Buffer>,
    vertex_count: u32,
    last_center: Vec2,
    last_extents: Vec2,
}

impl Grid {
    pub fn new() -> Self {
        Self {
            pipeline: None,
            vertex_buffer: None,
            vertex_count: 0,
            last_center: Vec2::splat(f32::NAN),
            last_extents: Vec2::splat(f32::NAN),
        }
    }

    pub fn ensure_pipeline(
        &mut self,
        device: &Device,
        format: TextureFormat,
        camera_bgl: &BindGroupLayout,
    ) {
        if self.pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(include_wgsl!("shaders/grid.wgsl"));
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[camera_bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<f32>())
                        as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as u64,
                            shader_location: 1,
                            format: VertexFormat::Float32,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        self.pipeline = Some(pipeline);
    }

    pub fn update(&mut self, device: &Device, camera: &Camera) {
        const GRID_TARGET_LINES: f32 = 20.0;
        const GRID_MAX_LINES: u32 = 1000;
        const GRID_MAJOR_ALPHA: f32 = 0.45;
        const GRID_MINOR_ALPHA: f32 = 0.28;
        const GRID_SUB_MINOR_ALPHA: f32 = 0.10;
        const GRID_EPSILON: f32 = 1e-6;
        const GRID_MAJOR_INTERVAL: i64 = 10;
        const GRID_MINOR_INTERVAL: i64 = 5;
        if camera.position == self.last_center && camera.half_extents == self.last_extents {
            return;
        }
        self.last_center = camera.position;
        self.last_extents = camera.half_extents;

        let target = GRID_TARGET_LINES;
        let span_x = camera.half_extents.x * 2.0;
        let span_y = camera.half_extents.y * 2.0;
        let step_x = nice_step(span_x / target);
        let step_y = nice_step(span_y / target);
        let min_x = camera.position.x - camera.half_extents.x;
        let max_x = camera.position.x + camera.half_extents.x;
        let min_y = camera.position.y - camera.half_extents.y;
        let max_y = camera.position.y + camera.half_extents.y;
        let start_x = (min_x / step_x).floor() * step_x;
        let start_y = (min_y / step_y).floor() * step_y;
        let mut verts: Vec<f32> = Vec::new();
        let mut count = 0u32;
        let max_lines = GRID_MAX_LINES; // safety
        // Vertical
        let mut x = start_x;
        while x <= max_x + GRID_EPSILON && count < max_lines {
            let idx = (x / step_x).round() as i64;
            let alpha = if idx % GRID_MAJOR_INTERVAL == 0 {
                GRID_MAJOR_ALPHA
            } else if idx % GRID_MINOR_INTERVAL == 0 {
                GRID_MINOR_ALPHA
            } else {
                GRID_SUB_MINOR_ALPHA
            }; // super-major > major > minor
            verts.extend_from_slice(&[x, min_y, alpha]);
            verts.extend_from_slice(&[x, max_y, alpha]);
            count += 2;
            x += step_x;
        }
        // Horizontal
        let mut y = start_y;
        while y <= max_y + GRID_EPSILON && count < max_lines {
            let idx = (y / step_y).round() as i64;
            let alpha = if idx % GRID_MAJOR_INTERVAL == 0 {
                GRID_MAJOR_ALPHA
            } else if idx % GRID_MINOR_INTERVAL == 0 {
                GRID_MINOR_ALPHA
            } else {
                GRID_SUB_MINOR_ALPHA
            };
            verts.extend_from_slice(&[min_x, y, alpha]);
            verts.extend_from_slice(&[max_x, y, alpha]);
            count += 2;
            y += step_y;
        }
        let raw = bytemuck::cast_slice(&verts);
        self.vertex_buffer = Some(device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Grid VB"),
            contents: raw,
            usage: BufferUsages::VERTEX,
        }));
        self.vertex_count = count;
    }

    pub fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
        if let (Some(pipeline), Some(vb)) = (&self.pipeline, &self.vertex_buffer) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera_bind_group, &[]);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new()
    }
}

fn nice_step(raw: f32) -> f32 {
    const NICE_STEP_BASES: [f32; 4] = [1.0, 2.0, 5.0, 10.0];
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10.0_f32.powf(exp);
    for &m in &NICE_STEP_BASES {
        if raw <= m * base {
            return m * base;
        }
    }
    base * 10.0
}
