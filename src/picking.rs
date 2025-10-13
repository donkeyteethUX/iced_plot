//! GPU-based hover picking using an offscreen ID buffer and tiny readback.
//!
//! This module owns the picking render pipeline, ID render target, and a small
//! request/result registry keyed by widget instance_id. The flow is:
//! - The widget submits a PickRequest on cursor move.
//! - During renderer prepare_frame, we render marker IDs into an R32Uint texture,
//!   copy a tiny region around the cursor into a staging buffer, map and scan it,
//!   then publish a PickResult.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::wgpu::PollType;
use iced::wgpu::*;

use crate::widget::{PlotState, SeriesSpan};

// ---- Public API to the widget ----

#[derive(Debug, Clone, Copy)]
pub(crate) struct PickRequest {
    pub cursor_x: f32,  // logical px in widget local coordinates
    pub cursor_y: f32,  // logical px in widget local coordinates
    pub radius_px: f32, // logical px
    pub seq: u64,       // monotonically increasing sequence
}

#[derive(Debug, Clone)]
pub(crate) struct PickResult {
    pub seq: u64,
    pub hit: Option<Hit>,
}

#[derive(Debug, Clone)]
pub(crate) struct Hit {
    pub series_label: String,
    pub point_index: usize, // index within its series span
    pub world: [f64; 2],
    pub size_px: f32,
}

#[derive(Default)]
struct InstanceEntry {
    latest_req: Option<PickRequest>,
    latest_res: Option<PickResult>,
}

static REGISTRY: OnceLock<Mutex<HashMap<u64, InstanceEntry>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<u64, InstanceEntry>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn submit_request(instance_id: u64, req: PickRequest) {
    let mut map = registry().lock().unwrap();
    let entry = map.entry(instance_id).or_default();
    // Replace if newer
    if entry.latest_req.map(|r| r.seq < req.seq).unwrap_or(true) {
        entry.latest_req = Some(req);
    }
}

pub(crate) fn take_result(instance_id: u64) -> Option<PickResult> {
    let mut map = registry().lock().unwrap();
    map.get_mut(&instance_id).and_then(|e| e.latest_res.take())
}

fn take_latest_request(instance_id: u64) -> Option<PickRequest> {
    let mut map = registry().lock().unwrap();
    map.get_mut(&instance_id).and_then(|e| e.latest_req.take())
}

fn publish_result(instance_id: u64, res: PickResult) {
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
        }
    }
}

impl PickingPass {
    /// Service a pick request: draw IDs, copy a small region around cursor, map and scan synchronously.
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
        state: &PlotState,
    ) {
        // Take the latest request, if any
        let req = match take_latest_request(instance_id) {
            Some(r) => r,
            None => return,
        };

        if marker_vb.is_none() || marker_instances == 0 {
            publish_result(
                instance_id,
                PickResult {
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

        // Submit and synchronously map the tiny buffer (acceptable small stall)
        queue.submit(std::iter::once(encoder.finish()));

        let buf = self.staging.as_ref().unwrap();
        let slice = buf.slice(0..needed);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        slice.map_async(MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        // Drive completion for this tiny buffer
        let _ = device.poll(PollType::Wait);
        let _ = rx.recv();
        let data = slice.get_mapped_range();
        // Scan for nearest non-zero, by squared distance in pixel space
        let mut best: Option<(u32, i32)> = None; // (id, dist2)
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
                    let dx = sx as i32 - cx as i32;
                    let dy = sy as i32 - cy as i32;
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
        drop(data);
        buf.unmap();

        // Decode and publish
        let hit = best.and_then(|(id, _)| self.decode_id_to_hit(id, state));
        publish_result(instance_id, PickResult { seq: req.seq, hit });
    }

    pub(crate) fn set_view(&mut self, w: u32, h: u32, scale: f32) {
        self.size_w = w.max(1);
        self.size_h = h.max(1);
        self.scale_factor = scale;
    }

    pub(crate) fn set_id_map(&mut self, map: Vec<(u32, u32)>) {
        self.id_map = map;
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
                    // Must match markers: 32 bytes per instance
                    array_stride: 32,
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

    fn decode_id_to_hit(&self, id: u32, state: &PlotState) -> Option<Hit> {
        // IDs are 1-based instance index
        let idx = (id as usize).saturating_sub(1);

        if idx >= self.id_map.len() {
            return None;
        }

        let (span_idx_u32, local_idx_u32) = self.id_map[idx];
        let span_idx = span_idx_u32 as usize;
        let local_idx = local_idx_u32 as usize;

        if span_idx >= state.series.len() {
            return None;
        }

        let span: &SeriesSpan = &state.series[span_idx];
        let point_idx = span.start + local_idx;

        if point_idx >= state.points.len() {
            return None;
        }

        let pt = &state.points[point_idx];
        let world = [pt.position[0], pt.position[1]];

        Some(Hit {
            series_label: span.label.clone(),
            point_index: local_idx,
            world,
            size_px: pt.size,
        })
    }
}
