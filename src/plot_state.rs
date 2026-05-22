use std::sync::Arc;

use glam::{DVec2, Vec2};
use iced::{
    Color, Rectangle, keyboard,
    mouse::{self, Event},
    time::Instant,
};

use crate::{
    AxisLink, AxisScale, ClickAction, DragAction, DragEvent, HLine, HoverPickEvent, KeyAction,
    LineStyle, PanDirection, PlotWidget, Point, ScrollAction, ShapeId, Size, VLine,
    axis_scale::plot_point_to_data,
    camera::Camera,
    picking::PickingState,
    plot_widget::{HighlightPoint, world_to_screen_position_x, world_to_screen_position_y},
    style::GridStyle,
    ticks::{PositionedTick, TickFormatter, TickProducer},
    transform::{data_point_to_plot_with_transform, data_value_to_plot_with_axis_range},
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
    pub(crate) grid_style: GridStyle,
    // Interaction state
    pub(crate) cursor_position: Vec2,
    pub(crate) last_click_time: Option<Instant>,
    pub(crate) last_click_button: Option<mouse::Button>,
    pub(crate) legend_collapsed: bool,
    pub(crate) modifiers: keyboard::Modifiers,
    pub(crate) press: ButtonPressState,
    pub(crate) selection: SelectionState,
    pub(crate) pan: PanState,
    pub(crate) drag: DragState,
    /// Hover/select point rendering data (for incremental rendering)
    pub(crate) highlighted_points: Arc<[HighlightPoint]>,
    // Version counters
    pub(crate) markers_version: u64,
    pub(crate) lines_version: u64,
    pub(crate) fills_version: u64,
    pub(crate) highlight_version: u64,
    pub(crate) data_src_version: u64, // version of source data last synced
    pub(crate) source_instance_id: Option<u64>,
    // Hover/picking internals
    pub(crate) hover_enabled: bool,
    pub(crate) pick_enabled: bool,
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
            grid_style: GridStyle::default(),
            cursor_position: Vec2::ZERO,
            last_click_time: None,
            last_click_button: None,
            legend_collapsed: false,
            modifiers: keyboard::Modifiers::default(),
            press: ButtonPressState::default(),
            selection: SelectionState::default(),
            pan: PanState::default(),
            drag: DragState::default(),
            markers_version: 1,
            lines_version: 1,
            fills_version: 1,
            highlight_version: 0,
            hover_enabled: true,
            pick_enabled: true,
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
            .map(|(highlight_point, _)| highlight_point.clone())
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
        let mut data_min_x: Option<f64> = None;
        let mut data_max_x: Option<f64> = None;
        let mut data_min_y: Option<f64> = None;
        let mut data_max_y: Option<f64> = None;
        let axis_ranges = self.camera.axis_ranges();

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
            let x_uses_axes = series
                .transform
                .x
                .as_ref()
                .is_some_and(|transform| transform.uses_axes_coordinates());
            let y_uses_axes = series
                .transform
                .y
                .as_ref()
                .is_some_and(|transform| transform.uses_axes_coordinates());

            // Add points and track bounds
            for (pos_index, &pos) in series.positions.iter().enumerate() {
                let Some(transformed) = data_point_to_plot_with_transform(
                    pos,
                    widget.x_axis_scale,
                    widget.y_axis_scale,
                    &series.transform,
                    Some(axis_ranges),
                ) else {
                    continue;
                };

                if !x_uses_axes {
                    data_min_x =
                        Some(data_min_x.map_or(transformed[0], |min| min.min(transformed[0])));
                    data_max_x =
                        Some(data_max_x.map_or(transformed[0], |max| max.max(transformed[0])));
                }
                if !y_uses_axes {
                    data_min_y =
                        Some(data_min_y.map_or(transformed[1], |min| min.min(transformed[1])));
                    data_max_y =
                        Some(data_max_y.map_or(transformed[1], |max| max.max(transformed[1])));
                }

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
                pickable: series.pickable,
            });

            // If this series has a world-space marker, the data_max should be adjusted to account for the marker size.
            if let Some(size) = series.marker_style.as_ref().and_then(|m| match m.size {
                Size::World(size) => Some(size),
                Size::Pixels(_) => None,
            }) {
                if !x_uses_axes
                    && widget.x_axis_scale == AxisScale::Linear
                    && let Some(max) = &mut data_max_x
                {
                    *max += size;
                }
                if !y_uses_axes
                    && widget.y_axis_scale == AxisScale::Linear
                    && let Some(max) = &mut data_max_y
                {
                    *max += size;
                }
            }
        }

        let data_min = (data_min_x.is_some() || data_min_y.is_some())
            .then(|| DVec2::new(data_min_x.unwrap_or(-1.0), data_min_y.unwrap_or(-1.0)));
        let data_max = (data_max_x.is_some() || data_max_y.is_some())
            .then(|| DVec2::new(data_max_x.unwrap_or(1.0), data_max_y.unwrap_or(1.0)));

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
                build_fill_span(
                    widget,
                    fill.begin,
                    fill.end,
                    fill.color,
                    x_domain,
                    y_domain,
                    axis_ranges,
                )
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

    fn cursor_local_position(&self, cursor: mouse::Cursor, allow_levitating: bool) -> Option<Vec2> {
        let position = match cursor {
            mouse::Cursor::Available(position) => position,
            mouse::Cursor::Levitating(position) if allow_levitating => position,
            mouse::Cursor::Levitating(_) | mouse::Cursor::Unavailable => return None,
        };

        Some(Vec2::new(
            position.x - self.bounds.x,
            position.y - self.bounds.y,
        ))
    }

    fn available_cursor_local_position_inside(&self, cursor: mouse::Cursor) -> Option<Vec2> {
        let position = self.cursor_local_position(cursor, false)?;
        self.point_inside(position.x, position.y)
            .then_some(position)
    }

    pub(crate) fn available_cursor_is_inside(&self, cursor: mouse::Cursor) -> bool {
        self.available_cursor_local_position_inside(cursor)
            .is_some()
    }

    pub(crate) fn drag_in_progress(&self) -> bool {
        self.pan.active || self.selection.active || self.drag.active
    }

    fn drag_in_progress_for(&self, button: mouse::Button) -> bool {
        (self.pan.active && self.pan.button == Some(button))
            || (self.selection.active && self.selection.button == Some(button))
            || (self.drag.active && self.drag.button == Some(button))
    }

    pub(crate) fn handle_mouse_event(
        &mut self,
        event: Event,
        cursor: mouse::Cursor,
        widget: &PlotWidget,
        publish_hover_pick: &mut Option<HoverPickEvent>,
        publish_drag_event: &mut Option<DragEvent>,
    ) -> bool {
        // Only request redraws when something actually changes or when we need
        // to service a picking request for a new cursor position.
        let mut needs_redraw = false;

        let viewport: DVec2 = Vec2::new(self.bounds.width, self.bounds.height).into();

        match event {
            Event::CursorMoved { .. } => {
                let Some(position) = self.cursor_local_position(cursor, self.drag_in_progress())
                else {
                    if self.picking.last_hover_cache.is_some() {
                        self.picking.last_hover_cache = None;
                        needs_redraw = true;
                    }
                    return needs_redraw;
                };
                let inside = self.point_inside(position.x, position.y);

                self.cursor_position = position;
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

                if self.drag.active
                    && let Some(button) = self.drag.button
                    && let Some(world) = self.cursor_world_data(viewport)
                {
                    *publish_drag_event = Some(DragEvent::Update { button, world });
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
            Event::ButtonPressed(button) => {
                // Only start button-driven interactions when the press starts
                // inside our bounds. Drags continue even if the cursor leaves.
                let Some(cursor_position) = self.available_cursor_local_position_inside(cursor)
                else {
                    return needs_redraw;
                };

                self.cursor_position = cursor_position;

                self.press.active = true;
                self.press.button = Some(button);
                self.press.start = self.cursor_position;

                let double_click_pending = self.is_double_click(button)
                    && widget.controls.double_click_action(button).is_some();

                if !double_click_pending {
                    match widget.controls.drag_action(button) {
                        Some(DragAction::BoxZoom) => {
                            self.selection.active = true;
                            self.selection.button = Some(button);
                            self.selection.start = self.cursor_position;
                            self.selection.end = self.cursor_position;
                            self.selection.moved = false;
                            needs_redraw = true;
                        }
                        Some(DragAction::Pan) => {
                            self.pan.active = true;
                            self.pan.button = Some(button);
                            self.pan.start_cursor = self.cursor_position.into();
                            self.pan.start_camera_center = self.camera.position;
                        }
                        _ => {}
                    }

                    if widget.controls.drag_action(button).is_none() {
                        self.drag.active = true;
                        self.drag.button = Some(button);
                        if let Some(world) = self.cursor_world_data(viewport) {
                            *publish_drag_event = Some(DragEvent::Start { button, world });
                        }
                    }
                }
            }
            Event::ButtonReleased(button) => {
                let drag_release = self.drag_in_progress_for(button);
                let release_position_available = if let Some(cursor_position) = if drag_release {
                    self.cursor_local_position(cursor, true)
                } else {
                    self.available_cursor_local_position_inside(cursor)
                } {
                    self.cursor_position = cursor_position;
                    true
                } else if !drag_release {
                    if self.press.button == Some(button) {
                        self.press.active = false;
                        self.press.button = None;
                    }
                    return needs_redraw;
                } else {
                    false
                };

                let click_candidate = release_position_available
                    && self.press.button == Some(button)
                    && (self.cursor_position - self.press.start).length()
                        <= widget.controls.drag_delta_threshold();

                if self.drag.active
                    && self.drag.button == Some(button)
                    && let Some(world) = self.cursor_world_data(viewport)
                {
                    *publish_drag_event = Some(DragEvent::End { button, world });
                    self.drag.active = false;
                    self.drag.button = None;
                }
                if self.drag.button == Some(button) {
                    self.drag.active = false;
                    self.drag.button = None;
                }
                if self.pan.active && self.pan.button == Some(button) {
                    self.pan.active = false;
                    self.pan.button = None;
                }
                if self.selection.active && self.selection.button == Some(button) {
                    self.selection.end = self.cursor_position;
                    let delta = self.selection.end - self.selection.start;
                    let dragged = delta.length() > widget.controls.drag_delta_threshold();
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
                            widget.controls.selection_padding(),
                        );
                        self.update_axis_links();
                    }
                    // Clear selection overlay after release
                    self.selection.active = false;
                    self.selection.button = None;
                    self.selection.moved = false;
                    needs_redraw = true;
                }

                if click_candidate && self.handle_mouse_click(button, widget, publish_hover_pick) {
                    needs_redraw = true;
                }

                if self.press.button == Some(button) {
                    self.press.active = false;
                    self.press.button = None;
                }
            }
            Event::WheelScrolled { delta } => {
                // Only respond to wheel when cursor is inside our bounds
                let Some(cursor_position) = self.available_cursor_local_position_inside(cursor)
                else {
                    return needs_redraw;
                };

                self.cursor_position = cursor_position;

                let (x, y) = match delta {
                    iced::mouse::ScrollDelta::Lines { x, y } => (x, y),
                    iced::mouse::ScrollDelta::Pixels { x, y } => (x, y),
                };

                match widget.controls.scroll_action(self.modifiers) {
                    Some(ScrollAction::Zoom) => {
                        self.zoom_at_cursor(y, viewport);
                        needs_redraw = true;
                    }
                    Some(ScrollAction::Pan) => {
                        let world_pan_x =
                            -x as f64 * (self.camera.half_extents.x / (viewport.x / 2.0));
                        let world_pan_y =
                            y as f64 * (self.camera.half_extents.y / (viewport.y / 2.0));
                        self.camera.position.x += world_pan_x;
                        self.camera.position.y += world_pan_y;
                        self.update_axis_links();
                        needs_redraw = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // camera uniform is handled in renderer per frame
        needs_redraw
    }

    pub(crate) fn handle_keyboard_event(
        &mut self,
        event: &keyboard::Event,
        widget: &PlotWidget,
        cursor: mouse::Cursor,
    ) -> bool {
        let cursor_over = self.available_cursor_is_inside(cursor);

        if let keyboard::Event::ModifiersChanged(modifiers) = event {
            if cursor_over {
                self.modifiers = *modifiers;
            }
            return false;
        }

        let keyboard::Event::KeyPressed { key, .. } = event else {
            return false;
        };

        if !cursor_over {
            return false;
        }

        match widget.controls.key_action(key) {
            Some(KeyAction::Autoscale) => {
                self.autoscale(true);
                true
            }
            Some(KeyAction::PanBy {
                direction,
                fraction,
            }) => {
                self.pan_by(direction, fraction);
                true
            }
            _ => false,
        }
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

    fn is_double_click(&self, button: mouse::Button) -> bool {
        self.last_click_button == Some(button)
            && self
                .last_click_time
                .is_some_and(|prev| Instant::now().duration_since(prev).as_millis() < 350)
    }

    fn handle_mouse_click(
        &mut self,
        button: mouse::Button,
        widget: &PlotWidget,
        publish_hover_pick: &mut Option<HoverPickEvent>,
    ) -> bool {
        let now = Instant::now();
        let double = self.is_double_click(button);
        let double_action = widget.controls.double_click_action(button);
        let click_action = widget.controls.click_action(button);

        let handled = if double {
            double_action
                .or(click_action)
                .is_some_and(|action| self.apply_click_action(action, widget, publish_hover_pick))
        } else {
            click_action
                .is_some_and(|action| self.apply_click_action(action, widget, publish_hover_pick))
        };

        self.last_click_time = Some(now);
        self.last_click_button = Some(button);
        handled
    }

    fn apply_click_action(
        &mut self,
        action: ClickAction,
        widget: &PlotWidget,
        publish_hover_pick: &mut Option<HoverPickEvent>,
    ) -> bool {
        match action {
            ClickAction::Autoscale => {
                self.autoscale(true);
                true
            }
            ClickAction::Pick => {
                self.pick_highlighted_point(widget, publish_hover_pick);
                false
            }
            ClickAction::ClearPick => {
                *publish_hover_pick = Some(HoverPickEvent::ClearPick);
                false
            }
        }
    }

    fn pick_highlighted_point(
        &mut self,
        widget: &PlotWidget,
        publish_hover_pick: &mut Option<HoverPickEvent>,
    ) {
        if !self.pick_enabled || self.pan.active || self.selection.active {
            return;
        }

        let picked = if let Some(HoverPickEvent::Hover(point_id)) = *publish_hover_pick {
            Some(point_id)
        } else {
            widget.pick_hit(self)
        };

        if let Some(point_id) = picked {
            *publish_hover_pick = Some(HoverPickEvent::Pick(point_id));
        }
    }

    fn zoom_at_cursor(&mut self, scroll_y: f32, viewport: DVec2) {
        let zoom_factor = if scroll_y > 0.0 { 0.95 } else { 1.05 };

        let cursor_render_before = self.camera.screen_to_render(
            DVec2::new(self.cursor_position.x as f64, self.cursor_position.y as f64),
            viewport,
        );

        self.camera.half_extents *= zoom_factor;

        let cursor_render_after = self.camera.screen_to_render(
            DVec2::new(self.cursor_position.x as f64, self.cursor_position.y as f64),
            viewport,
        );

        self.camera.position += cursor_render_before - cursor_render_after;
        self.update_axis_links();
    }

    fn pan_by(&mut self, direction: PanDirection, fraction: f64) {
        let delta = self.camera.half_extents * (2.0 * fraction);
        let pan_delta = match direction {
            PanDirection::Left => DVec2::new(-delta.x, 0.0),
            PanDirection::Right => DVec2::new(delta.x, 0.0),
            PanDirection::Up => DVec2::new(0.0, delta.y),
            PanDirection::Down => DVec2::new(0.0, -delta.y),
        };

        self.camera.position += pan_delta;
        self.update_axis_links();
    }

    fn cursor_world_data(&self, viewport: DVec2) -> Option<[f64; 2]> {
        let plot = self.camera.screen_to_world(
            DVec2::new(self.cursor_position.x as f64, self.cursor_position.y as f64),
            viewport,
        );
        plot_point_to_data([plot.x, plot.y], self.x_axis_scale, self.y_axis_scale)
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
    HLine(&'a HLine),
    VLine(&'a VLine),
}

fn resolve_fill_endpoint<'a>(widget: &'a PlotWidget, id: ShapeId) -> Option<FillEndpoint<'a>> {
    if let Some(series) = widget.series.get(&id) {
        return Some(FillEndpoint::Series(series));
    }
    if let Some(hline) = widget.hlines.get(&id) {
        return Some(FillEndpoint::HLine(hline));
    }
    if let Some(vline) = widget.vlines.get(&id) {
        return Some(FillEndpoint::VLine(vline));
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
    axis_ranges: ([f64; 2], [f64; 2]),
) -> Vec<[f64; 2]> {
    series
        .positions
        .iter()
        .filter_map(|&p| {
            data_point_to_plot_with_transform(
                p,
                x_axis_scale,
                y_axis_scale,
                &series.transform,
                Some(axis_ranges),
            )
        })
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
    axis_ranges: ([f64; 2], [f64; 2]),
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
                axis_ranges,
            ));
            let b = monotonic_increasing_x(transformed_series_points(
                sb,
                widget.x_axis_scale,
                widget.y_axis_scale,
                axis_ranges,
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
        (FillEndpoint::Series(series), FillEndpoint::HLine(hline))
        | (FillEndpoint::HLine(hline), FillEndpoint::Series(series)) => {
            let y_plot = data_value_to_plot_with_axis_range(
                hline.y,
                widget.y_axis_scale,
                hline.transform.as_ref(),
                Some(axis_ranges.1),
            )?;
            let points = transformed_series_points(
                series,
                widget.x_axis_scale,
                widget.y_axis_scale,
                axis_ranges,
            );
            for segment in points.windows(2) {
                let p0 = segment[0];
                let p1 = segment[1];
                let q0 = [p0[0], y_plot];
                let q1 = [p1[0], y_plot];
                push_quad_as_triangles(&mut vertices, p0, q0, p1, q1);
            }
        }
        (FillEndpoint::Series(series), FillEndpoint::VLine(vline))
        | (FillEndpoint::VLine(vline), FillEndpoint::Series(series)) => {
            let x_plot = data_value_to_plot_with_axis_range(
                vline.x,
                widget.x_axis_scale,
                vline.transform.as_ref(),
                Some(axis_ranges.0),
            )?;
            let points = transformed_series_points(
                series,
                widget.x_axis_scale,
                widget.y_axis_scale,
                axis_ranges,
            );
            for segment in points.windows(2) {
                let p0 = segment[0];
                let p1 = segment[1];
                let q0 = [x_plot, p0[1]];
                let q1 = [x_plot, p1[1]];
                push_quad_as_triangles(&mut vertices, p0, q0, p1, q1);
            }
        }
        (FillEndpoint::HLine(hline0), FillEndpoint::HLine(hline1)) => {
            let (x0, x1) = x_domain?;
            let y0 = data_value_to_plot_with_axis_range(
                hline0.y,
                widget.y_axis_scale,
                hline0.transform.as_ref(),
                Some(axis_ranges.1),
            )?;
            let y1 = data_value_to_plot_with_axis_range(
                hline1.y,
                widget.y_axis_scale,
                hline1.transform.as_ref(),
                Some(axis_ranges.1),
            )?;
            push_quad_as_triangles(&mut vertices, [x0, y0], [x0, y1], [x1, y0], [x1, y1]);
        }
        (FillEndpoint::VLine(vline0), FillEndpoint::VLine(vline1)) => {
            let (y0, y1) = y_domain?;
            let x0 = data_value_to_plot_with_axis_range(
                vline0.x,
                widget.x_axis_scale,
                vline0.transform.as_ref(),
                Some(axis_ranges.0),
            )?;
            let x1 = data_value_to_plot_with_axis_range(
                vline1.x,
                widget.x_axis_scale,
                vline1.transform.as_ref(),
                Some(axis_ranges.0),
            )?;
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
    pub(crate) pickable: bool,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct ButtonPressState {
    pub(crate) active: bool,
    pub(crate) button: Option<mouse::Button>,
    pub(crate) start: Vec2,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct SelectionState {
    pub(crate) active: bool,
    pub(crate) button: Option<mouse::Button>,
    pub(crate) start: Vec2,
    pub(crate) end: Vec2,
    pub(crate) moved: bool,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct PanState {
    pub(crate) active: bool,
    pub(crate) button: Option<mouse::Button>,
    pub(crate) start_cursor: DVec2,
    pub(crate) start_camera_center: DVec2,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct DragState {
    pub(crate) active: bool,
    pub(crate) button: Option<mouse::Button>,
}

#[cfg(test)]
mod tests {
    use glam::DVec2;
    use iced::Point;

    use super::*;
    use crate::{PointId, Series};

    #[test]
    fn axes_transform_series_maps_to_camera_range_and_skips_autoscale_bounds() {
        let mut widget = PlotWidget::new();
        widget
            .add_series(Series::circles(vec![[0.4, 0.6]], 5.0).with_axes_transform())
            .unwrap();

        let mut state = PlotState::default();
        state.camera.position = DVec2::new(10.0, 20.0);
        state.camera.half_extents = DVec2::new(5.0, 10.0);

        state.rebuild_from_widget(&widget);

        assert_eq!(state.points[0].position, [9.0, 22.0]);
        assert_eq!(state.data_min, None);
        assert_eq!(state.data_max, None);
    }

    #[test]
    fn arrow_keys_use_configured_pan_fraction_when_enabled_and_hovered() {
        let mut widget = PlotWidget::new();
        widget.controls.bind_key(
            keyboard::Key::Named(keyboard::key::Named::ArrowRight),
            KeyAction::PanBy {
                direction: PanDirection::Right,
                fraction: 0.25,
            },
        );

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;
        state.camera.half_extents = DVec2::new(10.0, 20.0);

        let changed = state.handle_keyboard_event(
            &arrow_key_event(
                keyboard::key::Named::ArrowRight,
                keyboard::key::Code::ArrowRight,
            ),
            &widget,
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
        );

        assert!(changed);
        assert_eq!(state.camera.position, DVec2::new(5.0, 0.0));
    }

    #[test]
    fn arrow_keys_do_not_pan_when_disabled() {
        let mut widget = PlotWidget::new();
        widget.controls.unbind_arrow_pan();

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;
        state.camera.half_extents = DVec2::new(10.0, 20.0);

        let changed = state.handle_keyboard_event(
            &arrow_key_event(keyboard::key::Named::ArrowUp, keyboard::key::Code::ArrowUp),
            &widget,
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
        );

        assert!(!changed);
        assert_eq!(state.camera.position, DVec2::ZERO);
    }

    #[test]
    fn configured_mouse_button_starts_and_stops_drag_pan() {
        let mut widget = PlotWidget::new();
        widget
            .controls
            .set_drag_action(DragAction::Pan, Some(mouse::Button::Middle));

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            ..PlotState::default()
        };

        let mut hover_pick = None;
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::ButtonPressed(mouse::Button::Middle),
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(state.pan.active);
        assert_eq!(state.pan.button, Some(mouse::Button::Middle));

        state.handle_mouse_event(
            Event::ButtonReleased(mouse::Button::Middle),
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(!state.pan.active);
        assert_eq!(state.pan.button, None);
    }

    #[test]
    fn levitating_cursor_does_not_start_drag_pan() {
        let widget = PlotWidget::new();
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            ..PlotState::default()
        };

        let mut hover_pick = None;
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::ButtonPressed(mouse::Button::Left),
            mouse::Cursor::Levitating(Point::new(50.0, 350.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(!state.pan.active);
        assert_eq!(state.pan.button, None);
    }

    #[test]
    fn levitating_cursor_continues_active_drag_pan() {
        let widget = PlotWidget::new();
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;
        state.camera.half_extents = DVec2::new(10.0, 30.0);

        let mut hover_pick = None;
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::ButtonPressed(mouse::Button::Left),
            mouse::Cursor::Available(Point::new(50.0, 150.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        state.handle_mouse_event(
            Event::CursorMoved {
                position: Point::new(50.0, 450.0),
            },
            mouse::Cursor::Levitating(Point::new(50.0, 450.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(state.pan.active);
        assert_eq!(state.cursor_position, Vec2::new(50.0, 350.0));
        assert_ne!(state.camera.position, DVec2::ZERO);
    }

    #[test]
    fn levitating_cursor_does_not_scroll_pan() {
        let widget = PlotWidget::new();
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;
        state.camera.half_extents = DVec2::new(10.0, 30.0);

        let mut hover_pick = None;
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::WheelScrolled {
                delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
            },
            mouse::Cursor::Levitating(Point::new(50.0, 350.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert_eq!(state.camera.position, DVec2::ZERO);
    }

    #[test]
    fn levitating_cursor_does_not_release_click() {
        let widget = PlotWidget::new();
        let point_id = PointId {
            series_id: ShapeId::new(),
            point_index: 0,
        };
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            press: ButtonPressState {
                active: true,
                button: Some(mouse::Button::Left),
                start: Vec2::new(50.0, 50.0),
            },
            ..PlotState::default()
        };

        let mut hover_pick = Some(HoverPickEvent::Hover(point_id));
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::ButtonReleased(mouse::Button::Left),
            mouse::Cursor::Levitating(Point::new(50.0, 350.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(matches!(hover_pick, Some(HoverPickEvent::Hover(id)) if id == point_id));
        assert!(!state.press.active);
    }

    #[test]
    fn available_cursor_uses_absolute_bounds_for_drag_start() {
        let widget = PlotWidget::new();
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            ..PlotState::default()
        };

        let mut hover_pick = None;
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::ButtonPressed(mouse::Button::Left),
            mouse::Cursor::Available(Point::new(50.0, 350.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(state.pan.active);
        assert_eq!(state.pan.button, Some(mouse::Button::Left));
        assert_eq!(state.cursor_position, Vec2::new(50.0, 250.0));
    }

    #[test]
    fn levitating_cursor_does_not_enable_keyboard_pan() {
        let mut widget = PlotWidget::new();
        widget.controls.bind_key(
            keyboard::Key::Named(keyboard::key::Named::ArrowRight),
            KeyAction::PanBy {
                direction: PanDirection::Right,
                fraction: 0.25,
            },
        );

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;
        state.camera.half_extents = DVec2::new(10.0, 20.0);

        let changed = state.handle_keyboard_event(
            &arrow_key_event(
                keyboard::key::Named::ArrowRight,
                keyboard::key::Code::ArrowRight,
            ),
            &widget,
            mouse::Cursor::Levitating(Point::new(50.0, 350.0)),
        );

        assert!(!changed);
        assert_eq!(state.camera.position, DVec2::ZERO);
    }

    #[test]
    fn levitating_cursor_does_not_update_keyboard_modifiers() {
        let widget = PlotWidget::new();
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 100.0,
                width: 100.0,
                height: 300.0,
            },
            modifiers: keyboard::Modifiers::NONE,
            ..PlotState::default()
        };

        state.handle_keyboard_event(
            &keyboard::Event::ModifiersChanged(keyboard::Modifiers::CTRL),
            &widget,
            mouse::Cursor::Levitating(Point::new(50.0, 350.0)),
        );

        assert_eq!(state.modifiers, keyboard::Modifiers::NONE);
    }

    #[test]
    fn unbound_non_left_button_publishes_drag_events() {
        let widget = PlotWidget::new();
        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            ..PlotState::default()
        };

        let mut hover_pick = None;
        let mut drag_event = None;
        state.handle_mouse_event(
            Event::ButtonPressed(mouse::Button::Middle),
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(matches!(
            drag_event,
            Some(DragEvent::Start {
                button: mouse::Button::Middle,
                ..
            })
        ));
        assert!(state.drag.active);
        assert_eq!(state.drag.button, Some(mouse::Button::Middle));

        drag_event = None;
        state.handle_mouse_event(
            Event::ButtonReleased(mouse::Button::Middle),
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(matches!(
            drag_event,
            Some(DragEvent::End {
                button: mouse::Button::Middle,
                ..
            })
        ));
        assert!(!state.drag.active);
        assert_eq!(state.drag.button, None);
    }

    #[test]
    fn left_click_picks_when_left_button_is_box_zoom() {
        let mut widget = PlotWidget::new();
        widget
            .controls
            .set_drag_action(DragAction::Pan, Some(mouse::Button::Right))
            .set_drag_action(DragAction::BoxZoom, Some(mouse::Button::Left));

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            ..PlotState::default()
        };

        let point_id = PointId {
            series_id: ShapeId::new(),
            point_index: 0,
        };
        let mut hover_pick = Some(HoverPickEvent::Hover(point_id));
        let mut drag_event = None;

        state.handle_mouse_event(
            Event::ButtonPressed(mouse::Button::Left),
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        state.handle_mouse_event(
            Event::ButtonReleased(mouse::Button::Left),
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
            &widget,
            &mut hover_pick,
            &mut drag_event,
        );

        assert!(matches!(hover_pick, Some(HoverPickEvent::Pick(id)) if id == point_id));
        assert!(!state.selection.active);
    }

    #[test]
    fn left_double_click_autoscales_when_left_button_is_box_zoom() {
        let mut widget = PlotWidget::new();
        widget
            .controls
            .set_drag_action(DragAction::Pan, Some(mouse::Button::Right))
            .set_drag_action(DragAction::BoxZoom, Some(mouse::Button::Left));

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            cursor_position: Vec2::new(50.0, 50.0),
            data_min: Some(DVec2::new(10.0, 20.0)),
            data_max: Some(DVec2::new(30.0, 60.0)),
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;

        let mut hover_pick = None;
        let mut drag_event = None;
        for _ in 0..2 {
            state.handle_mouse_event(
                Event::ButtonPressed(mouse::Button::Left),
                mouse::Cursor::Available(Point::new(50.0, 50.0)),
                &widget,
                &mut hover_pick,
                &mut drag_event,
            );
            state.handle_mouse_event(
                Event::ButtonReleased(mouse::Button::Left),
                mouse::Cursor::Available(Point::new(50.0, 50.0)),
                &widget,
                &mut hover_pick,
                &mut drag_event,
            );
        }

        assert_eq!(state.camera.position, DVec2::new(20.0, 40.0));
        assert!(!state.selection.active);
    }

    #[test]
    fn configured_key_autoscales_when_hovered() {
        let mut widget = PlotWidget::new();
        widget.controls.bind_key(
            keyboard::Key::Named(keyboard::key::Named::Home),
            KeyAction::Autoscale,
        );

        let mut state = PlotState {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            data_min: Some(DVec2::new(10.0, 20.0)),
            data_max: Some(DVec2::new(30.0, 60.0)),
            ..PlotState::default()
        };
        state.camera.position = DVec2::ZERO;

        let changed = state.handle_keyboard_event(
            &key_event(
                keyboard::Key::Named(keyboard::key::Named::Home),
                keyboard::key::Code::Home,
            ),
            &widget,
            mouse::Cursor::Available(Point::new(50.0, 50.0)),
        );

        assert!(changed);
        assert_eq!(state.camera.position, DVec2::new(20.0, 40.0));
    }

    fn arrow_key_event(named: keyboard::key::Named, code: keyboard::key::Code) -> keyboard::Event {
        key_event(keyboard::Key::Named(named), code)
    }

    fn key_event(key: keyboard::Key, code: keyboard::key::Code) -> keyboard::Event {
        keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Code(code),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }
    }
}
