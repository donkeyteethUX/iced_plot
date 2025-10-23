//! GPU renderer for PlotWidget.
use crate::LineStyle;
use crate::picking::PickingPass;
use crate::{camera::CameraUniform, grid::Grid, plot_state::PlotState};
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

/// Helper struct for managing vertex buffers
struct VertexBuffer {
    buffer: Buffer,
    vertex_count: u32,
}

/// Helper struct for managing line buffers with segments
struct LineBuffer {
    buffer: Buffer,
    segments: Vec<LineSegment>,
}

/// Cache for render pipelines
struct PipelineCache {
    marker: Option<RenderPipeline>,
    line: Option<RenderPipeline>,
    overlay: Option<RenderPipeline>,
    line_overlay: Option<RenderPipeline>,
}

impl PipelineCache {
    fn new() -> Self {
        Self {
            marker: None,
            line: None,
            overlay: None,
            line_overlay: None,
        }
    }
}

/// Cache for vertex buffers
struct BufferCache {
    markers: Option<VertexBuffer>,
    lines: Option<LineBuffer>,
    reflines: Option<LineBuffer>,
    selection: Option<VertexBuffer>,
    hover: Option<VertexBuffer>,
    crosshairs: Option<VertexBuffer>,
}

impl BufferCache {
    fn new() -> Self {
        Self {
            markers: None,
            lines: None,
            reflines: None,
            selection: None,
            hover: None,
            crosshairs: None,
        }
    }
}

/// Tracks version numbers to detect changes
struct VersionTracker {
    markers: u64,
    lines: u64,
    render_offset: glam::DVec2,
}

impl VersionTracker {
    fn new() -> Self {
        Self {
            markers: 0,
            lines: 0,
            render_offset: glam::DVec2::ZERO,
        }
    }
}

/// Helper for writing vertex data
struct VertexWriter {
    data: Vec<u8>,
}

impl VertexWriter {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    fn write_f32(&mut self, value: f32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    fn write_position(&mut self, pos: [f32; 2]) {
        self.write_f32(pos[0]);
        self.write_f32(pos[1]);
    }

    fn write_color(&mut self, color: &iced::Color) {
        self.write_f32(color.r);
        self.write_f32(color.g);
        self.write_f32(color.b);
        self.write_f32(color.a);
    }

    fn write_line_vertex(
        &mut self,
        pos: [f32; 2],
        color: &iced::Color,
        style: u32,
        distance: f32,
        param: f32,
    ) {
        self.write_position(pos);
        self.write_color(color);
        self.write_u32(style);
        self.write_f32(distance);
        self.write_f32(param);
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

pub struct PlotRenderer {
    format: TextureFormat,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    camera_bgl: BindGroupLayout,
    // Caches
    pipelines: PipelineCache,
    buffers: BufferCache,
    versions: VersionTracker,
    // Support objects
    grid: Grid,
    picking: PickingPass,
    scale_factor: f32,
    bounds_w: u32,
    bounds_h: u32,
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
            pipelines: PipelineCache::new(),
            buffers: BufferCache::new(),
            versions: VersionTracker::new(),
            grid: Grid::default(),
            picking: PickingPass::default(),
            bounds_w: 0,
            bounds_h: 0,
            scale_factor: 1.0,
        }
    }

    // Coordinate conversion helpers
    fn screen_to_clip(&self, x: f32, y: f32) -> [f32; 2] {
        let w = self.bounds_w.max(1) as f32;
        let h = self.bounds_h.max(1) as f32;
        [(x / w) * 2.0 - 1.0, 1.0 - (y / h) * 2.0]
    }

    fn world_to_ndc(&self, world: [f64; 2], camera: &crate::camera::Camera) -> [f32; 2] {
        let render_pos = [
            (world[0] - camera.render_offset.x) as f32,
            (world[1] - camera.render_offset.y) as f32,
        ];
        let ndc_x =
            (render_pos[0] - camera.effective_position().x as f32) / camera.half_extents.x as f32;
        let ndc_y =
            (render_pos[1] - camera.effective_position().y as f32) / camera.half_extents.y as f32;
        [ndc_x, ndc_y]
    }

    fn pixels_to_clip_delta(&self, pixels: f32) -> (f32, f32) {
        let w = self.bounds_w.max(1) as f32;
        let h = self.bounds_h.max(1) as f32;
        (2.0 * pixels / w, 2.0 * pixels / h)
    }

    // Helper to convert world position to render position (subtract offset)
    fn world_to_render_pos(&self, world: [f64; 2], camera: &crate::camera::Camera) -> [f32; 2] {
        [
            (world[0] - camera.render_offset.x) as f32,
            (world[1] - camera.render_offset.y) as f32,
        ]
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
        self.grid.update(device, state);
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
        let offset_changed = self.versions.render_offset != state.camera.render_offset;

        if state.markers_version != self.versions.markers || offset_changed {
            self.rebuild_markers(device, queue, state);
            self.versions.markers = state.markers_version;
        }
        if state.lines_version != self.versions.lines || offset_changed {
            self.rebuild_lines(device, queue, state);
            self.versions.lines = state.lines_version;
        }

        // Rebuild reference lines whenever camera changes
        self.rebuild_reflines(device, queue, state);

        // Update cached render offset
        self.versions.render_offset = state.camera.render_offset;

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
        let marker_buffer = self.buffers.markers.as_ref().map(|vb| &vb.buffer);
        let marker_instances = self
            .buffers
            .markers
            .as_ref()
            .map(|vb| vb.vertex_count)
            .unwrap_or(0);

        self.picking.service(
            instance_id,
            device,
            queue,
            &self.camera_bind_group,
            &self.camera_bgl,
            marker_buffer,
            marker_instances,
            &state.points,
            &state.series,
        );
    }

    pub fn ensure_marker_pipeline(&mut self, device: &Device) {
        if self.pipelines.marker.is_some() {
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
        self.pipelines.marker = Some(pipeline);
    }

    pub fn ensure_line_pipeline(&mut self, device: &Device) {
        if self.pipelines.line.is_some() {
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
        self.pipelines.line = Some(pipeline);
    }

    pub fn ensure_overlay_pipeline(&mut self, device: &Device) {
        if self.pipelines.overlay.is_some() {
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
        self.pipelines.overlay = Some(pipeline);
    }

    pub fn ensure_line_overlay_pipeline(&mut self, device: &Device) {
        if self.pipelines.line_overlay.is_some() {
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
        self.pipelines.line_overlay = Some(pipeline);
    }

    fn rebuild_markers(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        // Only include series that have markers (marker != u32::MAX)
        let marker_series_count: usize = state
            .series
            .iter()
            .filter(|s| s.marker != u32::MAX)
            .map(|s| s.len)
            .sum();

        if marker_series_count == 0 {
            self.buffers.markers = None;
            return;
        }

        let mut writer = VertexWriter::with_capacity(marker_series_count * 32);
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
                let render_pos = self.world_to_render_pos(p.position, &state.camera);
                writer.write_position(render_pos);
                writer.write_color(&s.color);
                writer.write_u32(s.marker);
                writer.write_f32(p.size);

                id_map.push((span_idx as u32, local_i as u32));
            }
        }

        let data = writer.as_slice();
        let needed = data.len() as u64;

        let recreate = match &self.buffers.markers {
            Some(vb) => vb.buffer.size() < needed,
            None => true,
        };

        if recreate {
            self.buffers.markers = Some(VertexBuffer {
                buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("marker vb"),
                    size: needed.max(1024),
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                vertex_count: marker_series_count as u32,
            });
        }

        if let Some(vb) = &self.buffers.markers {
            queue.write_buffer(&vb.buffer, 0, data);
        }

        // Update picking id map
        self.picking.set_id_map(id_map);
    }

    fn rebuild_lines(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.buffers.lines = None;
        if state.series.iter().all(|s| s.line_style.is_none()) {
            return;
        }

        let mut writer = VertexWriter::new();
        let mut segs: Vec<LineSegment> = Vec::new();

        for s in state.series.iter() {
            if s.line_style.is_none() || s.len < 2 {
                continue;
            }
            let first = (writer.len() / 36) as u32; // 36 bytes per vertex
            let (line_style_u32, style_param) = line_style_params(s.line_style.unwrap());

            let points_slice = &state.points[s.start..s.start + s.len];
            let mut cumulative_distance = 0.0f32;

            for (i, p) in points_slice.iter().enumerate() {
                if i > 0 {
                    let prev = &points_slice[i - 1];
                    let dx = p.position[0] - prev.position[0];
                    let dy = p.position[1] - prev.position[1];
                    cumulative_distance += (dx * dx + dy * dy).sqrt() as f32;
                }

                let render_pos = self.world_to_render_pos(p.position, &state.camera);
                writer.write_line_vertex(
                    render_pos,
                    &s.color,
                    line_style_u32,
                    cumulative_distance,
                    style_param,
                );
            }

            let count = (writer.len() / 36) as u32 - first;
            if count >= 2 {
                segs.push(LineSegment {
                    first_vertex: first,
                    vertex_count: count,
                });
            }
        }

        if writer.is_empty() {
            return;
        }

        let data = writer.as_slice();
        self.buffers.lines = Some(LineBuffer {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("line vb"),
                size: data.len() as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            segments: segs,
        });

        if let Some(lb) = &self.buffers.lines {
            queue.write_buffer(&lb.buffer, 0, data);
        }
    }

    fn rebuild_reflines(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.buffers.reflines = None;

        if state.vlines.is_empty() && state.hlines.is_empty() {
            return;
        }

        let mut writer = VertexWriter::new();
        let mut segs: Vec<LineSegment> = Vec::new();

        // Get visible viewport bounds in world coordinates
        let cam = &state.camera;
        let left = cam.position.x - cam.half_extents.x;
        let right = cam.position.x + cam.half_extents.x;
        let bottom = cam.position.y - cam.half_extents.y;
        let top = cam.position.y + cam.half_extents.y;

        // Add vertical lines
        for vline in state.vlines.iter() {
            // Check if vline is within viewport
            if vline.x < left || vline.x > right {
                continue;
            }

            let first = (writer.len() / 36) as u32;
            let (line_style_u32, style_param) = line_style_params(vline.line_style);

            // Create two vertices: bottom and top of viewport
            for (idx, y) in [bottom, top].iter().enumerate() {
                let render_pos = self.world_to_render_pos([vline.x, *y], &state.camera);
                let distance = if idx == 0 { 0.0 } else { (top - bottom) as f32 };
                writer.write_line_vertex(
                    render_pos,
                    &vline.color,
                    line_style_u32,
                    distance,
                    style_param,
                );
            }

            segs.push(LineSegment {
                first_vertex: first,
                vertex_count: 2,
            });
        }

        // Add horizontal lines
        for hline in state.hlines.iter() {
            // Check if hline is within viewport
            if hline.y < bottom || hline.y > top {
                continue;
            }

            let first = (writer.len() / 36) as u32;
            let (line_style_u32, style_param) = line_style_params(hline.line_style);

            // Create two vertices: left and right of viewport
            for (idx, x) in [left, right].iter().enumerate() {
                let render_pos = self.world_to_render_pos([*x, hline.y], &state.camera);
                let distance = if idx == 0 { 0.0 } else { (right - left) as f32 };
                writer.write_line_vertex(
                    render_pos,
                    &hline.color,
                    line_style_u32,
                    distance,
                    style_param,
                );
            }

            segs.push(LineSegment {
                first_vertex: first,
                vertex_count: 2,
            });
        }

        if writer.is_empty() {
            return;
        }

        let data = writer.as_slice();
        self.buffers.reflines = Some(LineBuffer {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("refline vb"),
                size: data.len() as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            segments: segs,
        });

        if let Some(lb) = &self.buffers.reflines {
            queue.write_buffer(&lb.buffer, 0, data);
        }
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
            let tl = self.screen_to_clip(min_x, min_y);
            let br = self.screen_to_clip(max_x, max_y);
            let tr = [br[0], tl[1]];
            let bl = [tl[0], br[1]];
            let mut data: Vec<f32> = Vec::new();
            for v in [tl, tr, bl, br] {
                data.extend_from_slice(&v);
                data.extend_from_slice(&FILL);
            }
            let raw = bytemuck::cast_slice(&data);
            self.buffers.selection = Some(VertexBuffer {
                buffer: device.create_buffer(&BufferDescriptor {
                    label: Some("selection vb"),
                    size: raw.len() as u64,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                vertex_count: 4,
            });
            if let Some(vb) = &self.buffers.selection {
                queue.write_buffer(&vb.buffer, 0, raw);
            }
        } else {
            self.buffers.selection = None;
        }
    }

    fn rebuild_hover(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.buffers.hover = None;
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
        // Convert world coordinates to NDC
        let ndc = self.world_to_ndc(world, &state.camera);

        // Convert size in px to clip delta
        let (dx, dy) = self.pixels_to_clip_delta(state.hovered_size_px.max(1.0) + 3.0);

        // Build a quad around (cx, cy) in clip coords
        let tl = [ndc[0] - dx, ndc[1] + dy];
        let tr = [ndc[0] + dx, ndc[1] + dy];
        let bl = [ndc[0] - dx, ndc[1] - dy];
        let br = [ndc[0] + dx, ndc[1] - dy];
        let color = [1.0, 1.0, 1.0, 0.25];
        let mut data: Vec<f32> = Vec::new();
        for v in [tl, tr, bl, br] {
            data.extend_from_slice(&v);
            data.extend_from_slice(&color);
        }
        let raw = bytemuck::cast_slice(&data);
        self.buffers.hover = Some(VertexBuffer {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("hover halo vb"),
                size: raw.len() as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            vertex_count: 4,
        });
        if let Some(vb) = &self.buffers.hover {
            queue.write_buffer(&vb.buffer, 0, raw);
        }
    }

    fn rebuild_crosshairs(&mut self, device: &Device, queue: &Queue, state: &PlotState) {
        self.buffers.crosshairs = None;

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
        let cursor_clip = self.screen_to_clip(pos.x, pos.y);

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
        self.buffers.crosshairs = Some(VertexBuffer {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("crosshairs vb"),
                size: raw.len() as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            vertex_count: 4,
        });
        if let Some(vb) = &self.buffers.crosshairs {
            queue.write_buffer(&vb.buffer, 0, raw);
        }
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
                label: Some("iced_plot main"),
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
            if let (Some(pipeline), Some(lb)) = (self.pipelines.line.as_ref(), &self.buffers.lines)
            {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, lb.buffer.slice(..));
                for seg in &lb.segments {
                    pass.draw(seg.first_vertex..seg.first_vertex + seg.vertex_count, 0..1);
                }
            }
            // reference lines (vlines and hlines)
            if let (Some(pipeline), Some(lb)) =
                (self.pipelines.line.as_ref(), &self.buffers.reflines)
            {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, lb.buffer.slice(..));
                for seg in &lb.segments {
                    pass.draw(seg.first_vertex..seg.first_vertex + seg.vertex_count, 0..1);
                }
            }
            // markers
            if let (Some(pipeline), Some(vb)) =
                (self.pipelines.marker.as_ref(), &self.buffers.markers)
            {
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, vb.buffer.slice(..));
                pass.draw(0..4, 0..vb.vertex_count);
            }
        }

        // Selection overlay
        if let Some(pipeline) = self.pipelines.overlay.as_ref() {
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
            if let Some(vb) = &self.buffers.selection {
                pass.set_vertex_buffer(0, vb.buffer.slice(..));
                pass.draw(0..vb.vertex_count, 0..1);
            }
            // Draw hover halo if present
            if let Some(vb) = &self.buffers.hover {
                pass.set_vertex_buffer(0, vb.buffer.slice(..));
                pass.draw(0..vb.vertex_count, 0..1);
            }
        }

        // Crosshairs overlay (using line list topology)
        if let (Some(pipeline), Some(vb)) = (
            self.pipelines.line_overlay.as_ref(),
            &self.buffers.crosshairs,
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
            pass.set_vertex_buffer(0, vb.buffer.slice(..));
            pass.draw(0..vb.vertex_count, 0..1);
        }
    }
}

// Helper to extract line style parameters
fn line_style_params(style: LineStyle) -> (u32, f32) {
    match style {
        LineStyle::Solid => (0u32, 0.0f32),
        LineStyle::Dotted { spacing } => (1u32, spacing),
        LineStyle::Dashed { length } => (2u32, length),
    }
}
