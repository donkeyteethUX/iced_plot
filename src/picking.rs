//! Implements picking for the plot widget.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use glam::{DVec2, Vec2};
use iced::Rectangle;
use iced::wgpu::*;

use crate::{MarkerSize, Point, PointId, camera::Camera, plot_state::SeriesSpan};

/// Threshold for number of points above which GPU picking is used instead of CPU picking.
pub(crate) const CPU_PICK_THRESHOLD: usize = 5000;

// ---- API to the plot widget ----

/// Tracks CPU/GPU picking state for a plot widget.
#[derive(Debug, Clone, Default)]
pub(crate) struct PickingState {
    /// Last hover hit, if any.
    pub(crate) last_hover_cache: Option<PointId>,

    /// When set, the matching GPU result is interpreted as a *pick* (click)
    /// instead of a hover.
    pending_gpu_pick_seq: Option<u64>,

    /// Last submitted GPU request sequence number
    pick_seq: u64,

    /// Last processed GPU result sequence number
    pick_result_seq: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum HoverRequest {
    /// Immediate CPU result.
    CpuHit(PointId),
    /// Immediate CPU miss.
    CpuMiss,
    /// GPU request was submitted; result will arrive in a later frame.
    RequestedGpu,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum GpuResultEvent {
    Hover(PointId),
    HoverMiss,
    Pick(PointId),
}

impl PickingState {
    fn submit_gpu_request(&mut self, instance_id: u64, cursor: Vec2, radius_px: f32) {
        self.pick_seq = self.pick_seq.wrapping_add(1);
        submit_request(
            instance_id,
            GpuPickRequest {
                cursor_x: cursor.x,
                cursor_y: cursor.y,
                radius_px,
                seq: self.pick_seq,
            },
        );
    }

    /// Request hover picking for the current cursor position.
    ///
    /// CPU vs GPU is decided internally based on point count.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request_hover(
        &mut self,
        instance_id: u64,
        cursor: Vec2,
        hover_radius_px: f32,
        points: &[Point],
        series: &[SeriesSpan],
        camera: &Camera,
        bounds: &Rectangle,
        valid_point_id: impl Fn(&PointId) -> bool,
    ) -> HoverRequest {
        if points.len() < CPU_PICK_THRESHOLD {
            if let Some(point) =
                cpu_pick_hit(points, series, camera, bounds, cursor, hover_radius_px)
                && valid_point_id(&point)
            {
                self.last_hover_cache = Some(point);
                HoverRequest::CpuHit(point)
            } else {
                HoverRequest::CpuMiss
            }
        } else {
            self.submit_gpu_request(instance_id, cursor, hover_radius_px);
            HoverRequest::RequestedGpu
        }
    }

    /// Request a click-to-pick hit at the current cursor position.
    ///
    /// Returns an immediate hit when using cache/CPU. For GPU, submits a request and returns None.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request_pick_hit(
        &mut self,
        instance_id: u64,
        cursor: Vec2,
        hover_radius_px: f32,
        points: &[Point],
        series: &[SeriesSpan],
        camera: &Camera,
        bounds: &Rectangle,
        valid_point_id: impl Fn(&PointId) -> bool,
    ) -> Option<PointId> {
        if let Some(point) = self.last_hover_cache
            && valid_point_id(&point)
        {
            return Some(point);
        }

        if points.len() < CPU_PICK_THRESHOLD {
            if let Some(point) =
                cpu_pick_hit(points, series, camera, bounds, cursor, hover_radius_px)
                && valid_point_id(&point)
            {
                return Some(point);
            }
        } else {
            self.submit_gpu_request(instance_id, cursor, hover_radius_px);
            // Mark this seq as a pick request.
            self.pending_gpu_pick_seq = Some(self.pick_seq);
        }
        None
    }

    /// Consume and interpret a GPU pick result (if available).
    pub(crate) fn consume_gpu_result(
        &mut self,
        instance_id: u64,
        valid_point_id: impl Fn(&PointId) -> bool,
    ) -> Option<GpuResultEvent> {
        let res = take_result(instance_id)?;
        if res.seq <= self.pick_result_seq {
            return None;
        }

        let mut out = None;

        if self.pending_gpu_pick_seq == Some(res.seq) {
            self.pending_gpu_pick_seq = None;
            if let Some(point) = res.hit
                && valid_point_id(&point)
            {
                out = Some(GpuResultEvent::Pick(point));
            }
        } else if let Some(point) = res.hit
            && valid_point_id(&point)
        {
            self.last_hover_cache = Some(point);
            out = Some(GpuResultEvent::Hover(point));
        } else {
            out = Some(GpuResultEvent::HoverMiss);
        }

        self.pick_result_seq = res.seq;
        out
    }

    pub(crate) fn has_outstanding_gpu_request(&self) -> bool {
        self.pick_seq > self.pick_result_seq
    }
}

fn marker_center_world(pt: &Point) -> DVec2 {
    let mut world = DVec2::new(pt.position[0], pt.position[1]);
    if pt.size_mode == crate::point::MARKER_SIZE_WORLD {
        let half = pt.size as f64 * 0.5;
        world.x += half;
        world.y += half;
    }
    world
}

fn cpu_pick_hit(
    points: &[Point],
    series: &[SeriesSpan],
    camera: &Camera,
    bounds: &Rectangle,
    cursor: Vec2,
    hover_radius_px: f32,
) -> Option<PointId> {
    if points.is_empty() || series.is_empty() {
        return None;
    }

    let width = bounds.width.max(1.0) as f64;
    let height = bounds.height.max(1.0) as f64;
    let cursor_x = cursor.x as f64;
    let cursor_y = cursor.y as f64;

    let mut span_idx = 0usize;
    let mut span_start = 0usize;
    let mut best: Option<(usize, f64)> = None;

    for (idx, pt) in points.iter().enumerate() {
        while span_idx < series.len() && idx >= span_start + series[span_idx].len {
            span_start += series[span_idx].len;
            span_idx += 1;
        }
        if span_idx >= series.len() {
            break;
        }

        let world = marker_center_world(pt);
        let ndc_x = (world.x - camera.position.x) / camera.half_extents.x;
        let ndc_y = (world.y - camera.position.y) / camera.half_extents.y;
        let screen_x = (ndc_x + 1.0) * 0.5 * width;
        let screen_y = (1.0 - ndc_y) * 0.5 * height;

        let dx = screen_x - cursor_x;
        let dy = screen_y - cursor_y;
        let d2 = dx * dx + dy * dy;
        let marker_px = MarkerSize::marker_size_px(pt.size, pt.size_mode, camera, bounds) as f64;
        let radius = hover_radius_px as f64 + marker_px * 0.5;
        if d2 <= radius * radius {
            if let Some((_, best_d2)) = best {
                if d2 < best_d2 {
                    best = Some((idx, d2));
                }
            } else {
                best = Some((idx, d2));
            }
        }
    }

    let (best_idx, _) = best?;
    let mut span_idx = 0usize;
    let mut span_start = 0usize;
    while span_idx < series.len() && best_idx >= span_start + series[span_idx].len {
        span_start += series[span_idx].len;
        span_idx += 1;
    }
    let span = series.get(span_idx)?;
    let local_idx = best_idx - span_start;
    Some(PointId {
        series_id: span.id,
        point_index: local_idx,
    })
}

// ---- GPU picking ----

#[derive(Debug, Clone, Copy)]
struct GpuPickRequest {
    pub cursor_x: f32,  // logical px in widget local coordinates
    pub cursor_y: f32,  // logical px in widget local coordinates
    pub radius_px: f32, // logical px
    pub seq: u64,       // monotonically increasing sequence
}

#[derive(Debug, Clone)]
struct GpuPickResult {
    pub seq: u64,
    pub hit: Option<PointId>,
}

#[derive(Default)]
struct InstanceEntry {
    latest_req: Option<GpuPickRequest>,
    latest_res: Option<GpuPickResult>,
}

static REGISTRY: OnceLock<Mutex<HashMap<u64, InstanceEntry>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<u64, InstanceEntry>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn submit_request(instance_id: u64, req: GpuPickRequest) {
    let mut map = registry().lock().unwrap();
    let entry = map.entry(instance_id).or_default();
    // Replace if newer
    if entry.latest_req.map(|r| r.seq < req.seq).unwrap_or(true) {
        entry.latest_req = Some(req);
    }
}

fn take_result(instance_id: u64) -> Option<GpuPickResult> {
    let mut map = registry().lock().unwrap();
    map.get_mut(&instance_id).and_then(|e| e.latest_res.take())
}

fn take_latest_request(instance_id: u64) -> Option<GpuPickRequest> {
    let mut map = registry().lock().unwrap();
    map.get_mut(&instance_id).and_then(|e| e.latest_req.take())
}

fn publish_result(instance_id: u64, res: GpuPickResult) {
    let mut map = registry().lock().unwrap();
    let entry = map.entry(instance_id).or_default();
    if entry.latest_res.as_ref().is_none_or(|r| r.seq < res.seq) {
        entry.latest_res = Some(res);
    }
}

// ---- GPU picking pass ----

pub(crate) struct PickingPass {
    // Render target holding u32 IDs
    pick_texture: Option<Texture>,
    pick_view: Option<TextureView>,
    size_w: u32,
    size_h: u32,
    scale_factor: f32,

    // Pipeline for rendering marker IDs
    pipeline: Option<RenderPipeline>,

    // Temporary staging buffer for readback (sync for now; tiny region)
    staging: Option<Buffer>,
    staging_size: u64,

    // Mapping from instance_id (draw instance) -> (span_index, local_pt_index)
    id_map: Vec<(u32, u32)>,

    pending: Option<PendingReadback>,
}

struct PendingReadback {
    instance_id: u64,
    seq: u64,
    needed: u64,
    bytes_per_row: u32,
    max_w: u32,
    max_h: u32,
    min_x: u32,
    min_y: u32,
    cx: u32,
    cy: u32,
    map_status: Arc<Mutex<Option<Result<(), BufferAsyncError>>>>,
}

impl Default for PickingPass {
    fn default() -> Self {
        Self {
            pick_texture: None,
            pick_view: None,
            size_w: 0,
            size_h: 0,
            scale_factor: 1.0,
            pipeline: None,
            staging: None,
            staging_size: 0,
            id_map: Vec::new(),
            pending: None,
        }
    }
}

impl PickingPass {
    /// Service a pick request: draw IDs, copy a small region around cursor, and start an async
    /// map/readback. Completion is handled on later frames without blocking.
    /// Publishes a PickResult via the registry.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn service(
        &mut self,
        instance_id: u64,
        device: &Device,
        queue: &Queue,
        camera_bg: &BindGroup,
        camera_bgl: &BindGroupLayout,
        marker_vb: Option<&Buffer>,
        marker_instances: u32,
        points: &[Point],
        series: &[SeriesSpan],
    ) {
        self.poll_pending(device, points, series);

        if self.pending.is_some() {
            return;
        }

        // Take the latest request, if any
        let req = match take_latest_request(instance_id) {
            Some(r) => r,
            None => return,
        };

        if marker_vb.is_none() || marker_instances == 0 {
            publish_result(
                instance_id,
                GpuPickResult {
                    seq: req.seq,
                    hit: None,
                },
            );
            return;
        }

        // Ensure resources
        self.ensure_target(device);
        self.ensure_pipeline(device, camera_bgl);

        let vb = marker_vb.unwrap();
        let view = self.pick_view.as_ref().unwrap();

        // Draw IDs into pick texture
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("pick encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("pick pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let w = self.size_w as f32;
            let h = self.size_h as f32;
            pass.set_viewport(0.0, 0.0, w, h, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, self.size_w, self.size_h);
            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.set_bind_group(0, camera_bg, &[]);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.draw(0..4, 0..marker_instances);
        }

        // Compute copy region in device pixels
        let cx = (req.cursor_x * self.scale_factor)
            .round()
            .clamp(0.0, self.size_w as f32 - 1.0) as u32;
        let cy = (req.cursor_y * self.scale_factor)
            .round()
            .clamp(0.0, self.size_h as f32 - 1.0) as u32;
        let r = (req.radius_px * self.scale_factor).ceil() as i32;
        let win = 2 * r + 1;
        let win = win.max(3) as u32;
        // Clamp to texture bounds
        let min_x = cx.saturating_sub(win / 2);
        let min_y = cy.saturating_sub(win / 2);
        let max_w = (self.size_w - min_x).min(win);
        let max_h = (self.size_h - min_y).min(win);

        let bytes_per_pixel = 4u32; // R32Uint
        let bytes_per_row = (max_w * bytes_per_pixel).div_ceil(256) * 256; // required alignment
        let needed = bytes_per_row as u64 * max_h as u64;
        self.ensure_staging(device, needed);

        let destination = TexelCopyBufferInfo {
            buffer: self.staging.as_ref().unwrap(),
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(max_h),
            },
        };
        let copy_size = Extent3d {
            width: max_w,
            height: max_h,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: self.pick_texture.as_ref().unwrap(),
                mip_level: 0,
                origin: Origin3d {
                    x: min_x,
                    y: min_y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            destination,
            copy_size,
        );

        // Submit and asynchronously map the tiny buffer
        queue.submit(std::iter::once(encoder.finish()));

        let buf = self.staging.as_ref().unwrap();
        let slice = buf.slice(0..needed);
        let map_status = Arc::new(Mutex::new(None));
        let status_clone = Arc::clone(&map_status);
        slice.map_async(MapMode::Read, move |res| {
            *status_clone.lock().unwrap() = Some(res);
        });

        self.pending = Some(PendingReadback {
            instance_id,
            seq: req.seq,
            needed,
            bytes_per_row,
            max_w,
            max_h,
            min_x,
            min_y,
            cx,
            cy,
            map_status,
        });
    }

    pub(crate) fn set_view(&mut self, w: u32, h: u32, scale: f32) {
        self.size_w = w.max(1);
        self.size_h = h.max(1);
        self.scale_factor = scale;
    }

    pub(crate) fn set_id_map(&mut self, map: Vec<(u32, u32)>) {
        self.id_map = map;
    }

    fn poll_pending(&mut self, device: &Device, points: &[Point], series: &[SeriesSpan]) {
        let Some(pending) = self.pending.as_ref() else {
            return;
        };

        let _ = device.poll(PollType::Poll);
        let Some(res) = pending.map_status.lock().unwrap().take() else {
            return;
        };

        let hit = match res {
            Ok(()) => {
                let buf = self.staging.as_ref().unwrap();
                let slice = buf.slice(0..pending.needed);
                let data = slice.get_mapped_range();
                let best = Self::scan_best_id(
                    &data,
                    pending.bytes_per_row,
                    pending.max_w,
                    pending.max_h,
                    pending.min_x,
                    pending.min_y,
                    pending.cx,
                    pending.cy,
                );
                drop(data);
                buf.unmap();
                best.and_then(|(id, _)| self.decode_id_to_hit(id, points, series))
            }
            Err(_) => {
                if let Some(buf) = self.staging.as_ref() {
                    buf.unmap();
                }
                None
            }
        };

        publish_result(
            pending.instance_id,
            GpuPickResult {
                seq: pending.seq,
                hit,
            },
        );

        self.pending = None;
    }

    #[allow(clippy::too_many_arguments)]
    fn scan_best_id(
        data: &[u8],
        bytes_per_row: u32,
        max_w: u32,
        max_h: u32,
        min_x: u32,
        min_y: u32,
        cx: u32,
        cy: u32,
    ) -> Option<(u32, i32)> {
        let mut best: Option<(u32, i32)> = None;
        for row in 0..max_h as usize {
            let row_off = row as u64 * bytes_per_row as u64;
            for col in 0..max_w as usize {
                let off = row_off + (col as u64) * 4;
                let id = u32::from_le_bytes([
                    data[off as usize],
                    data[off as usize + 1],
                    data[off as usize + 2],
                    data[off as usize + 3],
                ]);
                if id != 0 {
                    let sx = min_x as i32 + col as i32;
                    let sy = min_y as i32 + row as i32;
                    let dx = sx - cx as i32;
                    let dy = sy - cy as i32;
                    let d2 = dx * dx + dy * dy;
                    if let Some((_, bd2)) = best {
                        if d2 < bd2 {
                            best = Some((id, d2));
                        }
                    } else {
                        best = Some((id, d2));
                    }
                }
            }
        }
        best
    }

    fn ensure_staging(&mut self, device: &Device, needed: u64) {
        if self
            .staging
            .as_ref()
            .map(|b| b.size() >= needed)
            .unwrap_or(false)
        {
            return;
        }
        let size = needed.max(4096);
        self.staging = Some(device.create_buffer(&BufferDescriptor {
            label: Some("pick staging"),
            size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.staging_size = size;
    }

    fn ensure_target(&mut self, device: &Device) {
        let need_new = self
            .pick_texture
            .as_ref()
            .map(|t| {
                let size = t.size();
                size.width != self.size_w || size.height != self.size_h
            })
            .unwrap_or(true);
        if need_new {
            let tex = device.create_texture(&TextureDescriptor {
                label: Some("pick texture"),
                size: Extent3d {
                    width: self.size_w,
                    height: self.size_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Uint,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = tex.create_view(&TextureViewDescriptor::default());
            self.pick_view = Some(view);
            self.pick_texture = Some(tex);
        }
    }

    fn ensure_pipeline(&mut self, device: &Device, camera_bgl: &BindGroupLayout) {
        if self.pipeline.is_some() {
            return;
        }
        let shader = device.create_shader_module(include_wgsl!("shaders/pick_markers.wgsl"));
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pick layout"),
            bind_group_layouts: &[camera_bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pick pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    // Must match markers: 36 bytes per instance
                    array_stride: 36,
                    step_mode: VertexStepMode::Instance,
                    attributes: &[
                        VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x2,
                        },
                        VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: VertexFormat::Float32x4,
                        },
                        VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: VertexFormat::Uint32,
                        },
                        VertexAttribute {
                            offset: 28,
                            shader_location: 3,
                            format: VertexFormat::Float32,
                        },
                        VertexAttribute {
                            offset: 32,
                            shader_location: 4,
                            format: VertexFormat::Uint32,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::R32Uint,
                    blend: None,
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
        self.pipeline = Some(pipeline);
    }

    fn decode_id_to_hit(
        &self,
        id: u32,
        points: &[Point],
        series: &[SeriesSpan],
    ) -> Option<PointId> {
        // IDs are 1-based instance index
        let idx = (id as usize).saturating_sub(1);

        if idx >= self.id_map.len() {
            return None;
        }

        let (span_idx_u32, local_idx_u32) = self.id_map[idx];
        let span_idx = span_idx_u32 as usize;
        let local_idx = local_idx_u32 as usize;

        if span_idx >= series.len() {
            return None;
        }

        let span: &SeriesSpan = &series[span_idx];
        let point_idx = span.start + local_idx;

        if point_idx >= points.len() {
            return None;
        }

        Some(PointId {
            series_id: span.id,
            point_index: local_idx,
        })
    }
}
