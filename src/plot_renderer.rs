//! GPU renderer for PlotWidget.
use crate::LineStyle;
use crate::picking::PickingPass;
use crate::{camera::CameraUniform, grid::Grid, widget::PlotState};
use iced::widget::shader::Viewport;
use iced::{Rectangle, wgpu::*};

pub struct RenderParams<'a> {
    pub encoder: &'a mut CommandEncoder,
    pub target: &'a TextureView,
    pub bounds: Rectangle<u32>,
}

#[derive(Default, Clone)]
struct LineSegment {
    first_vertex: u32,
    vertex_count: u32,
}

pub struct PlotRenderer {
    format: TextureFormat,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    camera_bgl: BindGroupLayout,
    // Pipelines
    marker_pipeline: Option<RenderPipeline>,
    line_pipeline: Option<RenderPipeline>,
    overlay_pipeline: Option<RenderPipeline>,
    line_overlay_pipeline: Option<RenderPipeline>,
    // Buffers
    marker_vertex_buffer: Option<Buffer>,
    marker_instances: u32,
    line_vertex_buffer: Option<Buffer>,
    line_segments: Vec<LineSegment>,
    selection_vertex_buffer: Option<Buffer>,
    selection_vertex_count: u32,
    hover_vertex_buffer: Option<Buffer>,
    hover_vertex_count: u32,
    crosshairs_vertex_buffer: Option<Buffer>,
    crosshairs_vertex_count: u32,
    // Support objects
    grid: Grid,
    picking: PickingPass,
    scale_factor: f32,
    bounds_w: u32,
    bounds_h: u32,
    // Cached versions
    last_markers_version: u64,
    last_lines_version: u64,
    last_render_offset: glam::DVec2,
}

impl PlotRenderer {
    pub fn new(device: &Device, _queue: &Queue, format: TextureFormat) -> Self {
        let camera_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera_bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("camera_buffer"),
            size: std::mem::size_of::<crate::camera::CameraUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("camera_bg"),
            layout: &camera_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        Self {
            format,
            camera_buffer,
            camera_bind_group,
            camera_bgl,
            marker_pipeline: None,
            line_pipeline: None,
            overlay_pipeline: None,
            line_overlay_pipeline: None,
            marker_vertex_buffer: None,
            marker_instances: 0,
            line_vertex_buffer: None,
            line_segments: Vec::new(),
            selection_vertex_buffer: None,
            selection_vertex_count: 0,
            hover_vertex_buffer: None,
            hover_vertex_count: 0,
            crosshairs_vertex_buffer: None,
            crosshairs_vertex_count: 0,
            grid: Grid::default(),
            picking: PickingPass::default(),
            bounds_w: 0,
            bounds_h: 0,
            scale_factor: 1.0,
            last_markers_version: 0,
            last_lines_version: 0,
            last_render_offset: glam::DVec2::ZERO,
        }
    }

    fn ensure_pipelines_and_update_grid(
        &mut self,
        device: &Device,
        _queue: &Queue,
        state: &PlotState,
    ) {
        self.ensure_marker_pipeline(device);
        self.grid
            .ensure_pipeline(device, self.format, &self.camera_bgl);
        self.grid.update(device, &state.camera);
        if !state.series.is_empty() && state.series.iter().any(|s| s.line_style.is_some()) {
            self.ensure_line_pipeline(device);
        }
        self.ensure_overlay_pipeline(device);
        self.ensure_line_overlay_pipeline(device);
    }
    fn set_bounds(&mut self, w: u32, h: u32) {
        self.bounds_w = w;
        self.bounds_h = h;
    }
    fn set_scale_factor(&mut self, scale: f32) {
        self.scale_factor = scale;
    }

    fn sync(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        // Check if render offset changed - if so, we need to rebuild vertex buffers
        // since positions are stored relative to render_offset
        let offset_changed = self.last_render_offset != state.camera.render_offset;

        if state.markers_version != self.last_markers_version || offset_changed {
            self.rebuild_markers(device, queue, state);
            self.last_markers_version = state.markers_version;
        }
        if state.lines_version != self.last_lines_version || offset_changed {
            self.rebuild_lines(device, queue, state);
            self.last_lines_version = state.lines_version;
        }

        // Update cached render offset
        self.last_render_offset = state.camera.render_offset;

        // Selection is rebuilt whenever it's active.
        self.rebuild_selection(device, queue, state);

        // Hover halo is rebuilt every frame from state.hovered_world when present.
        self.rebuild_hover(device, queue, state);

        // Crosshairs are rebuilt every frame when enabled.
        self.rebuild_crosshairs(device, queue, state);
    }

    /// Prepare the renderer for a new frame given the viewport and current plot state.
    /// This sets format/viewport/scale, ensures pipelines and grid, and syncs buffers.
    pub(crate) fn prepare_frame(
        &mut self,
        device: &Device,
        queue: &Queue,
        viewport: &Viewport,
        bounds: &Rectangle,
        state: &PlotState,
    ) {
        let scale_factor = viewport.scale_factor();
        let bounds_width = (bounds.width * scale_factor) as u32;
        let bounds_height = (bounds.height * scale_factor) as u32;

        self.set_bounds(bounds_width, bounds_height);
        self.set_scale_factor(scale_factor);

        // Sync picking viewport
        self.picking
            .set_view(bounds_width, bounds_height, scale_factor);

        // Ensure pipelines/grid and synchronize GPU buffers
        self.ensure_pipelines_and_update_grid(device, queue, state);

        // Upload camera uniform based on current camera and bounds dimensions
        let mut cam_u = CameraUniform::default();
        cam_u.update(&state.camera, bounds_width, bounds_height);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&cam_u));
        self.sync(device, queue, state);
    }

    pub(crate) fn service_picking(
        &mut self,
        instance_id: u64,
        device: &Device,
        queue: &Queue,
        state: &PlotState,
    ) {
        self.picking.service(
            instance_id,
            device,
            queue,
            &self.camera_bind_group,
            &self.camera_bgl,
            self.marker_vertex_buffer.as_ref(),
            self.marker_instances,
            state,
        );
    }

    pub fn ensure_marker_pipeline(&mut self, device: &Device) {
        if self.marker_pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(include_wgsl!("shaders/markers.wgsl"));
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("markers layout"),
            bind_group_layouts: &[&self.camera_bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("markers pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    // Explicit 32-byte stride: vec2<f32> position (8) + vec4<f32> color (16)
                    // + u32 marker (4) + f32 size (4) = 32
                    array_stride: 32u64,
                    step_mode: VertexStepMode::Instance,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as u64,
                            shader_location: 1,
                            format: VertexFormat::Float32x4,
                        },
                        VertexAttribute {
                            offset: std::mem::size_of::<[f32; 6]>() as u64,
                            shader_location: 2,
                            format: VertexFormat::Uint32,
                        },
                        VertexAttribute {
                            offset: std::mem::size_of::<[f32; 6]>() as u64
                                + std::mem::size_of::<u32>() as u64,
                            shader_location: 3,
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
                    format: self.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        self.marker_pipeline = Some(pipeline);
    }

    pub fn ensure_line_pipeline(&mut self, device: &Device) {
        if self.line_pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(include_wgsl!("shaders/line.wgsl"));
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("line layout"),
            bind_group_layouts: &[&self.camera_bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("line pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: 36, // vec2<f32> position (8) + vec4<f32> color (16) + u32 line_style (4) + f32 distance (4) + f32 style_param (4)
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2, // position
                        },
                        VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: VertexFormat::Float32x4, // color
                        },
                        VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: VertexFormat::Uint32, // line_style
                        },
                        VertexAttribute {
                            offset: 28,
                            shader_location: 3,
                            format: VertexFormat::Float32, // distance_along_line
                        },
                        VertexAttribute {
                            offset: 32,
                            shader_location: 4,
                            format: VertexFormat::Float32, // style_param
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: self.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::LineStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        self.line_pipeline = Some(pipeline);
    }

    pub fn ensure_overlay_pipeline(&mut self, device: &Device) {
        if self.overlay_pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(include_wgsl!("shaders/selection.wgsl"));
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("overlay layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("overlay pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: (std::mem::size_of::<[f32; 2]>()
                        + std::mem::size_of::<[f32; 4]>()) as u64,
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
                            format: VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: self.format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        self.overlay_pipeline = Some(pipeline);
    }

    pub fn ensure_line_overlay_pipeline(&mut self, device: &Device) {
        if self.line_overlay_pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(include_wgsl!("shaders/selection.wgsl"));
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("line overlay layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("line overlay pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: 24,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: self.format,
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
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        self.line_overlay_pipeline = Some(pipeline);
    }

    fn rebuild_markers(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        // Build a tightly-packed CPU-side buffer matching the shader's input layout:
        // position: f32, f32
        // color: f32 x4 (from series)
        // marker: u32 (from series)
        // size: f32

        // Only include series that have markers (marker != u32::MAX)
        let marker_series_count: usize = state
            .series
            .iter()
            .filter(|s| s.marker != u32::MAX)
            .map(|s| s.len)
            .sum();

        if marker_series_count == 0 {
            self.marker_vertex_buffer = None;
            self.marker_instances = 0;
            return;
        }

        let mut raw: Vec<u8> = Vec::with_capacity(marker_series_count * 32);
        let mut id_map: Vec<(u32, u32)> = Vec::with_capacity(marker_series_count);
        // Iterate series so we can pick series-level color/marker for each point.
        for (span_idx, s) in state.series.iter().enumerate() {
            // Skip series without markers
            if s.marker == u32::MAX {
                continue;
            }

            // safety: ensure span indexes are valid with respect to points slice
            let end = s.start + s.len;
            if s.len == 0 || end > state.points.len() {
                continue;
            }
            for (local_i, p) in state.points[s.start..end].iter().enumerate() {
                // Subtract render_offset for high-precision rendering near zero
                let render_pos = [
                    (p.position[0] - state.camera.render_offset.x) as f32,
                    (p.position[1] - state.camera.render_offset.y) as f32,
                ];
                raw.extend_from_slice(&render_pos[0].to_le_bytes());
                raw.extend_from_slice(&render_pos[1].to_le_bytes());
                // color is stored on the SeriesSpan (four f32 components)
                raw.extend_from_slice(&s.color.r.to_le_bytes());
                raw.extend_from_slice(&s.color.g.to_le_bytes());
                raw.extend_from_slice(&s.color.b.to_le_bytes());
                raw.extend_from_slice(&s.color.a.to_le_bytes());
                // marker is a u32 on the SeriesSpan
                raw.extend_from_slice(&s.marker.to_le_bytes());
                // size is per-point
                raw.extend_from_slice(&p.size.to_le_bytes());

                id_map.push((span_idx as u32, local_i as u32));
            }
        }

        let needed = raw.len() as u64;
        let recreate = match &self.marker_vertex_buffer {
            Some(buf) => buf.size() < needed,
            None => true,
        };
        if recreate {
            self.marker_vertex_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: Some("marker vb"),
                size: needed.max(1024),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(buf) = &self.marker_vertex_buffer {
            queue.write_buffer(buf, 0, &raw);
        }
        self.marker_instances = marker_series_count as u32;
        // Update picking id map
        self.picking.set_id_map(id_map);
    }

    fn rebuild_lines(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.line_vertex_buffer = None;
        self.line_segments.clear();
        if state.series.iter().all(|s| s.line_style.is_none()) {
            return;
        }
        let mut raw: Vec<u8> = Vec::new();
        let mut segs: Vec<LineSegment> = Vec::new();
        for s in state.series.iter() {
            if s.line_style.is_none() || s.len < 2 {
                continue;
            }
            let first = (raw.len() / 36) as u32; // 36 bytes per vertex
            let (line_style_u32, style_param) = match s.line_style.unwrap() {
                LineStyle::Solid => (0u32, 0.0f32),
                LineStyle::Dotted { spacing } => (1u32, spacing),
                LineStyle::Dashed { length } => (2u32, length),
            };

            let points_slice = &state.points[s.start..s.start + s.len];
            let mut cumulative_distance = 0.0f32;

            for (i, p) in points_slice.iter().enumerate() {
                if i > 0 {
                    let prev = &points_slice[i - 1];
                    let dx = p.position[0] - prev.position[0];
                    let dy = p.position[1] - prev.position[1];
                    cumulative_distance += (dx * dx + dy * dy).sqrt() as f32;
                }

                // position: vec2<f32>
                let render_pos = [
                    (p.position[0] - state.camera.render_offset.x) as f32,
                    (p.position[1] - state.camera.render_offset.y) as f32,
                ];
                raw.extend_from_slice(&render_pos[0].to_le_bytes());
                raw.extend_from_slice(&render_pos[1].to_le_bytes());
                // color: vec4<f32>
                raw.extend_from_slice(&s.color.r.to_le_bytes());
                raw.extend_from_slice(&s.color.g.to_le_bytes());
                raw.extend_from_slice(&s.color.b.to_le_bytes());
                raw.extend_from_slice(&s.color.a.to_le_bytes());
                // line_style: u32
                raw.extend_from_slice(&line_style_u32.to_le_bytes());
                // distance_along_line: f32
                raw.extend_from_slice(&cumulative_distance.to_le_bytes());
                // style_param: f32
                raw.extend_from_slice(&style_param.to_le_bytes());
            }
            let count = (raw.len() / 36) as u32 - first;
            if count >= 2 {
                segs.push(LineSegment {
                    first_vertex: first,
                    vertex_count: count,
                });
            }
        }
        if raw.is_empty() {
            return;
        }
        self.line_vertex_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("line vb"),
            size: raw.len() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        if let Some(buf) = &self.line_vertex_buffer {
            queue.write_buffer(buf, 0, &raw);
        }
        self.line_segments = segs;
    }

    fn rebuild_selection(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        let w = self.bounds_w.max(1) as f32;
        let h = self.bounds_h.max(1) as f32;
        if w <= 1.0 || h <= 1.0 {
            return;
        }
        if state.selection.active || state.selection.moved {
            const FILL: [f32; 4] = [0.2, 0.6, 1.0, 0.2];
            let p0 = state.selection.start * self.scale_factor;
            let p1 = state.selection.end * self.scale_factor;
            let min_x = p0.x.min(p1.x);
            let max_x = p0.x.max(p1.x);
            let min_y = p0.y.min(p1.y);
            let max_y = p0.y.max(p1.y);
            let to_clip =
                |sx: f32, sy: f32| -> [f32; 2] { [(sx / w) * 2.0 - 1.0, 1.0 - (sy / h) * 2.0] };
            let tl = to_clip(min_x, min_y);
            let br = to_clip(max_x, max_y);
            let tr = [br[0], tl[1]];
            let bl = [tl[0], br[1]];
            let mut data: Vec<f32> = Vec::new();
            for v in [tl, tr, bl, br] {
                data.extend_from_slice(&v);
                data.extend_from_slice(&FILL);
            }
            let raw = bytemuck::cast_slice(&data);
            self.selection_vertex_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: Some("selection vb"),
                size: raw.len() as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            if let Some(buf) = &self.selection_vertex_buffer {
                queue.write_buffer(buf, 0, raw);
            }
            self.selection_vertex_count = 4;
        } else {
            self.selection_vertex_buffer = None;
            self.selection_vertex_count = 0;
        }
    }

    fn rebuild_hover(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.hover_vertex_buffer = None;
        self.hover_vertex_count = 0;
        let Some(world) = state.hovered_world else {
            return;
        };
        // Convert world -> screen px, then to clip for a small ring quad (approximate circle by a square with alpha falloff in shader? We reuse solid quad here)
        // We will draw a simple filled square halo in overlay space (clip) sized by marker size + padding.
        let w = self.bounds_w.max(1) as f32;
        let h = self.bounds_h.max(1) as f32;
        if w <= 1.0 || h <= 1.0 {
            return;
        }
        // Convert world coordinates to render coordinates (subtract offset)
        let render_pos = [
            world[0] - state.camera.render_offset.x,
            world[1] - state.camera.render_offset.y,
        ];
        // Project to clip using camera uniform math compatible with selection path
        let cam = &state.camera;
        let ndc_x =
            (render_pos[0] as f32 - cam.effective_position().x as f32) / cam.half_extents.x as f32;
        let ndc_y =
            (render_pos[1] as f32 - cam.effective_position().y as f32) / cam.half_extents.y as f32;
        // Convert size in px to clip delta
        let px_to_clip_x = 2.0 / w;
        let px_to_clip_y = 2.0 / h;
        let radius_px = state.hovered_size_px.max(1.0) + 3.0; // halo is marker radius + padding
        let dx = radius_px * px_to_clip_x;
        let dy = radius_px * px_to_clip_y;
        // Center in clip from NDC
        let cx = ndc_x;
        let cy = ndc_y;
        // Build a quad around (cx, cy) in clip coords
        let tl = [cx - dx, cy + dy];
        let tr = [cx + dx, cy + dy];
        let bl = [cx - dx, cy - dy];
        let br = [cx + dx, cy - dy];
        let color = [1.0, 1.0, 1.0, 0.25];
        let mut data: Vec<f32> = Vec::new();
        for v in [tl, tr, bl, br] {
            data.extend_from_slice(&v);
            data.extend_from_slice(&color);
        }
        let raw = bytemuck::cast_slice(&data);
        self.hover_vertex_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("hover halo vb"),
            size: raw.len() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        if let Some(buf) = &self.hover_vertex_buffer {
            queue.write_buffer(buf, 0, raw);
        }
        self.hover_vertex_count = 4;
    }

    fn rebuild_crosshairs(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.crosshairs_vertex_buffer = None;
        self.crosshairs_vertex_count = 0;

        if !state.crosshairs_enabled {
            return;
        }

        let w = self.bounds_w.max(1) as f32;
        let h = self.bounds_h.max(1) as f32;
        if w <= 1.0 || h <= 1.0 {
            return;
        }

        // Check if cursor is within bounds
        let pos = state.crosshairs_position * self.scale_factor;
        if pos.x < 0.0 || pos.y < 0.0 || pos.x > w || pos.y > h {
            return;
        }

        // Convert cursor position to clip coordinates
        let to_clip =
            |sx: f32, sy: f32| -> [f32; 2] { [(sx / w) * 2.0 - 1.0, 1.0 - (sy / h) * 2.0] };

        let cursor_clip = to_clip(pos.x, pos.y);

        // Thin gray line color (semi-transparent)
        let color = [0.5, 0.5, 0.5, 0.5];

        let mut data: Vec<f32> = Vec::new();

        // Horizontal line (left to right through cursor)
        let left = [-1.0, cursor_clip[1]];
        let right = [1.0, cursor_clip[1]];

        // Vertical line (top to bottom through cursor)
        let top = [cursor_clip[0], 1.0];
        let bottom = [cursor_clip[0], -1.0];

        // Add horizontal line vertices
        data.extend_from_slice(&left);
        data.extend_from_slice(&color);
        data.extend_from_slice(&right);
        data.extend_from_slice(&color);

        // Add vertical line vertices
        data.extend_from_slice(&top);
        data.extend_from_slice(&color);
        data.extend_from_slice(&bottom);
        data.extend_from_slice(&color);

        let raw = bytemuck::cast_slice(&data);
        self.crosshairs_vertex_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: Some("crosshairs vb"),
            size: raw.len() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        if let Some(buf) = &self.crosshairs_vertex_buffer {
            queue.write_buffer(buf, 0, raw);
        }
        self.crosshairs_vertex_count = 4;
    }

    pub fn encode(&self, params: RenderParams) {
        // Convert bounds to viewport coordinates
        let x = params.bounds.x as f32;
        let y = params.bounds.y as f32;
        let width = params.bounds.width as f32;
        let height = params.bounds.height as f32;

        // Main pass (grid, lines, markers)
        {
            let mut pass = params.encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("fastplot main"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: params.target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set viewport and scissor to respect bounds
            pass.set_viewport(x, y, width, height, 0.0, 1.0);
            pass.set_scissor_rect(
                params.bounds.x,
                params.bounds.y,
                params.bounds.width,
                params.bounds.height,
            );

            // grid
            self.grid.draw(&mut pass, &self.camera_bind_group);
            // lines
            if let (Some(pipeline), Some(vb)) =
                (self.line_pipeline.as_ref(), &self.line_vertex_buffer)
            {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                for seg in &self.line_segments {
                    pass.draw(seg.first_vertex..seg.first_vertex + seg.vertex_count, 0..1);
                }
            }
            // markers
            if let (Some(pipeline), Some(vb)) =
                (self.marker_pipeline.as_ref(), &self.marker_vertex_buffer)
            {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..4, 0..self.marker_instances);
            }
        }

        // Selection overlay
        if let Some(pipeline) = self.overlay_pipeline.as_ref() {
            let mut pass = params.encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("selection overlay"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: params.target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set viewport and scissor for selection overlay as well
            pass.set_viewport(x, y, width, height, 0.0, 1.0);
            pass.set_scissor_rect(
                params.bounds.x,
                params.bounds.y,
                params.bounds.width,
                params.bounds.height,
            );

            pass.set_pipeline(pipeline);
            // Draw selection if present
            if let Some(vb) = &self.selection_vertex_buffer {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..self.selection_vertex_count, 0..1);
            }
            // Draw hover halo if present
            if let Some(vb) = &self.hover_vertex_buffer {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.draw(0..self.hover_vertex_count, 0..1);
            }
        }

        // Crosshairs overlay (using line list topology)
        if let (Some(pipeline), Some(vb)) = (
            self.line_overlay_pipeline.as_ref(),
            &self.crosshairs_vertex_buffer,
        ) {
            let mut pass = params.encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("crosshairs overlay"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: params.target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Set viewport and scissor for crosshairs overlay
            pass.set_viewport(x, y, width, height, 0.0, 1.0);
            pass.set_scissor_rect(
                params.bounds.x,
                params.bounds.y,
                params.bounds.width,
                params.bounds.height,
            );

            pass.set_pipeline(pipeline);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..self.crosshairs_vertex_count, 0..1);
        }
    }
}
