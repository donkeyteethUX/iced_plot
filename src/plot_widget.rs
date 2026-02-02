use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use glam::{DVec2, Vec2};
use iced::{
    Color, Element, Length, Rectangle, Theme,
    alignment::{self, Horizontal, Vertical},
    keyboard,
    mouse::{self, Interaction},
    padding::{self, Padding},
    wgpu::TextureFormat,
    widget::{
        self, container,
        shader::{self, Pipeline, Viewport},
        stack,
    },
};
use indexmap::IndexMap;

use crate::{
    HLine, HoverPickEvent, MarkerSize, MarkerStyle, PlotUiMessage, Point, PointId, Series,
    TooltipContext, VLine, axes_labels,
    axis_link::AxisLink,
    camera::Camera,
    legend::{self, LegendEntry},
    message::{CursorPositionUiPayload, PlotRenderUpdate, TooltipUiPayload},
    picking,
    plot_renderer::{PlotRenderer, RenderParams},
    plot_state::PlotState,
    series::{SeriesError, ShapeId},
    ticks::{self, PositionedTick, TickFormatter, TickProducer},
};

pub type CursorProvider = Arc<dyn Fn(f64, f64) -> String + Send + Sync>;
/// Provider for highlighting a point.
///
/// Modifies the highlight point in mutable reference.
///
/// Returns the tooltip text to display for the point, if any.
pub type HighlightPointProvider =
    Arc<dyn Fn(TooltipContext<'_>, &mut HighlightPoint) -> Option<String> + Send + Sync>;

/// A plot widget that renders data series with interactive features.
pub struct PlotWidget {
    pub(crate) instance_id: u64,
    // Data
    pub(crate) series: IndexMap<ShapeId, Series>,
    pub(crate) vlines: IndexMap<ShapeId, VLine>,
    pub(crate) hlines: IndexMap<ShapeId, HLine>,
    pub(crate) hidden_shapes: HashSet<ShapeId>,
    pub(crate) data_version: u64,
    pub(crate) highlight_version: u64,
    // Configuration
    pub(crate) autoscale_on_updates: bool,
    pub(crate) legend_enabled: bool,
    pub(crate) legend_collapsed: bool,
    pub(crate) x_axis_label: String,
    pub(crate) y_axis_label: String,
    pub(crate) x_lim: Option<(f64, f64)>,
    pub(crate) y_lim: Option<(f64, f64)>,
    pub(crate) x_axis_link: Option<AxisLink>,
    pub(crate) y_axis_link: Option<AxisLink>,
    pub(crate) hover_radius_px: f32,
    pub(crate) pick_highlight_provider: Option<HighlightPointProvider>,
    pub(crate) hover_highlight_provider: Option<HighlightPointProvider>,
    pub(crate) cursor_overlay: bool,
    pub(crate) cursor_provider: Option<CursorProvider>,
    pub(crate) crosshairs_enabled: bool,
    pub(crate) controls_help_enabled: bool,
    pub(crate) controls_overlay_open: bool,
    pub(crate) x_axis_formatter: Option<TickFormatter>,
    pub(crate) y_axis_formatter: Option<TickFormatter>,
    pub(crate) x_tick_producer: Option<TickProducer>,
    pub(crate) y_tick_producer: Option<TickProducer>,
    pub(crate) tick_label_size: f32,
    pub(crate) axis_label_size: f32,
    pub(crate) data_aspect: Option<f64>,
    // UI state
    /// Map of picked point id to highlight point data & tooltip text.
    pub(crate) picked_points: IndexMap<PointId, (HighlightPoint, Option<TooltipUiPayload>)>,
    /// Map of hovered point id to highlight point data & tooltip text.
    pub(crate) hovered_points: IndexMap<PointId, (HighlightPoint, Option<TooltipUiPayload>)>,
    pub(crate) cursor_ui: Option<CursorPositionUiPayload>,
    pub(crate) x_ticks: Vec<PositionedTick>,
    pub(crate) y_ticks: Vec<PositionedTick>,
    // Camera and bounds for coordinate conversion (updated when ticks are updated)
    pub(crate) camera_bounds: Option<(Camera, Rectangle)>,
}

impl Default for PlotWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl PlotWidget {
    /// Create a new plot widget with default settings.
    pub fn new() -> Self {
        Self {
            instance_id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            series: IndexMap::new(),
            vlines: IndexMap::new(),
            hlines: IndexMap::new(),
            hidden_shapes: HashSet::new(),
            data_version: 0,
            highlight_version: 0,
            autoscale_on_updates: false,
            legend_enabled: true,
            legend_collapsed: false,
            x_axis_label: String::new(),
            y_axis_label: String::new(),
            x_lim: None,
            y_lim: None,
            x_axis_link: None,
            y_axis_link: None,
            hover_radius_px: 8.0,
            pick_highlight_provider: None,
            hover_highlight_provider: None,
            cursor_overlay: true,
            cursor_provider: None,
            crosshairs_enabled: false,
            controls_help_enabled: true,
            controls_overlay_open: false,
            x_axis_formatter: Some(Arc::new(ticks::default_formatter)),
            y_axis_formatter: Some(Arc::new(ticks::default_formatter)),
            x_tick_producer: Some(Arc::new(ticks::default_tick_producer)),
            y_tick_producer: Some(Arc::new(ticks::default_tick_producer)),
            tick_label_size: 10.0,
            axis_label_size: 16.0,
            data_aspect: None,
            x_ticks: Vec::new(),
            y_ticks: Vec::new(),
            picked_points: IndexMap::new(),
            hovered_points: IndexMap::new(),
            cursor_ui: None,
            camera_bounds: None,
        }
    }

    /// Add a data series to the plot.
    /// If there exists a series with the same `item.id` ([ShapeId]), the old one will be replaced.
    pub fn add_series(&mut self, item: Series) -> Result<(), SeriesError> {
        item.validate()?;
        _ = self.series.insert(item.id, item);
        self.data_version += 1;
        Ok(())
    }

    /// Set the data aspect ratio (y units per x unit). Use 1.0 for square pixels.
    pub fn set_data_aspect(&mut self, aspect: f64) {
        if aspect.is_finite() && aspect > 0.0 {
            self.data_aspect = Some(aspect);
        } else {
            self.data_aspect = None;
        }
        self.data_version = self.data_version.wrapping_add(1);
    }

    /// Remove a data series from the plot by its ID.
    pub fn remove_series(&mut self, id: &ShapeId) -> Result<(), SeriesError> {
        if self.series.shift_remove(id).is_some() {
            self.hidden_shapes.remove(id);
            self.data_version += 1;
            Ok(())
        } else {
            Err(SeriesError::NotFound(*id))
        }
    }

    /// Update a data series by its id.
    pub fn update_series<F: FnMut(&mut Series)>(
        &mut self,
        id: &ShapeId,
        mut f: F,
    ) -> Result<(), SeriesError> {
        if let Some(series) = self.series.get_mut(id) {
            f(series);
            self.data_version += 1;
            Ok(())
        } else {
            Err(SeriesError::NotFound(*id))
        }
    }

    /// Add a vertical reference line to the plot.
    /// If there exists a line with the same `vline.id` ([ShapeId]), the old one will be replaced.
    pub fn add_vline(&mut self, vline: VLine) {
        self.vlines.insert(vline.id, vline);
        self.data_version += 1;
    }

    /// Add a horizontal reference line to the plot.
    /// If there exists a line with the same `hline.id` ([ShapeId]), the old one will be replaced.
    pub fn add_hline(&mut self, hline: HLine) {
        self.hlines.insert(hline.id, hline);
        self.data_version += 1;
    }

    /// Set the x-axis label.
    pub fn set_x_axis_label(&mut self, label: impl Into<String>) {
        self.x_axis_label = label.into();
    }

    /// Set the y-axis label.
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

    /// Convert world position to screen position using camera and bounds
    /// Similar to how PositionedTick calculates screen position
    fn world_to_screen_position(
        world: [f64; 2],
        camera_bounds: &(Camera, Rectangle),
    ) -> Option<[f32; 2]> {
        let (camera, bounds) = camera_bounds;
        if let (Some(screen_x), Some(screen_y)) = (
            world_to_screen_position_x(world[0], camera, bounds),
            world_to_screen_position_y(world[1], camera, bounds),
        ) {
            Some([screen_x, screen_y])
        } else {
            None
        }
    }

    /// Compute the world-space anchor point for a tooltip.
    ///
    /// For pixel-sized markers (or no markers), this is the point's (x, y).
    /// For world-sized square markers, the marker is effectively bottom-left anchored,
    /// so we anchor the tooltip at the marker's center (x + size/2, y + size/2).
    fn tooltip_anchor_world(point: &HighlightPoint) -> [f64; 2] {
        if let Some(marker_style) = point.marker_style
            && let MarkerSize::World(size) = marker_style.size
        {
            let half = size * 0.5;
            [point.x + half, point.y + half]
        } else {
            [point.x, point.y]
        }
    }

    /// Update tooltip positions for all hovered and picked points
    /// This should be called when the plot canvas position changes
    fn update_tooltip_positions(&mut self) {
        if let Some(camera_bounds) = &self.camera_bounds {
            for (highlight_point, tooltip) in self
                .hovered_points
                .values_mut()
                .chain(self.picked_points.values_mut())
            {
                if let Some(tooltip) = tooltip {
                    tooltip.screen_xy = Self::world_to_screen_position(
                        Self::tooltip_anchor_world(highlight_point),
                        camera_bounds,
                    );
                }
            }
        }
    }

    /// Get an iterator over the series ids in the plot.
    pub fn series_ids(&self) -> Vec<ShapeId> {
        self.series.keys().copied().collect()
    }
    /// Get the position of a point in the plot.
    pub fn point_position(&self, point_id: PointId) -> Option<[f64; 2]> {
        self.series
            .get(&point_id.series_id)?
            .positions
            .get(point_id.point_index)
            .copied()
    }

    /// Find the nearest point to a given position in the plot.
    pub fn nearest_point(&self, series_id: ShapeId, x: f64, y: f64) -> Option<PointId> {
        let series = self.series.get(&series_id)?;
        let mut min_distance = f64::INFINITY;
        let mut nearest_point = None;
        for (i, position) in series.positions.iter().enumerate() {
            let distance = (position[0] - x).powi(2) + (position[1] - y).powi(2);
            if distance < min_distance {
                min_distance = distance;
                nearest_point = Some(PointId {
                    series_id,
                    point_index: i,
                });
            }
        }
        nearest_point
    }

    /// Find the nearest point to a given x-coordinate in the plot.
    pub fn nearest_point_horizontal(&self, series_id: ShapeId, x: f64) -> Option<PointId> {
        let series = self.series.get(&series_id)?;
        let mut min_distance = f64::INFINITY;
        let mut nearest_point = None;
        for (i, position) in series.positions.iter().enumerate() {
            let distance = (position[0] - x).abs();
            if distance < min_distance {
                min_distance = distance;
                nearest_point = Some(PointId {
                    series_id,
                    point_index: i,
                });
            }
        }
        nearest_point
    }

    /// Find the nearest point to a given y-coordinate in the plot.
    pub fn nearest_point_vertical(&self, series_id: ShapeId, y: f64) -> Option<PointId> {
        let series = self.series.get(&series_id)?;
        let mut min_distance = f64::INFINITY;
        let mut nearest_point = None;
        for (i, position) in series.positions.iter().enumerate() {
            let distance = (position[1] - y).abs();
            if distance < min_distance {
                min_distance = distance;
                nearest_point = Some(PointId {
                    series_id,
                    point_index: i,
                });
            }
        }
        nearest_point
    }

    /// Add a hover point to the plot.
    pub fn add_hover_point(&mut self, point_id: PointId) {
        if self.handle_hover_pick::<false>(point_id) {
            self.highlight_version = self.highlight_version.wrapping_add(1);
        }
    }
    /// Add a pick point to the plot.
    pub fn add_pick_point(&mut self, point_id: PointId) {
        if self.handle_hover_pick::<true>(point_id) {
            self.highlight_version = self.highlight_version.wrapping_add(1);
        }
    }
    /// Clear all hover points from the plot.
    pub fn clear_hover(&mut self) {
        if !self.hovered_points.is_empty() {
            self.hovered_points.clear();
            self.highlight_version = self.highlight_version.wrapping_add(1);
        }
    }
    /// Clear all pick points from the plot.
    pub fn clear_pick(&mut self) {
        if !self.picked_points.is_empty() {
            self.picked_points.clear();
            self.highlight_version = self.highlight_version.wrapping_add(1);
        }
    }
    fn handle_hover_pick<const PICK: bool>(&mut self, point_id: PointId) -> bool {
        let mut changed = false;
        let (highlight_provider, points) = if PICK {
            // Clicking an already-picked point deselects it.
            if self.picked_points.shift_remove(&point_id).is_some() {
                return true;
            }
            changed |= self.hovered_points.shift_remove(&point_id).is_some();
            (&self.pick_highlight_provider, &mut self.picked_points)
        } else {
            if self.picked_points.contains_key(&point_id) {
                return false;
            }
            (&self.hover_highlight_provider, &mut self.hovered_points)
        };
        if let Some(highlight_provider) = highlight_provider
            && let Some(series) = self.series.get(&point_id.series_id)
            && let Some(position) = series.positions.get(point_id.point_index)
            && let Some(camera_bounds) = &self.camera_bounds
        {
            let mut highlight_point = HighlightPoint {
                x: position[0],
                y: position[1],
                color: series
                    .point_colors
                    .as_ref()
                    .map(|colors| colors[point_id.point_index])
                    .unwrap_or(series.color),
                marker_style: series.marker_style,
                mask_padding: Some(3.0),
            };
            let tooltip_text = highlight_provider(
                TooltipContext {
                    series_id: series.id,
                    series_label: series.label.as_deref().unwrap_or(""),
                    point_index: point_id.point_index,
                },
                &mut highlight_point,
            );
            let tooltip = tooltip_text.map(|text| TooltipUiPayload {
                screen_xy: Self::world_to_screen_position(
                    Self::tooltip_anchor_world(&highlight_point),
                    camera_bounds,
                ),
                text,
            });
            let new_payload = (highlight_point, tooltip);
            match points.entry(point_id) {
                indexmap::map::Entry::Occupied(mut occupied) => {
                    if PartialEq::ne(occupied.get(), &new_payload) {
                        occupied.insert(new_payload);
                        changed = true;
                    }
                }
                indexmap::map::Entry::Vacant(vacant) => {
                    vacant.insert(new_payload);
                    changed = true;
                }
            }
        }
        changed
    }
    /// Handle a message sent to the plot widget.
    pub fn update(&mut self, msg: PlotUiMessage) {
        match msg {
            PlotUiMessage::ToggleLegend => {
                self.legend_collapsed = !self.legend_collapsed;
            }
            PlotUiMessage::ToggleControlsOverlay => {
                if self.controls_help_enabled {
                    self.controls_overlay_open = !self.controls_overlay_open;
                }
            }
            PlotUiMessage::ToggleSeriesVisibility(id) => {
                self.toggle_visibility(&id);
            }
            PlotUiMessage::RenderUpdate(payload) => {
                // Update camera and bounds when ticks are updated (camera changed)
                if let Some(camera_bounds) = payload.camera_bounds
                    && self.camera_bounds != Some(camera_bounds)
                {
                    self.camera_bounds = Some(camera_bounds);
                    // Update tooltip positions when camera/bounds change
                    self.update_tooltip_positions();
                }

                let highlight_changed = match payload.hover_pick {
                    Some(HoverPickEvent::Hover(point_id)) => {
                        self.hovered_points.clear();
                        self.handle_hover_pick::<false>(point_id)
                    }
                    Some(HoverPickEvent::Pick(point_id)) => {
                        self.handle_hover_pick::<true>(point_id)
                    }
                    Some(HoverPickEvent::ClearHover) => {
                        let highlight_changed = !self.hovered_points.is_empty();
                        self.hovered_points.clear();
                        highlight_changed
                    }
                    Some(HoverPickEvent::ClearPick) => {
                        let highlight_changed = !self.picked_points.is_empty();
                        self.picked_points.clear();
                        highlight_changed
                    }
                    _ => false,
                };

                // Trigger data version update to rebuild highlighted_points in PlotState
                if highlight_changed {
                    self.highlight_version = self.highlight_version.wrapping_add(1);
                }
                if payload.clear_cursor_position {
                    self.cursor_ui = None;
                }
                if let Some(c) = payload.cursor_position_ui {
                    self.cursor_ui = Some(c);
                }
                if let Some(ticks) = payload.x_ticks {
                    self.x_ticks = ticks;
                }
                if let Some(ticks) = payload.y_ticks {
                    self.y_ticks = ticks;
                }
            }
        }
    }

    /// View the plot widget.
    pub fn view<'a>(&'a self) -> iced::Element<'a, PlotUiMessage> {
        let plot = widget::shader(self)
            .width(Length::Fill)
            .height(Length::Fill);

        let inner_container = container(plot)
            .padding(2.0)
            .style(|theme: &Theme| container::background(theme.palette().background));

        let legend = if self.legend_enabled {
            legend::legend(self, self.legend_collapsed)
        } else {
            None
        };
        let elements = stack![
            inner_container,
            stack(
                self.hovered_points
                    .values()
                    .chain(self.picked_points.values())
                    .filter_map(|(_, tooltip)| tooltip.as_ref().and_then(|tooltip| {
                        Self::view_tooltip_overlay(tooltip, &self.camera_bounds)
                    }))
            ),
            self.view_top_right_overlay(legend.is_some()),
            self.view_tick_labels(),
            legend,
        ];

        container(axes_labels::stack_with_labels(
            elements,
            &self.x_axis_label,
            &self.y_axis_label,
            self.axis_label_size,
        ))
        .padding(3.0)
        .style(|theme: &Theme| container::background(theme.palette().background))
        .into()
    }

    /// Enable or disable autoscaling on updates (default: enabled)
    pub fn autoscale_on_updates(&mut self, enabled: bool) {
        self.autoscale_on_updates = enabled;
    }

    /// Set hover radius in logical pixels for picking markers (default: 8 px)
    pub fn hover_radius_px(&mut self, radius: f32) {
        self.hover_radius_px = radius.max(0.0);
    }

    /// Set a custom highlighter for picked point.
    pub fn set_pick_highlight_provider(&mut self, provider: HighlightPointProvider) {
        self.pick_highlight_provider = Some(provider);
    }

    /// Set a custom highlighter for hovered point.
    pub fn set_hover_highlight_provider(&mut self, provider: HighlightPointProvider) {
        self.hover_highlight_provider = Some(provider);
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

    /// Set a custom formatter for the x-axis tick labels.
    /// The formatter receives a GridMark (containing the tick value and step size)
    /// and the current visible range on the x-axis.
    pub fn set_x_axis_formatter(&mut self, formatter: TickFormatter) {
        self.x_axis_formatter = Some(formatter);
    }

    /// Set a custom formatter for the y-axis tick labels.
    /// The formatter receives a GridMark (containing the tick value and step size)
    /// and the current visible range on the y-axis.
    pub fn set_y_axis_formatter(&mut self, formatter: TickFormatter) {
        self.y_axis_formatter = Some(formatter);
    }

    /// Set a custom tick producer for generating tick positions along both axes.
    pub fn set_x_tick_producer(&mut self, producer: TickProducer) {
        self.x_tick_producer = Some(producer);
    }

    /// Set a custom tick producer for generating tick positions along the y-axis.
    pub fn set_y_tick_producer(&mut self, producer: TickProducer) {
        self.y_tick_producer = Some(producer);
    }

    /// Set the positions of an existing series.
    pub fn set_series_positions(&mut self, id: &ShapeId, positions: &[[f64; 2]]) {
        if let Some(series) = self.series.get_mut(id) {
            series.positions = positions.to_vec();
            if let Some(colors) = &mut series.point_colors
                && colors.len() != series.positions.len()
            {
                colors.resize(series.positions.len(), series.color);
            }
            self.data_version += 1;
        }
    }

    /// Set per-point colors for an existing series.
    pub fn set_series_point_colors(&mut self, id: &ShapeId, mut colors: Vec<Color>) {
        if let Some(series) = self.series.get_mut(id) {
            if colors.len() != series.positions.len() {
                colors.resize(series.positions.len(), series.color);
            }
            series.point_colors = Some(colors);
            self.data_version += 1;
        }
    }

    pub(crate) fn legend_entries(&self) -> Vec<LegendEntry> {
        let mut out = Vec::new();
        for (id, s) in &self.series {
            if let Some(ref label) = s.label {
                if label.is_empty() {
                    continue;
                }
                if s.positions.is_empty() {
                    continue;
                }
                // Include series that have either markers or lines
                if s.marker_style.is_some() || s.line_style.is_some() {
                    let marker = if let Some(ref marker_style) = s.marker_style {
                        marker_style.marker_type as u32
                    } else {
                        u32::MAX
                    };
                    out.push(LegendEntry {
                        id: *id,
                        label: label.clone(),
                        color: s.color,
                        _marker: marker,
                        _line_style: s.line_style,
                        hidden: self.hidden_shapes.contains(id),
                    });
                }
            }
        }
        // Add vertical reference lines to legend
        for (id, vline) in &self.vlines {
            if let Some(ref label) = vline.label
                && !label.is_empty()
            {
                out.push(LegendEntry {
                    id: *id,
                    label: label.clone(),
                    color: vline.color,
                    _marker: u32::MAX,
                    _line_style: Some(vline.line_style),
                    hidden: self.hidden_shapes.contains(id),
                });
            }
        }
        // Add horizontal reference lines to legend
        for (id, hline) in &self.hlines {
            if let Some(ref label) = hline.label
                && !label.is_empty()
            {
                out.push(LegendEntry {
                    id: *id,
                    label: label.clone(),
                    color: hline.color,
                    _marker: u32::MAX,
                    _line_style: Some(hline.line_style),
                    hidden: self.hidden_shapes.contains(id),
                });
            }
        }
        out
    }

    fn view_tooltip_overlay<'a>(
        payload: &'a TooltipUiPayload,
        camera_bounds: &Option<(Camera, Rectangle)>,
    ) -> Option<Element<'a, PlotUiMessage>> {
        use container::Style;
        const TOOLTIP_ALPHA: f32 = 0.7;
        fn tooltip_style(theme: &Theme) -> container::Style {
            let palette = theme.extended_palette();

            Style {
                background: Some(
                    palette
                        .background
                        .weak
                        .color
                        .scale_alpha(TOOLTIP_ALPHA)
                        .into(),
                ),
                text_color: Some(palette.background.weak.text.scale_alpha(TOOLTIP_ALPHA)),
                border: iced::border::rounded(2),
                ..Style::default()
            }
        }

        // Offset a bit from point position
        const OFFSET: f32 = 8.0;
        let [screen_x, screen_y] = payload.screen_xy?;

        // default: top-left aligned
        let mut top = screen_y + OFFSET;
        let mut right = 0.0;
        let mut bottom = 0.0;
        let mut left = screen_x + OFFSET;
        let mut align_x = alignment::Horizontal::Left;
        let mut align_y = Vertical::Top;

        // flip the tooltip if the point is outside this percentage of the bounds
        const FLIP_PCT: f32 = 0.8;
        if let Some((_, bounds)) = &camera_bounds {
            if screen_y > bounds.height * FLIP_PCT {
                // flip the tooltip to the bottom aligned
                top = 0.0;
                bottom = bounds.height - screen_y + OFFSET;
                align_y = Vertical::Bottom;
            }
            if screen_x > bounds.width * FLIP_PCT {
                // flip the tooltip to the right aligned
                left = 0.0;
                right = bounds.width - screen_x + OFFSET;
                align_x = alignment::Horizontal::Right;
            }
        }

        let tooltip_bubble = container(
            widget::text(&payload.text)
                .size(14.0)
                .wrapping(widget::text::Wrapping::None),
        )
        .padding(6.0)
        .style(tooltip_style);

        // Position tooltip at fixed location relative to point, not following cursor
        Some(
            container(tooltip_bubble)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding {
                    top,
                    right,
                    bottom,
                    left,
                })
                .align_x(align_x)
                .align_y(align_y)
                .style(container::transparent)
                .into(),
        )
    }

    fn view_cursor_overlay(&self) -> Option<Element<'_, PlotUiMessage>> {
        if !self.cursor_overlay {
            return None;
        }

        let Some(payload) = &self.cursor_ui else {
            return None;
        };

        let bubble = container(widget::text(payload.text.clone()).size(12.0))
            .padding(6.0)
            .style(container::rounded_box);

        Some(bubble.into())
    }

    fn view_top_right_overlay(&self, has_legend: bool) -> Element<'_, PlotUiMessage> {
        let help_btn = self.controls_help_enabled.then(|| {
            let help_label = if self.controls_overlay_open {
                "×"
            } else {
                "?"
            };

            widget::button(widget::text(help_label).size(12.0))
                .padding(6.0)
                .on_press(PlotUiMessage::ToggleControlsOverlay)
        });

        let top_row = widget::row![self.view_cursor_overlay(), help_btn].spacing(6.0);
        let col = widget::column![top_row, self.view_controls_overlay_panel(has_legend)]
            .spacing(6.0)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .align_x(Horizontal::Right);

        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: 4.0,
                right: 4.0,
                ..Padding::ZERO
            })
            .align_x(Horizontal::Right)
            .align_y(Vertical::Top)
            .style(container::transparent)
            .into()
    }

    fn view_controls_overlay_panel(&self, has_legend: bool) -> Option<Element<'_, PlotUiMessage>> {
        if !self.controls_help_enabled || !self.controls_overlay_open {
            return None;
        }

        let txt = |t| widget::text(t).size(12.0).style(widget::text::base);
        let content = widget::column![
            txt("Controls").style(widget::text::primary),
            txt("Left-drag: pan"),
            txt("Right-drag: box zoom"),
            txt("Ctrl + scroll: zoom at cursor"),
            txt("Scroll: pan (vertical/horizontal)"),
            txt("Double-click: reset / autoscale"),
            txt("Left-click point: pick"),
            txt("Esc: clear picked points"),
            has_legend.then(|| txt("Click icon in legend to toggle visibility.")),
        ]
        .spacing(2.0);

        Some(
            container(content)
                .padding(8.0)
                .style(container::rounded_box)
                .into(),
        )
    }

    fn view_tick_labels(&self) -> Option<Element<'_, PlotUiMessage>> {
        if self.x_ticks.is_empty() && self.y_ticks.is_empty() {
            return None;
        }

        let mut tick_elements = Vec::with_capacity(self.x_ticks.len() + self.y_ticks.len());
        let tick_text = |text| widget::text(text).size(self.tick_label_size);

        if let Some(formatter) = &self.x_axis_formatter {
            for tick in &self.x_ticks {
                let label_text = formatter(tick.tick);
                let centering_offset = 2.0 * (label_text.len() as f32); // A bit of a fudge.
                let text_widget = tick_text(label_text);
                let positioned_label = container(text_widget)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(padding::left(tick.screen_pos - centering_offset))
                    .align_x(Horizontal::Left)
                    .align_y(Vertical::Bottom)
                    .style(container::transparent);
                tick_elements.push(positioned_label.into());
            }
        }

        if let Some(formatter) = &self.y_axis_formatter {
            for tick in &self.y_ticks {
                let label_text = formatter(tick.tick);
                let text_widget = tick_text(label_text);
                let positioned_label = widget::container(text_widget)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(padding::top(tick.screen_pos - 5.0))
                    .align_x(alignment::Horizontal::Left)
                    .align_y(Vertical::Top)
                    .style(container::transparent);
                tick_elements.push(positioned_label.into());
            }
        }

        if tick_elements.is_empty() {
            return None;
        }

        Some(stack(tick_elements).into())
    }

    fn toggle_visibility(&mut self, id: &ShapeId) {
        let exists = self.series.contains_key(id)
            || self.vlines.contains_key(id)
            || self.hlines.contains_key(id);

        if !exists {
            println!("Toggle visibility: series not found: {id}");
            return;
        }
        if self.hidden_shapes.contains(id) {
            self.hidden_shapes.remove(id);
        } else {
            self.hidden_shapes.insert(*id);
        }
        self.data_version += 1;
    }
}

#[doc(hidden)]
pub struct Primitive {
    instance_id: u64,
    plot_widget: PlotState,
}

impl std::fmt::Debug for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Primitive")
            .field("instance_id", &self.instance_id)
            .finish_non_exhaustive()
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
        Primitive {
            instance_id: self.instance_id,
            plot_widget: state.clone(),
        }
    }
    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<shader::Action<PlotUiMessage>> {
        let mut needs_redraw = false;
        let mut publish_hover_pick: Option<HoverPickEvent> = None;
        let mut publish_cursor: Option<CursorPositionUiPayload> = None;
        let mut clear_cursor_position = false;

        // Sync highlight overlay data without rebuilding plot geometry.
        if self.highlight_version != state.highlight_src_version {
            let changed = state.sync_highlighted_points_from_widget(self);
            state.highlight_src_version = self.highlight_version;
            needs_redraw |= changed;
        }

        if self.data_version != state.data_src_version {
            // Rebuild derived state from widget data
            state.rebuild_from_widget(self);

            // Invalidate hover cache when data changes so hover/pick can be recomputed
            state.last_hover_cache = None;
            state.hover_version = state.hover_version.wrapping_add(1);

            // Submit a picking request if we have a cursor position and hover is enabled
            if state.hover_enabled && !state.pan.active && !state.selection.active {
                let inside = state.cursor_inside();
                if inside {
                    if state.points.len() < CPU_PICK_THRESHOLD {
                        if let Some(point) = cpu_pick_hit(state)
                            && self.valid_point_id(&point)
                        {
                            publish_hover_pick = Some(HoverPickEvent::Hover(point));
                        } else if !self.hovered_points.is_empty() {
                            publish_hover_pick = Some(HoverPickEvent::ClearHover);
                        }
                    } else {
                        state.pick_seq = state.pick_seq.wrapping_add(1);
                        picking::submit_request(
                            self.instance_id,
                            crate::picking::PickRequest {
                                cursor_x: state.cursor_position.x,
                                cursor_y: state.cursor_position.y,
                                radius_px: state.hover_radius_px,
                                seq: state.pick_seq,
                            },
                        );
                    }
                }
            }

            // Autoscale on first update or always if autoscale_on_updates is enabled.
            if state.data_src_version == 0 || self.autoscale_on_updates {
                state.autoscale();
            }

            state.data_src_version = self.data_version;
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
        state.hover_enabled =
            self.hover_highlight_provider.is_some() || self.pick_highlight_provider.is_some();
        state.hover_radius_px = self.hover_radius_px;
        state.crosshairs_enabled = self.crosshairs_enabled;

        // viewport size (screen pixels for this widget)
        let viewport = Vec2::new(state.bounds.width, state.bounds.height);

        match event {
            iced::Event::Mouse(mouse_event) => {
                needs_redraw |=
                    state.handle_mouse_event(*mouse_event, self, &mut publish_hover_pick);

                // If cursor moved and hover enabled, submit a GPU pick request
                if let iced::mouse::Event::CursorMoved { .. } = mouse_event
                    && state.hover_enabled
                    && !state.pan.active
                    && !state.selection.active
                {
                    // Only submit pick request if cursor is within widget bounds
                    let inside = state.cursor_inside();
                    if inside {
                        if state.points.len() < CPU_PICK_THRESHOLD {
                            if let Some(point) = cpu_pick_hit(state)
                                && self.valid_point_id(&point)
                            {
                                publish_hover_pick = Some(HoverPickEvent::Hover(point));
                            } else if !self.hovered_points.is_empty() {
                                // Moved but not over any point - clear hover
                                publish_hover_pick = Some(HoverPickEvent::ClearHover);
                            }
                        } else {
                            picking::submit_request(
                                self.instance_id,
                                crate::picking::PickRequest {
                                    cursor_x: state.cursor_position.x,
                                    cursor_y: state.cursor_position.y,
                                    radius_px: state.hover_radius_px,
                                    seq: state.pick_seq,
                                },
                            );

                            // Increment pick sequence number to track the outstanding request.
                            state.pick_seq = state.pick_seq.wrapping_add(1);
                        }
                    } else if !self.hovered_points.is_empty() {
                        // Cursor left bounds - clear hover
                        publish_hover_pick = Some(HoverPickEvent::ClearHover);
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

                // Handle cursor leave
                if let iced::mouse::Event::CursorLeft = mouse_event
                    && !self.hovered_points.is_empty()
                {
                    publish_hover_pick = Some(HoverPickEvent::ClearHover);
                }
            }
            // CursorLeft is handled inside the Mouse(...) branch above via state.handle_mouse_event
            iced::Event::Keyboard(keyboard_event) => {
                if let keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                } = keyboard_event
                {
                    publish_hover_pick = Some(HoverPickEvent::ClearPick);
                }
                needs_redraw |= state.handle_keyboard_event(keyboard_event);
            }
            _ => {}
        }

        if let Some(aspect) = self.data_aspect
            && apply_data_aspect(&mut state.camera, &state.bounds, aspect)
        {
            needs_redraw = true;
        }

        // Process picking results after event handling (works for both mouse events and data updates)
        if state.hover_enabled && state.points.len() >= CPU_PICK_THRESHOLD {
            // Try to consume a GPU pick result for this instance
            if let Some(res) = picking::take_result(self.instance_id) {
                // Only process if this result is newer than the last processed one
                if res.seq > state.pick_result_seq {
                    // Only process GPU pick results if we haven't already set publish_hover_pick (e.g., from ESC key)
                    if publish_hover_pick.is_none() {
                        // Check if this is a select request
                        if state.pending_gpu_pick_seq == Some(res.seq) {
                            state.pending_gpu_pick_seq = None;
                            // Process as select
                            if let Some(point) = res.hit
                                && self.valid_point_id(&point)
                            {
                                publish_hover_pick = Some(HoverPickEvent::Pick(point));
                            }
                        } else {
                            // Process as hover
                            if let Some(point) = res.hit
                                && self.valid_point_id(&point)
                            {
                                // Update hover cache
                                state.last_hover_cache = Some(point);
                                publish_hover_pick = Some(HoverPickEvent::Hover(point));
                            } else {
                                // Moved but not over any point - clear hover
                                if state.last_hover_cache.is_some()
                                    || !self.hovered_points.is_empty()
                                {
                                    state.last_hover_cache = None;
                                    publish_hover_pick = Some(HoverPickEvent::ClearHover);
                                }
                            }
                        }
                    }
                    // Always update pick_result_seq to mark this result as processed, even if we skipped processing
                    state.pick_result_seq = res.seq;
                }
            }
        }

        // If we have an outstanding GPU pick request, keep drawing until the result arrives.
        needs_redraw |= state.pick_seq > state.pick_result_seq;

        let mut publish_x_ticks = None;
        let mut publish_y_ticks = None;

        if needs_redraw {
            // If we need to redraw, there's a good chance we need to update the ticks.
            state.update_ticks(self.x_tick_producer.as_ref(), self.y_tick_producer.as_ref());
            publish_x_ticks = Some(state.x_ticks.clone());
            publish_y_ticks = Some(state.y_ticks.clone());
        }

        let needs_publish = publish_hover_pick.is_some()
            || publish_cursor.is_some()
            || publish_x_ticks.is_some()
            || publish_y_ticks.is_some()
            || clear_cursor_position;

        if needs_publish {
            // Include camera and bounds info when we have a hover/pick event or when ticks are updated
            let camera_bounds = if publish_hover_pick.is_some()
                || publish_x_ticks.is_some()
                || publish_y_ticks.is_some()
            {
                Some((state.camera, state.bounds))
            } else {
                None
            };

            return Some(shader::Action::publish(PlotUiMessage::RenderUpdate(
                PlotRenderUpdate {
                    hover_pick: publish_hover_pick,
                    clear_cursor_position,
                    cursor_position_ui: publish_cursor,
                    x_ticks: publish_x_ticks,
                    y_ticks: publish_y_ticks,
                    camera_bounds,
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

#[doc(hidden)]
pub struct PlotRendererState {
    renderers: HashMap<u64, PlotRenderer>,
    format: TextureFormat,
}

impl shader::Primitive for Primitive {
    type Pipeline = PlotRendererState;

    fn prepare(
        &self,
        renderer_state: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        // Get or create renderer for this widget instance.
        let renderer = renderer_state
            .renderers
            .entry(self.instance_id)
            .or_insert_with(|| PlotRenderer::new(device, queue, renderer_state.format));

        renderer.prepare_frame(device, queue, viewport, bounds, &self.plot_widget);
        renderer.service_picking(self.instance_id, device, queue, &self.plot_widget);
    }

    fn render(
        &self,
        renderer_state: &Self::Pipeline,
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

/// Threshold for number of points above which GPU picking is used instead of CPU picking.
const CPU_PICK_THRESHOLD: usize = 5000;

fn cpu_pick_hit(state: &PlotState) -> Option<PointId> {
    if state.points.is_empty() || state.series.is_empty() {
        return None;
    }

    let width = state.bounds.width.max(1.0) as f64;
    let height = state.bounds.height.max(1.0) as f64;
    let cursor_x = state.cursor_position.x as f64;
    let cursor_y = state.cursor_position.y as f64;

    let mut span_idx = 0usize;
    let mut span_start = 0usize;
    let mut best: Option<(usize, f64)> = None;

    for (idx, pt) in state.points.iter().enumerate() {
        while span_idx < state.series.len() && idx >= span_start + state.series[span_idx].len {
            span_start += state.series[span_idx].len;
            span_idx += 1;
        }
        if span_idx >= state.series.len() {
            break;
        }

        let world = marker_center_world(pt);
        let ndc_x = (world.x - state.camera.position.x) / state.camera.half_extents.x;
        let ndc_y = (world.y - state.camera.position.y) / state.camera.half_extents.y;
        let screen_x = (ndc_x + 1.0) * 0.5 * width;
        let screen_y = (1.0 - ndc_y) * 0.5 * height;

        let dx = screen_x - cursor_x;
        let dy = screen_y - cursor_y;
        let d2 = dx * dx + dy * dy;
        let marker_px =
            MarkerSize::marker_size_px(pt.size, pt.size_mode, &state.camera, &state.bounds) as f64;
        let radius = state.hover_radius_px as f64 + marker_px * 0.5;
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
    while span_idx < state.series.len() && best_idx >= span_start + state.series[span_idx].len {
        span_start += state.series[span_idx].len;
        span_idx += 1;
    }
    let span = state.series.get(span_idx)?;
    let local_idx = best_idx - span_start;
    Some(PointId {
        series_id: span.id,
        point_index: local_idx,
    })
}

impl PlotWidget {
    /// Check if the point index is valid for the series
    fn valid_point_id(&self, point_id: &PointId) -> bool {
        self.series
            .get(&point_id.series_id)
            .map(|series| point_id.point_index < series.positions.len())
            .unwrap_or(false)
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

fn apply_data_aspect(camera: &mut Camera, bounds: &Rectangle, aspect: f64) -> bool {
    let width = bounds.width.max(1.0) as f64;
    let height = bounds.height.max(1.0) as f64;
    let target_half_y = aspect * camera.half_extents.x * (height / width);
    if (camera.half_extents.y - target_half_y).abs() > f64::EPSILON {
        camera.half_extents.y = target_half_y;
        return true;
    }
    false
}

impl PlotWidget {
    pub(crate) fn pick_hit(&self, state: &mut PlotState) -> Option<PointId> {
        if let Some(point) = state.last_hover_cache
            && self.valid_point_id(&point)
        {
            return Some(point);
        }
        if state.points.len() < CPU_PICK_THRESHOLD {
            if let Some(point) = cpu_pick_hit(state)
                && self.valid_point_id(&point)
            {
                return Some(point);
            }
        } else {
            state.pick_seq = state.pick_seq.wrapping_add(1);
            let seq = state.pick_seq;
            state.pending_gpu_pick_seq = Some(seq);
            picking::submit_request(
                self.instance_id,
                crate::picking::PickRequest {
                    cursor_x: state.cursor_position.x,
                    cursor_y: state.cursor_position.y,
                    radius_px: state.hover_radius_px,
                    seq,
                },
            );
        }
        None
    }
}

impl Pipeline for PlotRendererState {
    fn new(
        _device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self
    where
        Self: Sized,
    {
        PlotRendererState {
            renderers: HashMap::new(),
            format,
        }
    }
}

// Global unique ID generator for widget instances
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The highlight context for a point. You can modify
///
/// + `x` and `y` to change the position of the highlight point (not recommended);
/// + `color` to change the color of the highlight point;
/// + `marker_style` to change the marker style of the highlight point;
/// + `mask_padding` to change the mask padding of the highlight point;
///
///  to change the highlight point.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightPoint {
    /// Data-space coordinates
    pub x: f64,
    /// Data-space coordinates
    pub y: f64,
    pub color: Color,
    /// Optional marker style for the series. If None, no markers are drawn.
    pub marker_style: Option<MarkerStyle>,
    /// Mask padding in pixels. If None, no mask is drawn.
    pub mask_padding: Option<f32>,
}

impl HighlightPoint {
    /// Resize the marker of the highlight point.
    /// For both pixel-based and world-based markers, the size will be multiplied by the factor.
    pub fn resize_marker(&mut self, factor: f64) {
        if let Some(marker_style) = &mut self.marker_style {
            match &mut marker_style.size {
                MarkerSize::Pixels(size) => {
                    *size *= factor as f32;
                }
                MarkerSize::World(size) => {
                    *size *= factor;
                }
            }
        }
    }
}

/// Convert world position to screen position
pub(crate) fn world_to_screen_position_x(
    x: f64,
    camera: &Camera,
    bounds: &Rectangle,
) -> Option<f32> {
    let ndc_x = (x - camera.position.x) / camera.half_extents.x;
    let screen_x = (ndc_x as f32 + 1.0) * 0.5 * bounds.width;

    if screen_x < 0.0 || screen_x > bounds.width {
        None
    } else {
        Some(screen_x)
    }
}

/// Convert world position to screen position
pub(crate) fn world_to_screen_position_y(
    y: f64,
    camera: &Camera,
    bounds: &Rectangle,
) -> Option<f32> {
    let ndc_y = (y - camera.position.y) / camera.half_extents.y;
    let screen_y = (1.0 - ndc_y as f32) * 0.5 * bounds.height;

    if screen_y < 0.0 || screen_y > bounds.height {
        None
    } else {
        Some(screen_y)
    }
}
