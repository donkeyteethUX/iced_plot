use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use glam::{DVec2, Vec2};
use iced::{
    Element, Length, Padding, Rectangle, alignment, color, keyboard,
    mouse::{self, Event, Interaction},
    wgpu::TextureFormat,
    widget::{
        self, container,
        shader::{self, Viewport},
        stack,
    },
};

use crate::{
    Color, HLine, LineStyle, PlotUiMessage, Series, TooltipContext, VLine, axes_labels,
    axis_link::AxisLink,
    camera::Camera,
    legend,
    message::{CursorPositionUiPayload, PlotRenderUpdate, TooltipUiPayload},
    picking,
    plot_renderer::{PlotRenderer, RenderParams},
    point::Point,
    series::SeriesError,
};

pub type TooltipProvider = Arc<dyn Fn(&TooltipContext) -> String + Send + Sync>;
pub type CursorProvider = Arc<dyn Fn(f64, f64) -> String + Send + Sync>;

pub struct PlotWidget {
    instance_id: u64,
    data: PlotData,
    autoscale_on_updates: bool,
    legend_collapsed: bool,
    x_axis_label: String,
    y_axis_label: String,
    // Axis limits
    x_lim: Option<(f64, f64)>,
    y_lim: Option<(f64, f64)>,
    // Axis links for synchronization
    x_axis_link: Option<AxisLink>,
    y_axis_link: Option<AxisLink>,
    // Tooltip config
    tooltips_enabled: bool,
    hover_radius_px: f32,
    tooltip_provider: Option<TooltipProvider>,
    tooltip_ui: Option<TooltipUiPayload>,
    cursor_overlay: bool,
    cursor_provider: Option<CursorProvider>,
    cursor_ui: Option<CursorPositionUiPayload>,
    crosshairs_enabled: bool,
}

impl Default for PlotWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl PlotWidget {
    pub fn new() -> Self {
        Self {
            instance_id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            tooltips_enabled: true,
            hover_radius_px: 8.0,
            tooltip_ui: None,
            cursor_overlay: true,
            cursor_provider: None,
            cursor_ui: None,
            autoscale_on_updates: false,
            x_axis_label: String::new(),
            y_axis_label: String::new(),
            x_lim: None,
            y_lim: None,
            x_axis_link: None,
            y_axis_link: None,
            legend_collapsed: false,
            data: PlotData::default(),
            tooltip_provider: None,
            crosshairs_enabled: true,
        }
    }

    pub fn add_series(&mut self, item: Series) -> Result<(), SeriesError> {
        self.data.add_series(item)
    }

    pub fn remove_series(&mut self, label: &str) -> bool {
        self.data.remove_series(label)
    }

    /// Add a vertical reference line to the plot.
    pub fn add_vline(&mut self, vline: VLine) -> Result<(), SeriesError> {
        self.data.add_vline(vline)
    }

    /// Add a horizontal reference line to the plot.
    pub fn add_hline(&mut self, hline: HLine) -> Result<(), SeriesError> {
        self.data.add_hline(hline)
    }

    pub fn set_x_axis_label(&mut self, label: impl Into<String>) {
        self.x_axis_label = label.into();
    }

    pub fn set_y_axis_label(&mut self, label: impl Into<String>) {
        self.y_axis_label = label.into();
    }

    /// Set the x-axis limits (min, max) for the plot.
    ///
    /// If set, these will override autoscaling for the x-axis.
    pub fn set_x_lim(&mut self, min: f64, max: f64) {
        self.x_lim = Some((min, max));
    }

    /// Set the y-axis limits (min, max) for the plot.
    ///
    /// If set, these will override autoscaling for the y-axis.
    pub fn set_y_lim(&mut self, min: f64, max: f64) {
        self.y_lim = Some((min, max));
    }

    /// Link the x-axis to other plots. When the x-axis is panned or zoomed,
    /// all plots sharing this link will update synchronously.
    pub fn set_x_axis_link(&mut self, link: AxisLink) {
        self.x_axis_link = Some(link);
    }

    /// Link the y-axis to other plots. When the y-axis is panned or zoomed,
    /// all plots sharing this link will update synchronously.
    pub fn set_y_axis_link(&mut self, link: AxisLink) {
        self.y_axis_link = Some(link);
    }

    /// Handle a message sent to the plot widget.
    pub fn update(&mut self, msg: PlotUiMessage) {
        match msg {
            PlotUiMessage::ToggleLegend => {
                self.legend_collapsed = !self.legend_collapsed;
            }
            PlotUiMessage::ToggleSeriesVisibility(label) => {
                self.data.toggle_series_visibility(&label);
            }
            PlotUiMessage::RenderUpdate(payload) => {
                if payload.clear_tooltip {
                    self.tooltip_ui = None;
                }
                if payload.clear_cursor_position {
                    self.cursor_ui = None;
                }
                if let Some(t) = payload.tooltip_ui {
                    self.tooltip_ui = Some(t);
                }
                if let Some(c) = payload.cursor_position_ui {
                    self.cursor_ui = Some(c);
                }
            }
        }
    }

    /// View the plot widget.
    pub fn view<'a>(&'a self) -> iced::Element<'a, PlotUiMessage> {
        let plot = widget::shader(self)
            .width(Length::Fill)
            .height(Length::Fill);

        let inner_container = widget::container(plot)
            .padding(2.0)
            .style(|_| container::background(color!(20, 20, 20)));

        let mut elements = stack![
            inner_container,
            legend::legend(&self.data, self.legend_collapsed),
        ];

        if let Some(tooltip_overlay) = self.view_tooltip_overlay() {
            elements = elements.push(tooltip_overlay);
        };

        if let Some(cursor_overlay) = self.view_cursor_overlay() {
            elements = elements.push(cursor_overlay);
        };

        widget::container(axes_labels::stack_with_labels(
            elements,
            &self.x_axis_label,
            &self.y_axis_label,
        ))
        .padding(3.0)
        .style(|_| container::background(color!(50, 50, 50)))
        .into()
    }

    /// Enable or disable hover tooltips (default: enabled)
    pub fn tooltips(&mut self, enabled: bool) {
        self.tooltips_enabled = enabled;
    }

    /// Enable or disable autoscaling on updates (default: enabled)
    pub fn autoscale_on_updates(&mut self, enabled: bool) {
        self.autoscale_on_updates = enabled;
    }

    /// Set hover radius in logical pixels for picking markers (default: 8 px)
    pub fn hover_radius_px(&mut self, radius: f32) {
        self.hover_radius_px = radius.max(0.0);
    }

    /// Set a custom tooltip text formatter.
    /// The formatter receives series label, point index, and data coordinates.
    pub fn set_tooltip_provider(&mut self, provider: TooltipProvider) {
        self.tooltip_provider = Some(provider);
    }

    /// Enable or disable the small cursor-position overlay shown in the
    /// lower-left corner of the plot. Disabled by default.
    pub fn set_cursor_overlay(&mut self, enabled: bool) {
        self.cursor_overlay = enabled;
    }

    /// Provide a custom formatter for the cursor overlay. Called with
    /// (x, y) world coordinates and should return the formatted string.
    pub fn set_cursor_provider(&mut self, provider: CursorProvider) {
        self.cursor_provider = Some(provider);
    }

    /// Enable or disable crosshairs that follow the cursor position.
    pub fn set_crosshairs(&mut self, enabled: bool) {
        self.crosshairs_enabled = enabled;
    }

    /// Set the positions of an existing series.
    pub fn set_series_positions(&mut self, label: &str, positions: &[[f64; 2]]) {
        self.data.set_series_positions(label, positions);
    }

    fn view_tooltip_overlay(&self) -> Option<Element<'_, PlotUiMessage>> {
        let Some(payload) = &self.tooltip_ui else {
            return None;
        };

        // Offset a bit from cursor
        let offset_x = payload.x + 8.0;
        let offset_y = payload.y + 8.0;

        let bubble = widget::container(widget::text(payload.text.clone()).size(14.0).style(
            // TODO: Use theme colors consistently everywhere rather than hardcoding.
            |_| widget::text::Style {
                color: Some(iced::Color::WHITE),
            },
        ))
        .padding(6.0)
        .style(|theme| {
            widget::container::rounded_box(theme)
                .background(color!(12, 12, 15, 0.9))
                .border(iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: color!(255, 255, 255, 0.12),
                })
        });

        let overlay = widget::container(bubble)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                left: offset_x,
                right: 0.0,
                top: offset_y,
                bottom: 0.0,
            })
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Top)
            .style(|_| widget::container::background(color!(0, 0, 0, 0.0)))
            .into();

        Some(overlay)
    }

    fn view_cursor_overlay(&self) -> Option<Element<'_, PlotUiMessage>> {
        if !self.cursor_overlay {
            return None;
        }

        let Some(payload) = &self.cursor_ui else {
            return None;
        };

        let bubble = widget::container(widget::text(payload.text.clone()).size(12.0))
            .padding(6.0)
            .style(|theme| {
                widget::container::dark(theme).border(iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: color!(255, 255, 255, 0.2),
                })
            });

        Some(
            widget::container(bubble)
                .width(Length::Shrink)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Bottom)
                .into(),
        )
    }
}

#[derive(Debug, Default)]
pub(crate) struct PlotData {
    pub(crate) series: Vec<Series>,
    pub(crate) vlines: Vec<VLine>,
    pub(crate) hlines: Vec<HLine>,
    pub(crate) version: u64,
    hidden_labels: HashSet<String>,
}

impl PlotData {
    /// Adds a series to the plot
    fn add_series(&mut self, item: Series) -> Result<(), SeriesError> {
        // Validate series invariants
        item.validate()?;

        // Enforce unique non-empty labels
        if let Some(label) = item.label.as_deref()
            && !label.is_empty()
            && self
                .series
                .iter()
                .any(|s| s.label.as_deref() == Some(label))
        {
            return Err(SeriesError::DuplicateLabel(label.to_string()));
        }

        self.series.push(item);
        self.version += 1;
        Ok(())
    }

    /// Removes a series by label. Returns true if found and removed.
    fn remove_series(&mut self, label: &str) -> bool {
        if let Some(idx) = self
            .series
            .iter()
            .position(|s| s.label.as_deref() == Some(label))
        {
            self.series.remove(idx);
            self.version += 1;
            self.hidden_labels.remove(label);
            return true;
        }
        false
    }

    fn set_series_positions(&mut self, label: &str, positions: &[[f64; 2]]) {
        if let Some(idx) = self
            .series
            .iter()
            .position(|s| s.label.as_deref() == Some(label))
        {
            self.series[idx].positions = positions.to_vec();
            self.version += 1;
        }
    }

    /// Add a vertical reference line to the plot.
    fn add_vline(&mut self, vline: VLine) -> Result<(), SeriesError> {
        // Enforce unique non-empty labels
        if let Some(label) = vline.label.as_deref()
            && !label.is_empty()
        {
            // Check for duplicate labels in vlines
            if self
                .vlines
                .iter()
                .any(|v| v.label.as_deref() == Some(label))
            {
                return Err(SeriesError::DuplicateLabel(label.to_string()));
            }
            // Check for duplicate labels in hlines
            if self
                .hlines
                .iter()
                .any(|h| h.label.as_deref() == Some(label))
            {
                return Err(SeriesError::DuplicateLabel(label.to_string()));
            }
            // Check for duplicate labels in series
            if self
                .series
                .iter()
                .any(|s| s.label.as_deref() == Some(label))
            {
                return Err(SeriesError::DuplicateLabel(label.to_string()));
            }
        }

        self.vlines.push(vline);
        self.version += 1;
        Ok(())
    }

    /// Add a horizontal reference line to the plot.
    fn add_hline(&mut self, hline: HLine) -> Result<(), SeriesError> {
        // Enforce unique non-empty labels
        if let Some(label) = hline.label.as_deref()
            && !label.is_empty()
        {
            // Check for duplicate labels in hlines
            if self
                .hlines
                .iter()
                .any(|h| h.label.as_deref() == Some(label))
            {
                return Err(SeriesError::DuplicateLabel(label.to_string()));
            }
            // Check for duplicate labels in vlines
            if self
                .vlines
                .iter()
                .any(|v| v.label.as_deref() == Some(label))
            {
                return Err(SeriesError::DuplicateLabel(label.to_string()));
            }
            // Check for duplicate labels in series
            if self
                .series
                .iter()
                .any(|s| s.label.as_deref() == Some(label))
            {
                return Err(SeriesError::DuplicateLabel(label.to_string()));
            }
        }

        self.hlines.push(hline);
        self.version += 1;
        Ok(())
    }

    pub(crate) fn legend_entries(&self) -> Vec<LegendEntry> {
        let mut out = Vec::new();
        for s in &self.series {
            if let Some(ref label) = s.label {
                if label.is_empty() {
                    continue;
                }
                if s.positions.is_empty() {
                    continue;
                }
                // Include series that have either markers or lines
                if s.marker_style.is_some() || s.line_style.is_some() {
                    let (color, marker) = if let Some(ref marker_style) = s.marker_style {
                        (marker_style.color, marker_style.marker_type as u32)
                    } else {
                        // For line-only series, use a default color (could come from line style in future)
                        // and a marker type that indicates no marker should be shown
                        (Color::from_rgb(0.5, 0.5, 0.5), u32::MAX)
                    };
                    out.push(LegendEntry {
                        label: label.clone(),
                        color,
                        marker,
                        line_style: s.line_style,
                        hidden: self.hidden_labels.contains(label),
                    });
                }
            }
        }
        // Add vertical reference lines to legend
        for vline in &self.vlines {
            if let Some(ref label) = vline.label
                && !label.is_empty()
            {
                out.push(LegendEntry {
                    label: label.clone(),
                    color: vline.color,
                    marker: u32::MAX,
                    line_style: Some(vline.line_style),
                    hidden: self.hidden_labels.contains(label),
                });
            }
        }
        // Add horizontal reference lines to legend
        for hline in &self.hlines {
            if let Some(ref label) = hline.label
                && !label.is_empty()
            {
                out.push(LegendEntry {
                    label: label.clone(),
                    color: hline.color,
                    marker: u32::MAX,
                    line_style: Some(hline.line_style),
                    hidden: self.hidden_labels.contains(label),
                });
            }
        }
        out
    }

    fn toggle_series_visibility(&mut self, label: &str) {
        // Check if it's a series, vline, or hline
        let exists = self
            .series
            .iter()
            .any(|s| s.label.as_deref() == Some(label))
            || self
                .vlines
                .iter()
                .any(|v| v.label.as_deref() == Some(label))
            || self
                .hlines
                .iter()
                .any(|h| h.label.as_deref() == Some(label));

        if !exists {
            println!("Toggle series visibility: series not found: {label}");
            return;
        }
        if self.hidden_labels.contains(label) {
            self.hidden_labels.remove(label);
        } else {
            self.hidden_labels.insert(label.to_string());
        }
        self.version += 1;
    }
}

#[derive(Debug, Clone)]
pub struct PlotState {
    pub(crate) src_version: u64, // version of PlotData last synced
    // Shared data: cheap O(1) clone when producing a Primitive
    pub(crate) points: Arc<[Point]>,      // vertex/instance data
    pub(crate) series: Arc<[SeriesSpan]>, // spans describing logical series
    pub(crate) vlines: Arc<[VLine]>,      // vertical reference lines
    pub(crate) hlines: Arc<[HLine]>,      // horizontal reference lines
    pub(crate) labels: HashSet<String>,
    pub(crate) data_min: Option<DVec2>,
    pub(crate) data_max: Option<DVec2>,
    // Axis limits
    pub(crate) x_lim: Option<(f64, f64)>,
    pub(crate) y_lim: Option<(f64, f64)>,
    // Axis links for synchronization
    x_axis_link: Option<AxisLink>,
    y_axis_link: Option<AxisLink>,
    x_link_version: u64,
    y_link_version: u64,
    // UI / camera
    pub(crate) camera: Camera,
    pub(crate) bounds: Rectangle,
    // Interaction state
    cursor_position: Vec2,
    last_click_time: Option<Instant>,
    legend_collapsed: bool,
    modifiers: keyboard::Modifiers,
    pub(crate) selection: SelectionState,
    pub(crate) pan: PanState,
    // Version counters
    pub(crate) markers_version: u64,
    pub(crate) lines_version: u64,
    // Hover/picking internals
    hover_enabled: bool,
    hover_radius_px: f32,
    last_hover_cache: Option<HoverHit>,
    // For renderer: hovered marker world coords and pixel size
    pub(crate) hovered_world: Option<[f64; 2]>,
    pub(crate) hovered_size_px: f32,
    pub(crate) hover_version: u64,
    // Crosshairs
    pub(crate) crosshairs_enabled: bool,
    pub(crate) crosshairs_position: Vec2,
}

impl Default for PlotState {
    fn default() -> Self {
        Self::new()
    }
}

impl PlotState {
    /// Creates a new empty plot widget with default viewport size
    pub fn new() -> Self {
        Self {
            src_version: 0,
            points: Arc::new([]),
            series: Arc::new([]),
            vlines: Arc::new([]),
            hlines: Arc::new([]),
            labels: HashSet::new(),
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
            hover_enabled: true,
            hover_radius_px: 8.0,
            last_hover_cache: None,
            hovered_world: None,
            hovered_size_px: 0.0,
            hover_version: 0,
            crosshairs_enabled: false,
            crosshairs_position: Vec2::ZERO,
        }
    }

    /// Adds a series to the plot
    pub fn add_series(&mut self, item: Series) {
        assert!(!item.positions.is_empty());
        assert!(item.line_style.is_some() || item.marker_style.is_some());
        assert!(item.label.as_ref().is_none_or(|l| !self.labels.contains(l)));

        // Clone-on-write
        let mut points = self.points.to_vec();
        let mut series = self.series.to_vec();
        let start = points.len();
        let mut local_min = self
            .data_min
            .unwrap_or(DVec2::new(item.positions[0][0], item.positions[0][1]));
        let mut local_max = self.data_max.unwrap_or(local_min);
        for p in &item.positions {
            local_min = local_min.min(DVec2::new(p[0], p[1]));
            local_max = local_max.max(DVec2::new(p[0], p[1]));
        }
        self.data_min = Some(match self.data_min {
            Some(m) => m.min(local_min),
            None => local_min,
        });
        self.data_max = Some(match self.data_max {
            Some(m) => m.max(local_max),
            None => local_max,
        });
        let label_string = item.label.unwrap_or_default();
        let line_style = item.line_style;

        // Determine color and marker info
        let (color, marker) = if let Some(ref marker_style) = item.marker_style {
            (marker_style.color, marker_style.marker_type as u32)
        } else {
            (Color::WHITE, u32::MAX) // u32::MAX indicates no marker.
        };

        // Only create points if we have markers OR lines (lines need points for geometry)
        if item.marker_style.is_some() || line_style.is_some() {
            let size = item.marker_style.as_ref().map(|ms| ms.size).unwrap_or(1.0);
            for pos in item.positions {
                points.push(Point {
                    position: pos,
                    size,
                });
            }
        }

        let len = points.len() - start;
        series.push(SeriesSpan {
            label: label_string.clone(),
            start,
            len,
            line_style,
            color,
            marker,
        });
        if !label_string.is_empty() {
            self.labels.insert(label_string);
        }
        self.points = points.into();
        self.series = series.into();
        self.markers_version += 1;
        if line_style.is_some() {
            self.lines_version += 1;
        }
    }

    pub fn autoscale(&mut self) {
        if let (Some(data_min), Some(data_max)) = (self.data_min, self.data_max) {
            // Use user-specified limits if available, otherwise use data bounds
            let mut min_v = data_min;
            let mut max_v = data_max;

            if let Some((x_min, x_max)) = self.x_lim {
                min_v.x = x_min;
                max_v.x = x_max;
            }
            if let Some((y_min, y_max)) = self.y_lim {
                min_v.y = y_min;
                max_v.y = y_max;
            }

            self.camera.set_bounds(min_v, max_v, 0.05);
            self.update_axis_links();
        }
    }

    pub fn handle_mouse_event(&mut self, event: Event) -> bool {
        self.process_input(event)
    }

    /// Toggle the collapsed state of the legend
    pub fn toggle_legend(&mut self) {
        self.legend_collapsed = !self.legend_collapsed;
    }

    pub fn set_series_line_style(&mut self, label: &str, line_style: Option<LineStyle>) -> bool {
        if self
            .series
            .iter()
            .any(|s| s.label == label && s.line_style != line_style)
        {
            let mut series = self.series.to_vec();
            for s in &mut series {
                if s.label == label {
                    if s.line_style != line_style {
                        s.line_style = line_style;
                        self.lines_version += 1;
                    }
                    break;
                }
            }
            self.series = series.into();
            return true;
        }
        false
    }

    /// Update axis links with current camera state
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

    pub fn process_input(&mut self, ev: Event) -> bool {
        const SELECTION_DELTA_THRESHOLD: f32 = 4.0; // pixels
        const SELECTION_PADDING: f32 = 0.02; // fractional padding in world units relative to selection size
        let mut needs_redraw = false;

        // Only request redraws when something actually changes or when we need
        // to service a picking request for a new cursor position.

        let viewport = Vec2::new(self.bounds.width, self.bounds.height);

        match ev {
            Event::CursorMoved { position } => {
                // Check if the cursor is inside this widget's bounds in window space
                let inside = position.x >= self.bounds.x
                    && position.x <= (self.bounds.x + self.bounds.width)
                    && position.y >= self.bounds.y
                    && position.y <= (self.bounds.y + self.bounds.height);

                // Store cursor in local coordinates (relative to bounds)
                self.cursor_position =
                    Vec2::new(position.x - self.bounds.x, position.y - self.bounds.y);
                // Update crosshairs position when enabled
                self.crosshairs_position = self.cursor_position;

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
                        DVec2::new(viewport.x as f64, viewport.y as f64),
                    );
                    let render_start = self.camera.screen_to_render(
                        DVec2::new(
                            self.pan.start_cursor.x as f64,
                            self.pan.start_cursor.y as f64,
                        ),
                        DVec2::new(viewport.x as f64, viewport.y as f64),
                    );
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
                        if self.last_hover_cache.is_some() || self.hovered_world.is_some() {
                            self.last_hover_cache = None;
                            self.hovered_world = None;
                            self.hover_version = self.hover_version.wrapping_add(1);
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
                if self.last_hover_cache.is_some() || self.hovered_world.is_some() {
                    self.last_hover_cache = None;
                    self.hovered_world = None;
                    self.hover_version = self.hover_version.wrapping_add(1);
                    needs_redraw = true;
                }
            }
            Event::ButtonPressed(mouse::Button::Left) => {
                // Only start panning if the press started inside our bounds
                // (Drags will continue even if the cursor leaves later)
                let inside = self.cursor_position.x >= 0.0
                    && self.cursor_position.y >= 0.0
                    && self.cursor_position.x <= self.bounds.width
                    && self.cursor_position.y <= self.bounds.height;
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
                    self.autoscale();
                    needs_redraw = true;
                } else {
                    // Start panning
                    self.pan.active = true;
                    self.pan.start_cursor = self.cursor_position;
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
                let inside = self.cursor_position.x >= 0.0
                    && self.cursor_position.y >= 0.0
                    && self.cursor_position.x <= self.bounds.width
                    && self.cursor_position.y <= self.bounds.height;
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
                        let screen_size = DVec2::new(viewport.x as f64, viewport.y as f64);
                        let p1 = self.camera.screen_to_world(
                            DVec2::new(
                                self.selection.start.x as f64,
                                self.selection.start.y as f64,
                            ),
                            screen_size,
                        );
                        let p2 = self.camera.screen_to_world(
                            DVec2::new(self.selection.end.x as f64, self.selection.end.y as f64),
                            screen_size,
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
                let inside = self.cursor_position.x >= 0.0
                    && self.cursor_position.y >= 0.0
                    && self.cursor_position.x <= self.bounds.width
                    && self.cursor_position.y <= self.bounds.height;
                if !inside {
                    return needs_redraw;
                }
                // Only zoom when Ctrl is held down
                if self.modifiers.contains(keyboard::Modifiers::CTRL) {
                    if let iced::mouse::ScrollDelta::Pixels { y, .. } = delta {
                        // Apply zoom factor based on scroll direction
                        let zoom_factor = if y > 0.0 { 0.95 } else { 1.05 };

                        // Convert cursor position to render coordinates before zoom (without offset)
                        let cursor_render_before = self.camera.screen_to_render(
                            DVec2::new(
                                self.cursor_position.x as f64,
                                self.cursor_position.y as f64,
                            ),
                            DVec2::new(viewport.x as f64, viewport.y as f64),
                        );

                        // Apply zoom by scaling half_extents
                        self.camera.half_extents *= zoom_factor;

                        // Convert cursor position to render coordinates after zoom
                        let cursor_render_after = self.camera.screen_to_render(
                            DVec2::new(
                                self.cursor_position.x as f64,
                                self.cursor_position.y as f64,
                            ),
                            DVec2::new(viewport.x as f64, viewport.y as f64),
                        );

                        // Adjust camera position (in render space) to keep cursor at same position
                        let render_delta = cursor_render_before - cursor_render_after;
                        // Convert render delta back to world space and adjust camera position
                        self.camera.position += render_delta;

                        self.update_axis_links();
                        needs_redraw = true;
                    }
                } else if let iced::mouse::ScrollDelta::Pixels { y, x } = delta {
                    let scroll_ratio = y / x;

                    if scroll_ratio.abs() > 2.0 {
                        // Mostly vertical scroll
                        let y_pan_amount = 20.0 * if y > 0.0 { -1.0 } else { 1.0 };
                        // Convert pan amount from screen space to world space
                        let world_pan =
                            y_pan_amount * (self.camera.half_extents.y / (viewport.y as f64 / 2.0));
                        self.camera.position.y += world_pan;
                        self.update_axis_links();
                        needs_redraw = true;
                    } else if scroll_ratio.abs() < 0.5 {
                        // Mostly horizontal scroll
                        let x_pan_amount = 20.0 * if x > 0.0 { -1.0 } else { 1.0 };
                        // Convert pan amount from screen space to world space
                        let world_pan_x =
                            x_pan_amount * (self.camera.half_extents.x / (viewport.x as f64 / 2.0));
                        self.camera.position.x -= world_pan_x;
                        self.update_axis_links();
                        needs_redraw = true;
                    }
                }
            }
            _ => {}
        }

        // camera uniform is handled in renderer per frame
        needs_redraw
    }

    pub fn handle_keyboard_event(&mut self, event: keyboard::Event) -> bool {
        if let keyboard::Event::ModifiersChanged(modifiers) = event {
            self.modifiers = modifiers;
        }
        false // No need to redraw
    }
}

impl shader::Program<PlotUiMessage> for PlotWidget {
    type State = PlotState;
    type Primitive = Primitive;

    fn draw(
        &self,
        state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        Primitive::new(self.instance_id, state.clone())
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<shader::Action<PlotUiMessage>> {
        let mut needs_redraw = false;

        if self.data.version != state.src_version {
            // todo: something less dumb
            state.series = Arc::new([]);
            state.points = Arc::new([]);
            state.labels.clear();
            state.data_min = None;
            state.data_max = None;
            for s in &self.data.series {
                if s.label
                    .as_ref()
                    .is_some_and(|l| !self.data.hidden_labels.contains(l.as_str()))
                {
                    state.add_series(s.clone());
                }
            }

            // Sync reference lines (filter by hidden labels)
            let visible_vlines: Vec<VLine> = self
                .data
                .vlines
                .iter()
                .filter(|v| {
                    v.label
                        .as_ref()
                        .map(|l| !self.data.hidden_labels.contains(l.as_str()))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            state.vlines = Arc::from(visible_vlines);

            let visible_hlines: Vec<HLine> = self
                .data
                .hlines
                .iter()
                .filter(|h| {
                    h.label
                        .as_ref()
                        .map(|l| !self.data.hidden_labels.contains(l.as_str()))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            state.hlines = Arc::from(visible_hlines);

            // Force GPU buffers to rebuild for both markers and lines even when zero series remain
            state.markers_version = state.markers_version.wrapping_add(1);
            state.lines_version = state.lines_version.wrapping_add(1);

            // Autoscale on first update or always if autoscale_on_updates is enabled.
            if state.src_version == 0 || self.autoscale_on_updates {
                state.autoscale();
            }

            state.src_version = self.data.version;
            state.legend_collapsed = self.legend_collapsed;
            state.x_lim = self.x_lim;
            state.y_lim = self.y_lim;
            state.x_axis_link = self.x_axis_link.clone();
            state.y_axis_link = self.y_axis_link.clone();
            needs_redraw = true;
        }

        // Check if axis links have been updated by other plots
        if let Some(ref link) = state.x_axis_link {
            let link_version = link.version();
            if link_version != state.x_link_version {
                let (position, half_extent, version) = link.get();
                state.camera.position.x = position;
                state.camera.half_extents.x = half_extent;
                state.x_link_version = version;
                needs_redraw = true;
            }
        }
        if let Some(ref link) = state.y_axis_link {
            let link_version = link.version();
            if link_version != state.y_link_version {
                let (position, half_extent, version) = link.get();
                state.camera.position.y = position;
                state.camera.half_extents.y = half_extent;
                state.y_link_version = version;
                needs_redraw = true;
            }
        }

        state.bounds = bounds;
        // Sync hover configuration from widget to internal state
        state.hover_enabled = self.tooltips_enabled;
        state.hover_radius_px = self.hover_radius_px;
        // Sync crosshairs configuration
        state.crosshairs_enabled = self.crosshairs_enabled;
        let mut clear_tooltip = false;
        let mut clear_cursor_position = false;
        let mut publish_tooltip: Option<TooltipUiPayload> = None;
        let mut publish_cursor: Option<CursorPositionUiPayload> = None;
        // viewport size (screen pixels for this widget)
        let viewport = Vec2::new(state.bounds.width, state.bounds.height);

        match event {
            iced::Event::Mouse(mouse_event) => {
                let before = state.last_hover_cache.clone().map(|h| h.key());
                needs_redraw |= state.handle_mouse_event(*mouse_event);
                // If cursor moved and hover enabled, submit a GPU pick request
                if let iced::mouse::Event::CursorMoved { .. } = mouse_event
                    && state.hover_enabled
                    && !state.pan.active
                    && !state.selection.active
                {
                    // Only submit pick request if cursor is within widget bounds
                    let inside = state.cursor_position.x >= 0.0
                        && state.cursor_position.y >= 0.0
                        && state.cursor_position.x <= state.bounds.width
                        && state.cursor_position.y <= state.bounds.height;
                    if inside {
                        picking::submit_request(
                            self.instance_id,
                            crate::picking::PickRequest {
                                cursor_x: state.cursor_position.x,
                                cursor_y: state.cursor_position.y,
                                radius_px: state.hover_radius_px,
                                seq: state.hover_version.wrapping_add(1),
                            },
                        );
                    }
                    // Publish cursor overlay updates when enabled
                    if self.cursor_overlay {
                        if inside {
                            let world = state.camera.screen_to_world(
                                DVec2::new(
                                    state.cursor_position.x as f64,
                                    state.cursor_position.y as f64,
                                ),
                                DVec2::new(viewport.x as f64, viewport.y as f64),
                            );
                            let text = if let Some(p) = &self.cursor_provider {
                                (p)(world.x, world.y)
                            } else {
                                format!("{:.4}, {:.4}", world.x, world.y)
                            };

                            publish_cursor = Some(CursorPositionUiPayload {
                                x: world.x,
                                y: world.y,
                                text,
                            });
                        } else {
                            clear_cursor_position = true;
                        }
                    }
                }
                // If hover was cleared due to cursor leave or disabled bounds, clear tooltip immediately
                if before.is_some() && state.last_hover_cache.is_none() {
                    clear_tooltip = true;
                }
                // If hovered point changed, compute formatted text and fire callback
                if state.hover_enabled {
                    // Try to consume a GPU pick result for this instance
                    if let Some(res) = picking::take_result(self.instance_id) {
                        match res.hit {
                            Some(hit) => {
                                // Update hover cache and overlay
                                let world_v = DVec2::new(hit.world[0], hit.world[1]);
                                state.hovered_world = Some(hit.world);
                                state.hovered_size_px = hit.size_px;
                                let ctx = TooltipContext {
                                    series_label: hit.series_label.clone(),
                                    point_index: hit.point_index,
                                    x: hit.world[0],
                                    y: hit.world[1],
                                };
                                let text = if let Some(p) = &self.tooltip_provider {
                                    (p)(&ctx)
                                } else {
                                    // Default: simple coordinates with compact formatting
                                    format!("{:.4}, {:.4}", ctx.x, ctx.y)
                                };
                                let hover_hit = HoverHit {
                                    series_label: hit.series_label,
                                    point_index: hit.point_index,
                                    _world: world_v,
                                    _size_px: hit.size_px,
                                };
                                state.last_hover_cache = Some(hover_hit);
                                state.hover_version = state.hover_version.wrapping_add(1);
                                publish_tooltip = Some(TooltipUiPayload {
                                    x: state.cursor_position.x,
                                    y: state.cursor_position.y,
                                    text,
                                });
                                needs_redraw = true;
                            }
                            None => {
                                if before.is_some() {
                                    state.hovered_world = None;
                                    state.last_hover_cache = None;
                                    state.hover_version = state.hover_version.wrapping_add(1);
                                    clear_tooltip = true;
                                    needs_redraw = true;
                                }
                            }
                        }
                    }
                } else if before.is_some() {
                    clear_tooltip = true;
                }
            }
            // CursorLeft is handled inside the Mouse(...) branch above via state.handle_mouse_event
            iced::Event::Keyboard(keyboard_event) => {
                needs_redraw |= state.handle_keyboard_event(keyboard_event.clone());
            }
            _ => {}
        }

        let needs_publish = publish_tooltip.is_some()
            || publish_cursor.is_some()
            || clear_tooltip
            || clear_cursor_position;

        if needs_publish {
            return Some(shader::Action::publish(PlotUiMessage::RenderUpdate(
                PlotRenderUpdate {
                    clear_tooltip,
                    clear_cursor_position,
                    tooltip_ui: publish_tooltip,
                    cursor_position_ui: publish_cursor,
                },
            )));
        }

        needs_redraw.then(shader::Action::request_redraw)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Interaction {
        // Return appropriate mouse cursor based on current interaction state
        if state.pan.active {
            Interaction::Grabbing
        } else if state.selection.active {
            Interaction::Crosshair
        } else if state.last_hover_cache.is_some() {
            Interaction::Pointer
        } else {
            Interaction::None
        }
    }
}

pub struct PlotRendererState {
    renderers: HashMap<u64, PlotRenderer>,
    format: TextureFormat,
}

#[derive(Debug)]
pub struct Primitive {
    instance_id: u64,
    plot_widget: PlotState,
}

impl Primitive {
    pub fn new(instance_id: u64, plot_widget: PlotState) -> Self {
        Self {
            instance_id,
            plot_widget,
        }
    }
}

impl shader::Primitive for Primitive {
    type Renderer = PlotRendererState;

    fn initialize(
        &self,
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self::Renderer {
        PlotRendererState {
            renderers: HashMap::new(),
            format,
        }
    }

    fn prepare(
        &self,
        renderer_state: &mut Self::Renderer,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        // Get or create renderer for this widget instance
        let renderer = renderer_state
            .renderers
            .entry(self.instance_id)
            .or_insert_with(|| PlotRenderer::new(device, queue, renderer_state.format));
        // Unified, single-call preparation of the renderer per frame
        renderer.prepare_frame(device, queue, viewport, bounds, &self.plot_widget);
        renderer.service_picking(self.instance_id, device, queue, &self.plot_widget);
    }

    fn render(
        &self,
        renderer_state: &Self::Renderer,
        encoder: &mut iced::wgpu::CommandEncoder,
        target: &iced::wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if let Some(renderer) = renderer_state.renderers.get(&self.instance_id) {
            renderer.encode(RenderParams {
                encoder,
                target,
                bounds: *clip_bounds,
            });
        }
    }
}

#[derive(Debug, Clone)]
pub struct LegendEntry {
    pub label: String,
    pub color: Color,
    pub marker: u32,
    pub line_style: Option<LineStyle>,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct SeriesSpan {
    pub label: String,
    pub start: usize,
    pub len: usize,
    pub line_style: Option<LineStyle>,
    pub color: Color,
    pub marker: u32,
}

#[derive(Default, Debug, Clone)]
pub struct SelectionState {
    pub active: bool,
    pub start: Vec2,
    pub end: Vec2,
    pub moved: bool,
}

#[derive(Default, Debug, Clone)]
pub struct PanState {
    pub active: bool,
    pub start_cursor: Vec2,
    pub start_camera_center: DVec2,
}

#[derive(Debug, Clone)]
struct HoverHit {
    series_label: String,
    point_index: usize,
    _world: DVec2,
    _size_px: f32,
}

impl HoverHit {
    fn key(&self) -> (String, usize) {
        (self.series_label.clone(), self.point_index)
    }
}

// Global unique ID generator for widget instances
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
