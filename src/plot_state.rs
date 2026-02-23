use std::sync::Arc;

use glam::{DVec2, Vec2};
use iced::{
    Color, Rectangle, keyboard,
    mouse::{self, Event},
    time::Instant,
};

use crate::{
    AxisLink, AxisScale, HLine, HoverPickEvent, LineStyle, MarkerSize, PlotWidget, Point, ShapeId,
    VLine,
    axis_scale::data_point_to_plot,
    camera::Camera,
    picking::PickingState,
    plot_widget::{HighlightPoint, world_to_screen_position_x, world_to_screen_position_y},
    ticks::{PositionedTick, TickFormatter, TickProducer},
};

#[derive(Clone)]
/// PlotState is a projection of the widget configuration, data, and interaction state.
/// It holds the GPU-ready data needed for rendering the plot.
///
/// Not part of the public API, but pub visibility is required for the shader implementation.
pub struct PlotState {
    // Immutable shared data to allow cheap shallow clones.
    pub(crate) points: Arc<[Point]>,       // vertex/instance data
    pub(crate) point_colors: Arc<[Color]>, // per-point colors (matches points)
    pub(crate) series: Arc<[SeriesSpan]>,  // spans describing logical series
    pub(crate) fills: Arc<[FillSpan]>,     // triangulated fill spans
    pub(crate) vlines: Arc<[VLine]>,       // vertical reference lines
    pub(crate) hlines: Arc<[HLine]>,       // horizontal reference lines
    pub(crate) data_min: Option<DVec2>,
    pub(crate) data_max: Option<DVec2>,
    // Axis limits
    pub(crate) x_lim: Option<(f64, f64)>,
    pub(crate) y_lim: Option<(f64, f64)>,
    pub(crate) x_axis_scale: AxisScale,
    pub(crate) y_axis_scale: AxisScale,
    // Axis links for synchronization
    pub(crate) x_axis_link: Option<AxisLink>,
    pub(crate) y_axis_link: Option<AxisLink>,
    pub(crate) x_link_version: u64,
    pub(crate) y_link_version: u64,
    // UI / camera
    pub(crate) camera: Camera,
    pub(crate) bounds: Rectangle,
    pub(crate) x_ticks: Vec<PositionedTick>,
    pub(crate) y_ticks: Vec<PositionedTick>,
    // Interaction state
    pub(crate) cursor_position: Vec2,
    pub(crate) last_click_time: Option<Instant>,
    pub(crate) legend_collapsed: bool,
    pub(crate) modifiers: keyboard::Modifiers,
    pub(crate) selection: SelectionState,
    pub(crate) pan: PanState,
    /// Hover/select point rendering data (for incremental rendering)
    pub(crate) highlighted_points: Arc<[HighlightPoint]>,
    // Version counters
    pub(crate) markers_version: u64,
    pub(crate) lines_version: u64,
    pub(crate) fills_version: u64,
    pub(crate) highlight_version: u64,
    pub(crate) data_src_version: u64, // version of source data last synced
    pub(crate) highlight_src_version: u64,
    pub(crate) source_instance_id: Option<u64>,
    // Hover/picking internals
    pub(crate) hover_enabled: bool,
    pub(crate) hover_radius_px: f32,
    pub(crate) picking: PickingState,
    pub(crate) crosshairs_enabled: bool,
    pub(crate) crosshairs_position: Vec2,
    pub(crate) x_axis_formatter: Option<TickFormatter>,
    pub(crate) y_axis_formatter: Option<TickFormatter>,
}

impl Default for PlotState {
    fn default() -> Self {
        Self {
            data_src_version: 0,
            highlight_src_version: 0,
            source_instance_id: None,
            points: Arc::new([]),
            point_colors: Arc::new([]),
            highlighted_points: Arc::new([]),
            series: Arc::new([]),
            fills: Arc::new([]),
            vlines: Arc::new([]),
            hlines: Arc::new([]),
            data_min: None,
            data_max: None,
            x_lim: None,
            y_lim: None,
            x_axis_scale: AxisScale::Linear,
            y_axis_scale: AxisScale::Linear,
            x_axis_link: None,
            y_axis_link: None,
            x_link_version: 0,
            y_link_version: 0,
            camera: Camera::new(1000, 600),
            bounds: Rectangle::default(),
            cursor_position: Vec2::ZERO,
            last_click_time: None,
            legend_collapsed: false,
            modifiers: keyboard::Modifiers::default(),
            selection: SelectionState::default(),
            pan: PanState::default(),
            markers_version: 1,
            lines_version: 1,
            fills_version: 1,
            highlight_version: 0,
            hover_enabled: true,
            hover_radius_px: 8.0,
            picking: PickingState::default(),
            crosshairs_enabled: false,
            crosshairs_position: Vec2::ZERO,
            x_axis_formatter: None,
            y_axis_formatter: None,
            x_ticks: Vec::new(),
            y_ticks: Vec::new(),
        }
    }
}

impl PlotState {
    /// Sync hover/pick highlight overlay points from the widget without rebuilding plot geometry.
    ///
    /// Returns true if the overlay data changed.
    pub(crate) fn sync_highlighted_points_from_widget(&mut self, widget: &PlotWidget) -> bool {
        let highlighted_points: Vec<_> = widget
            .visible_highlighted_points()
            .map(|(highlight_point, _)| *highlight_point)
            .collect();

        if self.highlighted_points.as_ref() != highlighted_points.as_slice() {
            self.highlight_version = self.highlight_version.wrapping_add(1);
            self.highlighted_points = highlighted_points.into();
            true
        } else {
            false
        }
    }

    /// Rebuild GPU data from widget configuration.
    pub(crate) fn rebuild_from_widget(&mut self, widget: &PlotWidget) {
        let mut points = Vec::new();
        let mut point_colors = Vec::new();
        let mut series_spans = Vec::new();
        let mut data_min: Option<DVec2> = None;
        let mut data_max: Option<DVec2> = None;

        // Process each series
        for (id, series) in &widget.series {
            // Skip hidden series
            if widget.hidden_shapes.contains(id) {
                continue;
            }

            if series.positions.is_empty() {
                continue;
            }

            let start = points.len();
            let mut point_indices = Vec::new();

            // Add points and track bounds
            for (pos_index, &pos) in series.positions.iter().enumerate() {
                let Some(transformed) =
                    data_point_to_plot(pos, widget.x_axis_scale, widget.y_axis_scale)
                else {
                    continue;
                };
                let p = DVec2::new(transformed[0], transformed[1]);
                data_min = Some(data_min.map_or(p, |m| m.min(p)));
                data_max = Some(data_max.map_or(p, |m| m.max(p)));

                // Only create points if we have markers OR lines (lines need points for geometry)
                if series.marker_style.is_some() || series.line_style.is_some() {
                    let (size, size_mode) = series
                        .marker_style
                        .as_ref()
                        .map(|ms| ms.size.to_raw())
                        .unwrap_or((1.0, crate::point::MARKER_SIZE_PIXELS));
                    let color = series
                        .point_colors
                        .as_ref()
                        .and_then(|colors| colors.get(pos_index))
                        .copied()
                        .unwrap_or(series.color);
                    points.push(Point {
                        position: transformed,
                        size,
                        size_mode,
                    });
                    point_colors.push(color);
                    point_indices.push(pos_index);
                }
            }

            let (color, marker) = series
                .marker_style
                .as_ref()
                .map(|m| (m.marker_type as u32,))
                .map(|(marker,)| (series.color, marker))
                .unwrap_or((series.color, u32::MAX));

            series_spans.push(SeriesSpan {
                id: *id,
                start,
                len: points.len() - start,
                point_indices: point_indices.into(),
                line_style: series.line_style,
                color,
                marker,
            });

            // If this series has a world-space marker, the data_max should be adjusted to account for the marker size.
            if let Some(size) = series.marker_style.as_ref().and_then(|m| match m.size {
                MarkerSize::World(size) => Some(size),
                MarkerSize::Pixels(_) => None,
            }) && let Some(data_max) = &mut data_max
            {
                if widget.x_axis_scale == AxisScale::Linear {
                    data_max.x += size;
                }
                if widget.y_axis_scale == AxisScale::Linear {
                    data_max.y += size;
                }
            }
        }

        // Filter visible reference lines
        let vlines: Vec<_> = widget
            .vlines
            .iter()
            .filter(|(id, _)| !widget.hidden_shapes.contains(id))
            .map(|(_, v)| v.clone())
            .collect();

        let hlines: Vec<_> = widget
            .hlines
            .iter()
            .filter(|(id, _)| !widget.hidden_shapes.contains(id))
            .map(|(_, h)| h.clone())
            .collect();

        let x_domain = plot_x_domain(widget, data_min, data_max);
        let y_domain = plot_y_domain(widget, data_min, data_max);

        let fills: Vec<_> = widget
            .fills
            .iter()
            .filter(|(fill_id, fill)| {
                !widget.hidden_shapes.contains(fill_id)
                    && !widget.hidden_shapes.contains(&fill.begin)
                    && !widget.hidden_shapes.contains(&fill.end)
            })
            .filter_map(|(_, fill)| {
                build_fill_span(widget, fill.begin, fill.end, fill.color, x_domain, y_domain)
                    .filter(|span| !span.vertices.is_empty())
            })
            .collect();

        self.points = points.into();
        self.point_colors = point_colors.into();
        self.series = series_spans.into();
        self.fills = fills.into();
        self.vlines = vlines.into();
        self.hlines = hlines.into();
        self.data_min = data_min;
        self.data_max = data_max;
        self.legend_collapsed = widget.legend_collapsed;
        self.x_lim = widget.x_lim;
        self.y_lim = widget.y_lim;
        self.x_axis_scale = widget.x_axis_scale;
        self.y_axis_scale = widget.y_axis_scale;
        self.x_axis_link = widget.x_axis_link.clone();
        self.y_axis_link = widget.y_axis_link.clone();

        // highlighted_points
        self.sync_highlighted_points_from_widget(widget);
        self.highlight_src_version = widget.highlight_version;

        // Copy formatters
        self.x_axis_formatter = widget.x_axis_formatter.clone();
        self.y_axis_formatter = widget.y_axis_formatter.clone();

        // Force GPU buffers to rebuild only when data actually changes
        // (not when only hover/pick changes - that's tracked by highlight_version)
        self.markers_version = self.markers_version.wrapping_add(1);
        self.lines_version = self.lines_version.wrapping_add(1);
        self.fills_version = self.fills_version.wrapping_add(1);
    }

    pub(crate) fn autoscale(&mut self, update_axis_links: bool) {
        // Use user-specified limits if available, otherwise use data bounds
        let mut min_v = DVec2::new(-1.0, -1.0);
        let mut max_v = DVec2::new(1.0, 1.0);

        if let (Some(data_min), Some(data_max)) = (self.data_min, self.data_max) {
            min_v = data_min;
            max_v = data_max;
        }

        if let Some((y_min, y_max)) = self.y_lim
            && let (Some(y_min), Some(y_max)) = (
                self.y_axis_scale.data_to_plot(y_min),
                self.y_axis_scale.data_to_plot(y_max),
            )
        {
            min_v.y = y_min;
            max_v.y = y_max;
        }

        if let Some((x_min, x_max)) = self.x_lim
            && let (Some(x_min), Some(x_max)) = (
                self.x_axis_scale.data_to_plot(x_min),
                self.x_axis_scale.data_to_plot(x_max),
            )
        {
            min_v.x = x_min;
            max_v.x = x_max;
        }

        self.camera.set_bounds(min_v, max_v, 0.05);
        if update_axis_links {
            self.update_axis_links();
        }
    }

    pub(crate) fn update_ticks(
        &mut self,
        x_tick_producer: Option<&TickProducer>,
        y_tick_producer: Option<&TickProducer>,
    ) {
        // Calculate x-axis ticks
        let min_x_plot = self.camera.position.x - self.camera.half_extents.x;
        let max_x_plot = self.camera.position.x + self.camera.half_extents.x;
        let min_x = self
            .x_axis_scale
            .plot_to_data(min_x_plot)
            .unwrap_or(min_x_plot);
        let max_x = self
            .x_axis_scale
            .plot_to_data(max_x_plot)
            .unwrap_or(max_x_plot);

        let x_tick_values = match x_tick_producer {
            Some(producer) => producer(min_x, max_x),
            None => Vec::new(),
        };

        self.x_ticks.clear();
        for tick in x_tick_values {
            let Some(tick_plot) = self.x_axis_scale.data_to_plot(tick.value) else {
                continue;
            };
            // Convert world position to screen position
            if let Some(screen_pos) =
                world_to_screen_position_x(tick_plot, &self.camera, &self.bounds)
            {
                self.x_ticks.push(PositionedTick { screen_pos, tick });
            }
        }

        // Calculate y-axis ticks
        let min_y_plot = self.camera.position.y - self.camera.half_extents.y;
        let max_y_plot = self.camera.position.y + self.camera.half_extents.y;
        let min_y = self
            .y_axis_scale
            .plot_to_data(min_y_plot)
            .unwrap_or(min_y_plot);
        let max_y = self
            .y_axis_scale
            .plot_to_data(max_y_plot)
            .unwrap_or(max_y_plot);

        let y_tick_values = match y_tick_producer {
            Some(producer) => producer(min_y, max_y),
            None => Vec::new(),
        };

        self.y_ticks.clear();
        for tick in y_tick_values {
            let Some(tick_plot) = self.y_axis_scale.data_to_plot(tick.value) else {
                continue;
            };
            // Convert world position to screen position
            if let Some(screen_pos) =
                world_to_screen_position_y(tick_plot, &self.camera, &self.bounds)
            {
                self.y_ticks.push(PositionedTick { screen_pos, tick });
            }
        }
    }

    pub(crate) fn point_inside(&self, x: f32, y: f32) -> bool {
        x >= 0.0 && y >= 0.0 && x <= self.bounds.width && y <= self.bounds.height
    }

    pub(crate) fn cursor_inside(&self) -> bool {
        self.point_inside(self.cursor_position.x, self.cursor_position.y)
    }

    pub(crate) fn handle_mouse_event(
        &mut self,
        event: Event,
        cursor: mouse::Cursor,
        widget: &PlotWidget,
        publish_hover_pick: &mut Option<HoverPickEvent>,
    ) -> bool {
        const SELECTION_DELTA_THRESHOLD: f32 = 4.0; // pixels
        const SELECTION_PADDING: f32 = 0.02; // fractional padding in world units relative to selection size

        // Only request redraws when something actually changes or when we need
        // to service a picking request for a new cursor position.
        let mut needs_redraw = false;

        let viewport: DVec2 = Vec2::new(self.bounds.width, self.bounds.height).into();

        match event {
            Event::CursorMoved { mut position } => {
                if let mouse::Cursor::Available(p) | mouse::Cursor::Levitating(p) = cursor {
                    // cursor position can consider the scrolled offset
                    position = p;
                }
                // Check if the cursor is inside this widget's bounds in window space
                let inside = self.point_inside(position.x, position.y);

                // Store cursor in local coordinates (relative to bounds)
                self.cursor_position =
                    Vec2::new(position.x - self.bounds.x, position.y - self.bounds.y);
                // Update crosshairs position when enabled
                if widget.crosshairs_enabled {
                    self.crosshairs_position = self.cursor_position;
                    needs_redraw = true;
                }

                // Handle selection (right click drag)
                if self.selection.active {
                    self.selection.end = self.cursor_position;
                    self.selection.moved = true;
                    needs_redraw = true;
                }

                // Handle panning (left click drag)
                if self.pan.active {
                    // Convert screen positions to render coordinates (without offset)
                    let render_current = self.camera.screen_to_render(
                        DVec2::new(self.cursor_position.x as f64, self.cursor_position.y as f64),
                        viewport,
                    );
                    let render_start = self
                        .camera
                        .screen_to_render(self.pan.start_cursor, viewport);
                    let render_delta = render_current - render_start;

                    // Update camera position by applying the render space delta
                    self.camera.position = self.pan.start_camera_center - render_delta;
                    self.update_axis_links();
                    needs_redraw = true;
                }

                // Hover picking (only when not panning or selecting)
                if !self.pan.active && !self.selection.active && self.hover_enabled {
                    if !inside {
                        // If cursor leaves this widget, clear hover state for this widget only
                        if self.picking.last_hover_cache.is_some() {
                            self.picking.last_hover_cache = None;
                            // Redraw once to clear hover halo overlay
                            needs_redraw = true;
                        }
                        return needs_redraw;
                    } else {
                        // Inside bounds and hover enabled: request a redraw so the renderer
                        // can service the GPU picking request for this cursor position.
                        needs_redraw = true;
                    }
                }
            }
            Event::CursorLeft => {
                // Clear hover state on leave and request a redraw to clear hover halo
                if self.picking.last_hover_cache.is_some() {
                    self.picking.last_hover_cache = None;
                    needs_redraw = true;
                }
            }
            Event::ButtonPressed(mouse::Button::Left) => {
                // Only start panning if the press started inside our bounds
                // (Drags will continue even if the cursor leaves later)
                let inside = self.cursor_inside();
                if !inside {
                    return needs_redraw;
                }
                let now = Instant::now();
                let double = if let Some(prev) = self.last_click_time {
                    now.duration_since(prev).as_millis() < 350
                } else {
                    false
                };
                self.last_click_time = Some(now);
                if double {
                    self.autoscale(true);
                    needs_redraw = true;
                } else {
                    if self.hover_enabled && !self.pan.active && !self.selection.active {
                        // check if the cursor is hovering over a point
                        let picked =
                            if let Some(HoverPickEvent::Hover(point_id)) = *publish_hover_pick {
                                Some(point_id)
                            } else {
                                widget.pick_hit(self)
                            };

                        if let Some(point_id) = picked {
                            // Upgrade the "hover" to a "pick".
                            *publish_hover_pick = Some(HoverPickEvent::Pick(point_id));
                        }
                    }
                    // Start panning
                    self.pan.active = true;
                    self.pan.start_cursor = self.cursor_position.into();
                    self.pan.start_camera_center = self.camera.position;
                }
            }
            Event::ButtonReleased(mouse::Button::Left) => {
                if self.pan.active {
                    self.pan.active = false;
                }
            }
            Event::ButtonPressed(mouse::Button::Right) => {
                // Only start selection if inside our bounds
                let inside = self.cursor_inside();
                if !inside {
                    return needs_redraw;
                }
                // Start selection
                self.selection.active = true;
                self.selection.start = self.cursor_position;
                self.selection.end = self.cursor_position;
                self.selection.moved = false;
                needs_redraw = true;
            }
            Event::ButtonReleased(mouse::Button::Right) => {
                if self.selection.active {
                    self.selection.end = self.cursor_position;
                    let delta = self.selection.end - self.selection.start;
                    let dragged = delta.length() > SELECTION_DELTA_THRESHOLD;
                    // Perform zoom if user actually dragged a region of non-trivial size
                    if dragged {
                        // Convert screen (pixels) to world coords using camera helper
                        let p1 = self.camera.screen_to_world(
                            DVec2::new(
                                self.selection.start.x as f64,
                                self.selection.start.y as f64,
                            ),
                            viewport,
                        );
                        let p2 = self.camera.screen_to_world(
                            DVec2::new(self.selection.end.x as f64, self.selection.end.y as f64),
                            viewport,
                        );
                        let min_v = DVec2::new(p1.x.min(p2.x), p1.y.min(p2.y));
                        let max_v = DVec2::new(p1.x.max(p2.x), p1.y.max(p2.y));
                        // Use set_bounds_preserve_offset to avoid changing the render_offset during zoom
                        self.camera.set_bounds_preserve_offset(
                            min_v,
                            max_v,
                            SELECTION_PADDING as f64,
                        );
                        self.update_axis_links();
                    }
                    // Clear selection overlay after release
                    self.selection.active = false;
                    self.selection.moved = false;
                    needs_redraw = true;
                }
            }
            Event::WheelScrolled { delta } => {
                // Only respond to wheel when cursor is inside our bounds
                let inside = self.cursor_inside();
                if !inside {
                    return needs_redraw;
                }

                let (x, y) = match delta {
                    iced::mouse::ScrollDelta::Lines { x, y } => (x, y),
                    iced::mouse::ScrollDelta::Pixels { x, y } => (x, y),
                };

                // Only zoom when Ctrl is held down
                if self.modifiers.contains(keyboard::Modifiers::CTRL) {
                    // Apply zoom factor based on scroll direction
                    let zoom_factor = if y > 0.0 { 0.95 } else { 1.05 };

                    // Convert cursor position to render coordinates before zoom (without offset)
                    let cursor_render_before = self.camera.screen_to_render(
                        DVec2::new(self.cursor_position.x as f64, self.cursor_position.y as f64),
                        viewport,
                    );

                    // Apply zoom by scaling half_extents
                    self.camera.half_extents *= zoom_factor;

                    // Convert cursor position to render coordinates after zoom
                    let cursor_render_after = self.camera.screen_to_render(
                        DVec2::new(self.cursor_position.x as f64, self.cursor_position.y as f64),
                        viewport,
                    );

                    // Adjust camera position (in render space) to keep cursor at same position
                    let render_delta = cursor_render_before - cursor_render_after;
                    // Convert render delta back to world space and adjust camera position
                    self.camera.position += render_delta;

                    self.update_axis_links();
                    needs_redraw = true;
                } else if widget.scroll_to_pan_enabled {
                    let world_pan_x = -x as f64 * (self.camera.half_extents.x / (viewport.x / 2.0));
                    let world_pan_y = y as f64 * (self.camera.half_extents.y / (viewport.y / 2.0));
                    self.camera.position.x += world_pan_x;
                    self.camera.position.y += world_pan_y;
                    self.update_axis_links();
                    needs_redraw = true;
                }
            }
            _ => {}
        }

        // camera uniform is handled in renderer per frame
        needs_redraw
    }

    pub(crate) fn handle_keyboard_event(&mut self, event: &keyboard::Event) -> bool {
        if let keyboard::Event::ModifiersChanged(modifiers) = event {
            self.modifiers = *modifiers;
        }
        false // No need to redraw
    }

    fn update_axis_links(&mut self) {
        if let Some(ref link) = self.x_axis_link {
            link.set(self.camera.position.x, self.camera.half_extents.x);
            self.x_link_version = link.version();
        }
        if let Some(ref link) = self.y_axis_link {
            link.set(self.camera.position.y, self.camera.half_extents.y);
            self.y_link_version = link.version();
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FillSpan {
    pub(crate) color: Color,
    /// Triangle list vertices in plot/world coordinates.
    pub(crate) vertices: Arc<[[f64; 2]]>,
}

enum FillEndpoint<'a> {
    Series(&'a crate::Series),
    HLine(f64),
    VLine(f64),
}

fn resolve_fill_endpoint<'a>(widget: &'a PlotWidget, id: ShapeId) -> Option<FillEndpoint<'a>> {
    if let Some(series) = widget.series.get(&id) {
        return Some(FillEndpoint::Series(series));
    }
    if let Some(hline) = widget.hlines.get(&id) {
        return Some(FillEndpoint::HLine(hline.y));
    }
    if let Some(vline) = widget.vlines.get(&id) {
        return Some(FillEndpoint::VLine(vline.x));
    }
    None
}

fn plot_x_domain(
    widget: &PlotWidget,
    data_min: Option<DVec2>,
    data_max: Option<DVec2>,
) -> Option<(f64, f64)> {
    if let Some((min, max)) = widget.x_lim
        && let (Some(min), Some(max)) = (
            widget.x_axis_scale.data_to_plot(min),
            widget.x_axis_scale.data_to_plot(max),
        )
    {
        return (min < max).then_some((min, max));
    }
    match (data_min, data_max) {
        (Some(min), Some(max)) if min.x < max.x => Some((min.x, max.x)),
        _ => None,
    }
}

fn plot_y_domain(
    widget: &PlotWidget,
    data_min: Option<DVec2>,
    data_max: Option<DVec2>,
) -> Option<(f64, f64)> {
    if let Some((min, max)) = widget.y_lim
        && let (Some(min), Some(max)) = (
            widget.y_axis_scale.data_to_plot(min),
            widget.y_axis_scale.data_to_plot(max),
        )
    {
        return (min < max).then_some((min, max));
    }
    match (data_min, data_max) {
        (Some(min), Some(max)) if min.y < max.y => Some((min.y, max.y)),
        _ => None,
    }
}

fn transformed_series_points(
    series: &crate::Series,
    x_axis_scale: AxisScale,
    y_axis_scale: AxisScale,
) -> Vec<[f64; 2]> {
    series
        .positions
        .iter()
        .filter_map(|&p| data_point_to_plot(p, x_axis_scale, y_axis_scale))
        .collect()
}

/// Keep only strictly increasing-x points in their original order.
///
/// This avoids sorting and lets fill interpolation run in linear time.
/// Out-of-order (or duplicate-x) points are skipped.
fn monotonic_increasing_x(points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(points.len());
    let mut last_x: Option<f64> = None;
    for p in points {
        match last_x {
            Some(x_prev) if p[0] <= x_prev => {}
            _ => {
                last_x = Some(p[0]);
                out.push(p);
            }
        }
    }
    out
}

fn find_segment_covering_x(points: &[[f64; 2]], x: f64) -> Option<usize> {
    if points.len() < 2 {
        return None;
    }
    let eps = 1e-12;
    let mut idx = 0usize;
    while idx + 1 < points.len() {
        let x0 = points[idx][0];
        let x1 = points[idx + 1][0];
        if x >= x0 - eps && x <= x1 + eps {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn y_at_x_in_segment(points: &[[f64; 2]], seg_idx: usize, x: f64) -> Option<f64> {
    let p0 = *points.get(seg_idx)?;
    let p1 = *points.get(seg_idx + 1)?;
    let x0 = p0[0];
    let x1 = p1[0];
    let eps = 1e-9;
    if x < x0 - eps || x > x1 + eps {
        return None;
    }
    let dx = x1 - x0;
    if dx.abs() <= f64::EPSILON {
        return Some((p0[1] + p1[1]) * 0.5);
    }
    let t = (x - x0) / dx;
    Some(p0[1] + t * (p1[1] - p0[1]))
}

fn advance_segment_to_x(points: &[[f64; 2]], seg_idx: &mut usize, x: f64) {
    let eps = 1e-12;
    while *seg_idx + 2 <= points.len().saturating_sub(1) && points[*seg_idx + 1][0] <= x + eps {
        *seg_idx += 1;
    }
}

fn push_quad_as_triangles(
    vertices: &mut Vec<[f64; 2]>,
    a0: [f64; 2],
    b0: [f64; 2],
    a1: [f64; 2],
    b1: [f64; 2],
) {
    vertices.extend_from_slice(&[a0, b0, a1, a1, b0, b1]);
}

fn build_fill_span(
    widget: &PlotWidget,
    begin: ShapeId,
    end: ShapeId,
    color: Color,
    x_domain: Option<(f64, f64)>,
    y_domain: Option<(f64, f64)>,
) -> Option<FillSpan> {
    let begin_endpoint = resolve_fill_endpoint(widget, begin)?;
    let end_endpoint = resolve_fill_endpoint(widget, end)?;

    let mut vertices: Vec<[f64; 2]> = Vec::new();

    match (begin_endpoint, end_endpoint) {
        (FillEndpoint::Series(sa), FillEndpoint::Series(sb)) => {
            let a = monotonic_increasing_x(transformed_series_points(
                sa,
                widget.x_axis_scale,
                widget.y_axis_scale,
            ));
            let b = monotonic_increasing_x(transformed_series_points(
                sb,
                widget.x_axis_scale,
                widget.y_axis_scale,
            ));
            if a.len() < 2 || b.len() < 2 {
                return None;
            }

            let overlap_min = a.first()?[0].max(b.first()?[0]);
            let overlap_max = a.last()?[0].min(b.last()?[0]);
            if overlap_min >= overlap_max {
                return None;
            }

            let mut seg_a = find_segment_covering_x(&a, overlap_min)?;
            let mut seg_b = find_segment_covering_x(&b, overlap_min)?;

            let mut x_curr = overlap_min;
            let mut y_a_curr = y_at_x_in_segment(&a, seg_a, x_curr)?;
            let mut y_b_curr = y_at_x_in_segment(&b, seg_b, x_curr)?;

            let eps = 1e-12;
            loop {
                let next_a = a.get(seg_a + 1).map(|p| p[0]).unwrap_or(f64::INFINITY);
                let next_b = b.get(seg_b + 1).map(|p| p[0]).unwrap_or(f64::INFINITY);
                let x_next = next_a.min(next_b).min(overlap_max);

                if x_next <= x_curr + eps {
                    break;
                }

                let y_a_next = y_at_x_in_segment(&a, seg_a, x_next)?;
                let y_b_next = y_at_x_in_segment(&b, seg_b, x_next)?;

                push_quad_as_triangles(
                    &mut vertices,
                    [x_curr, y_a_curr],
                    [x_curr, y_b_curr],
                    [x_next, y_a_next],
                    [x_next, y_b_next],
                );

                x_curr = x_next;
                y_a_curr = y_a_next;
                y_b_curr = y_b_next;

                if x_curr >= overlap_max - eps {
                    break;
                }

                advance_segment_to_x(&a, &mut seg_a, x_curr);
                advance_segment_to_x(&b, &mut seg_b, x_curr);

                if seg_a + 1 >= a.len() || seg_b + 1 >= b.len() {
                    break;
                }
            }
        }
        (FillEndpoint::Series(series), FillEndpoint::HLine(y_data))
        | (FillEndpoint::HLine(y_data), FillEndpoint::Series(series)) => {
            let y_plot = widget.y_axis_scale.data_to_plot(y_data)?;
            let points =
                transformed_series_points(series, widget.x_axis_scale, widget.y_axis_scale);
            for segment in points.windows(2) {
                let p0 = segment[0];
                let p1 = segment[1];
                let q0 = [p0[0], y_plot];
                let q1 = [p1[0], y_plot];
                push_quad_as_triangles(&mut vertices, p0, q0, p1, q1);
            }
        }
        (FillEndpoint::Series(series), FillEndpoint::VLine(x_data))
        | (FillEndpoint::VLine(x_data), FillEndpoint::Series(series)) => {
            let x_plot = widget.x_axis_scale.data_to_plot(x_data)?;
            let points =
                transformed_series_points(series, widget.x_axis_scale, widget.y_axis_scale);
            for segment in points.windows(2) {
                let p0 = segment[0];
                let p1 = segment[1];
                let q0 = [x_plot, p0[1]];
                let q1 = [x_plot, p1[1]];
                push_quad_as_triangles(&mut vertices, p0, q0, p1, q1);
            }
        }
        (FillEndpoint::HLine(y0_data), FillEndpoint::HLine(y1_data)) => {
            let (x0, x1) = x_domain?;
            let y0 = widget.y_axis_scale.data_to_plot(y0_data)?;
            let y1 = widget.y_axis_scale.data_to_plot(y1_data)?;
            push_quad_as_triangles(&mut vertices, [x0, y0], [x0, y1], [x1, y0], [x1, y1]);
        }
        (FillEndpoint::VLine(x0_data), FillEndpoint::VLine(x1_data)) => {
            let (y0, y1) = y_domain?;
            let x0 = widget.x_axis_scale.data_to_plot(x0_data)?;
            let x1 = widget.x_axis_scale.data_to_plot(x1_data)?;
            push_quad_as_triangles(&mut vertices, [x0, y0], [x1, y0], [x0, y1], [x1, y1]);
        }
        _ => {
            return None;
        }
    }

    (!vertices.is_empty()).then_some(FillSpan {
        color,
        vertices: vertices.into(),
    })
}

#[derive(Debug, Clone)]
pub(crate) struct SeriesSpan {
    pub(crate) id: ShapeId,
    pub(crate) start: usize,
    pub(crate) len: usize,
    pub(crate) point_indices: Arc<[usize]>,
    pub(crate) line_style: Option<LineStyle>,
    pub(crate) color: Color,
    pub(crate) marker: u32,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct SelectionState {
    pub(crate) active: bool,
    pub(crate) start: Vec2,
    pub(crate) end: Vec2,
    pub(crate) moved: bool,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct PanState {
    pub(crate) active: bool,
    pub(crate) start_cursor: DVec2,
    pub(crate) start_camera_center: DVec2,
}
