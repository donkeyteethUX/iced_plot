use std::sync::Arc;

use glam::{DVec2, Vec2};
use iced::{Color, Rectangle, keyboard, mouse, time::Instant};

use crate::{
    AxisLink, HLine, LineStyle, MarkerSize, PlotInputEvent, PlotPointerEvent, PlotWidget, Point,
    ShapeId, VLine,
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
    pub(crate) vlines: Arc<[VLine]>,       // vertical reference lines
    pub(crate) hlines: Arc<[HLine]>,       // horizontal reference lines
    pub(crate) data_min: Option<DVec2>,
    pub(crate) data_max: Option<DVec2>,
    // Axis limits
    pub(crate) x_lim: Option<(f64, f64)>,
    pub(crate) y_lim: Option<(f64, f64)>,
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
    pub(crate) highlight_version: u64,
    pub(crate) data_src_version: u64, // version of source data last synced
    pub(crate) highlight_src_version: u64,
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
            points: Arc::new([]),
            point_colors: Arc::new([]),
            highlighted_points: Arc::new([]),
            series: Arc::new([]),
            vlines: Arc::new([]),
            hlines: Arc::new([]),
            data_min: None,
            data_max: None,
            x_lim: None,
            y_lim: None,
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

            // Add points and track bounds
            for (pos_index, &pos) in series.positions.iter().enumerate() {
                let p = DVec2::new(pos[0], pos[1]);
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
                        position: pos,
                        size,
                        size_mode,
                    });
                    point_colors.push(color);
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
                data_max.x += size;
                data_max.y += size;
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

        self.points = points.into();
        self.point_colors = point_colors.into();
        self.series = series_spans.into();
        self.vlines = vlines.into();
        self.hlines = hlines.into();
        self.data_min = data_min;
        self.data_max = data_max;
        self.legend_collapsed = widget.legend_collapsed;
        self.x_lim = widget.x_lim;
        self.y_lim = widget.y_lim;
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
    }

    pub(crate) fn autoscale(&mut self, update_axis_links: bool) {
        // Use user-specified limits if available, otherwise use data bounds
        let mut min_v = DVec2::new(-1.0, -1.0);
        let mut max_v = DVec2::new(1.0, 1.0);

        if let (Some(data_min), Some(data_max)) = (self.data_min, self.data_max) {
            min_v = data_min;
            max_v = data_max;
        }

        if let Some((y_min, y_max)) = self.y_lim {
            min_v.y = y_min;
            max_v.y = y_max;
        }

        if let Some((x_min, x_max)) = self.x_lim {
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
        let min_x = self.camera.position.x - self.camera.half_extents.x;
        let max_x = self.camera.position.x + self.camera.half_extents.x;

        let x_tick_values = match x_tick_producer {
            Some(producer) => producer(min_x, max_x),
            None => Vec::new(),
        };

        self.x_ticks.clear();
        for tick in x_tick_values {
            // Convert world position to screen position
            if let Some(screen_pos) =
                world_to_screen_position_x(tick.value, &self.camera, &self.bounds)
            {
                self.x_ticks.push(PositionedTick { screen_pos, tick });
            }
        }

        // Calculate y-axis ticks
        let min_y = self.camera.position.y - self.camera.half_extents.y;
        let max_y = self.camera.position.y + self.camera.half_extents.y;

        let y_tick_values = match y_tick_producer {
            Some(producer) => producer(min_y, max_y),
            None => Vec::new(),
        };

        self.y_ticks.clear();
        for tick in y_tick_values {
            // Convert world position to screen position
            if let Some(screen_pos) =
                world_to_screen_position_y(tick.value, &self.camera, &self.bounds)
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

    pub(crate) fn apply_input_event(
        &mut self,
        input: &PlotInputEvent,
        interactions_enabled: bool,
        scroll_to_pan_enabled: bool,
    ) -> InputEffects {
        const SELECTION_DELTA_THRESHOLD: f32 = 4.0; // pixels
        const SELECTION_PADDING: f32 = 0.02; // fractional padding in world units relative to selection size

        let mut effects = InputEffects::default();
        let viewport: DVec2 = Vec2::new(self.bounds.width, self.bounds.height).into();

        let mut update_cursor = |pointer: &PlotPointerEvent| {
            self.modifiers = pointer.modifiers;
            self.cursor_position = Vec2::new(pointer.local[0], pointer.local[1]);
            if self.crosshairs_enabled {
                self.crosshairs_position = self.cursor_position;
                effects.needs_redraw = true;
            }
        };

        match input {
            PlotInputEvent::CursorMoved(pointer) | PlotInputEvent::CursorEntered(pointer) => {
                update_cursor(pointer);
                effects.cursor_moved = true;
                effects.cursor_left = !pointer.inside;

                if interactions_enabled {
                    // Handle selection (right click drag)
                    if self.selection.active {
                        self.selection.end = self.cursor_position;
                        self.selection.moved = true;
                        effects.needs_redraw = true;
                    }

                    // Handle panning (left click drag)
                    if self.pan.active {
                        let render_current = self.camera.screen_to_render(
                            DVec2::new(
                                self.cursor_position.x as f64,
                                self.cursor_position.y as f64,
                            ),
                            viewport,
                        );
                        let render_start = self
                            .camera
                            .screen_to_render(self.pan.start_cursor, viewport);
                        let render_delta = render_current - render_start;

                        self.camera.position = self.pan.start_camera_center - render_delta;
                        self.update_axis_links();
                        effects.needs_redraw = true;
                    }

                    // Hover picking request flag when inside and idle
                    if !self.pan.active
                        && !self.selection.active
                        && self.hover_enabled
                        && pointer.inside
                    {
                        effects.needs_redraw = true;
                    }
                }
            }
            PlotInputEvent::CursorLeft(pointer) => {
                self.modifiers = pointer.modifiers;
                effects.cursor_left = true;
            }
            PlotInputEvent::ButtonPressed { button, pointer } => {
                self.modifiers = pointer.modifiers;
                if interactions_enabled {
                    match button {
                        mouse::Button::Left => {
                            if !pointer.inside {
                                return effects;
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
                                effects.needs_redraw = true;
                            } else {
                                if self.hover_enabled && !self.pan.active && !self.selection.active
                                {
                                    effects.request_pick_on_click = true;
                                }
                                self.pan.active = true;
                                self.pan.start_cursor =
                                    DVec2::new(pointer.local[0] as f64, pointer.local[1] as f64);
                                self.pan.start_camera_center = self.camera.position;
                            }
                        }
                        mouse::Button::Right => {
                            if !pointer.inside {
                                return effects;
                            }
                            self.selection.active = true;
                            self.selection.start = self.cursor_position;
                            self.selection.end = self.cursor_position;
                            self.selection.moved = false;
                            effects.needs_redraw = true;
                        }
                        _ => {}
                    }
                }
            }
            PlotInputEvent::ButtonReleased { button, pointer } => {
                self.modifiers = pointer.modifiers;
                if interactions_enabled {
                    match button {
                        mouse::Button::Left => {
                            if self.pan.active {
                                self.pan.active = false;
                            }
                        }
                        mouse::Button::Right => {
                            if self.selection.active {
                                self.selection.end = self.cursor_position;
                                let delta = self.selection.end - self.selection.start;
                                let dragged = delta.length() > SELECTION_DELTA_THRESHOLD;
                                if dragged {
                                    let p1 = self.camera.screen_to_world(
                                        DVec2::new(
                                            self.selection.start.x as f64,
                                            self.selection.start.y as f64,
                                        ),
                                        viewport,
                                    );
                                    let p2 = self.camera.screen_to_world(
                                        DVec2::new(
                                            self.selection.end.x as f64,
                                            self.selection.end.y as f64,
                                        ),
                                        viewport,
                                    );
                                    let min_v = DVec2::new(p1.x.min(p2.x), p1.y.min(p2.y));
                                    let max_v = DVec2::new(p1.x.max(p2.x), p1.y.max(p2.y));
                                    self.camera.set_bounds_preserve_offset(
                                        min_v,
                                        max_v,
                                        SELECTION_PADDING as f64,
                                    );
                                    self.update_axis_links();
                                }
                                self.selection.active = false;
                                self.selection.moved = false;
                                effects.needs_redraw = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            PlotInputEvent::WheelScrolled { delta, pointer } => {
                self.modifiers = pointer.modifiers;
                if interactions_enabled {
                    if !pointer.inside {
                        return effects;
                    }

                    let (x, y) = match delta {
                        iced::mouse::ScrollDelta::Lines { x, y } => (*x, *y),
                        iced::mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                    };

                    if self.modifiers.contains(keyboard::Modifiers::CTRL) {
                        let zoom_factor = if y > 0.0 { 0.95 } else { 1.05 };

                        let cursor_render_before = self.camera.screen_to_render(
                            DVec2::new(
                                self.cursor_position.x as f64,
                                self.cursor_position.y as f64,
                            ),
                            viewport,
                        );

                        self.camera.half_extents *= zoom_factor;

                        let cursor_render_after = self.camera.screen_to_render(
                            DVec2::new(
                                self.cursor_position.x as f64,
                                self.cursor_position.y as f64,
                            ),
                            viewport,
                        );

                        let render_delta = cursor_render_before - cursor_render_after;
                        self.camera.position += render_delta;

                        self.update_axis_links();
                        effects.needs_redraw = true;
                    } else if scroll_to_pan_enabled {
                        let world_pan_x =
                            -x as f64 * (self.camera.half_extents.x / (viewport.x / 2.0));
                        let world_pan_y =
                            y as f64 * (self.camera.half_extents.y / (viewport.y / 2.0));
                        self.camera.position.x += world_pan_x;
                        self.camera.position.y += world_pan_y;
                        self.update_axis_links();
                        effects.needs_redraw = true;
                    }
                }
            }
        }

        effects
    }

    pub(crate) fn handle_keyboard_event(&mut self, event: &keyboard::Event) -> bool {
        if let keyboard::Event::ModifiersChanged(modifiers) = event {
            self.modifiers = *modifiers;
        }
        false // No need to redraw
    }

    pub(crate) fn update_axis_links(&mut self) {
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
pub(crate) struct SeriesSpan {
    pub(crate) id: ShapeId,
    pub(crate) start: usize,
    pub(crate) len: usize,
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

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct InputEffects {
    pub(crate) needs_redraw: bool,
    pub(crate) cursor_moved: bool,
    pub(crate) cursor_left: bool,
    pub(crate) request_pick_on_click: bool,
}
