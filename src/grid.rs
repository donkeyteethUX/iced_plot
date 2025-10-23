use glam::DVec2;
use iced::wgpu::{util::DeviceExt, *};

use crate::plot_state::PlotState;

pub(crate) struct Grid {
    pipeline: Option<RenderPipeline>,
    vertex_buffer: Option<Buffer>,
    vertex_count: u32,
    last_center: DVec2,
    last_extents: DVec2,
}

/// The visual weight of a tick / grid line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickWeight {
    Major,
    Minor,
    SubMinor,
}

impl Grid {
    pub(crate) fn ensure_pipeline(
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

    pub(crate) fn update(&mut self, device: &Device, state: &PlotState) {
        const GRID_MAJOR_ALPHA: f32 = 0.45;
        const GRID_MINOR_ALPHA: f32 = 0.28;
        const GRID_SUB_MINOR_ALPHA: f32 = 0.10;

        let camera = &state.camera;

        if camera.position == self.last_center && camera.half_extents == self.last_extents {
            return;
        }

        self.last_center = camera.position;
        self.last_extents = camera.half_extents;

        // Calculate bounds in render space (world - offset) for line endpoints
        let render_center = camera.effective_position();
        let min_x = render_center.x - camera.half_extents.x;
        let max_x = render_center.x + camera.half_extents.x;
        let min_y = render_center.y - camera.half_extents.y;
        let max_y = render_center.y + camera.half_extents.y;

        let mut verts = Vec::new();
        let mut count = 0u32;

        // Build vertical lines from precomputed x ticks
        for positioned_tick in &state.x_ticks {
            let render_x = positioned_tick.tick.value - camera.render_offset.x;
            let alpha = match positioned_tick.tick.line_type {
                TickWeight::Major => GRID_MAJOR_ALPHA,
                TickWeight::Minor => GRID_MINOR_ALPHA,
                TickWeight::SubMinor => GRID_SUB_MINOR_ALPHA,
            };
            verts.extend_from_slice(&[render_x as f32, min_y as f32, alpha]);
            verts.extend_from_slice(&[render_x as f32, max_y as f32, alpha]);
            count += 2;
        }

        // Build horizontal lines from precomputed y ticks
        for positioned_tick in &state.y_ticks {
            let render_y = positioned_tick.tick.value - camera.render_offset.y;
            let alpha = match positioned_tick.tick.line_type {
                TickWeight::Major => GRID_MAJOR_ALPHA,
                TickWeight::Minor => GRID_MINOR_ALPHA,
                TickWeight::SubMinor => GRID_SUB_MINOR_ALPHA,
            };
            verts.extend_from_slice(&[min_x as f32, render_y as f32, alpha]);
            verts.extend_from_slice(&[max_x as f32, render_y as f32, alpha]);
            count += 2;
        }

        self.vertex_count = count;
        self.vertex_buffer = Some(device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("Grid VB"),
            contents: bytemuck::cast_slice(&verts),
            usage: BufferUsages::VERTEX,
        }));
    }

    pub(crate) fn draw<'a>(&'a self, pass: &mut RenderPass<'a>, camera_bind_group: &'a BindGroup) {
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
        Self {
            pipeline: None,
            vertex_buffer: None,
            vertex_count: 0,
            last_center: DVec2::splat(f64::NAN),
            last_extents: DVec2::splat(f64::NAN),
        }
    }
}
