use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use glam::{DVec2, Vec2};
use iced::{
    Color, Element, Length, Padding, Rectangle, Theme,
    alignment::{self, Horizontal},
    mouse::{self, Interaction},
    padding,
    wgpu::TextureFormat,
    widget::{
        self, container,
        shader::{self, Viewport},
        stack,
    },
};

use crate::{
    HLine, PlotUiMessage, Series, TooltipContext, VLine, axes_labels,
    axis_link::AxisLink,
    legend::{self, LegendEntry},
    message::{CursorPositionUiPayload, PlotRenderUpdate, TooltipUiPayload},
    picking,
    plot_renderer::{PlotRenderer, RenderParams},
    plot_state::{HoverHit, PlotState},
    series::SeriesError,
    ticks::{self, PositionedTick, TickFormatter, TickProducer},
};

pub type TooltipProvider = Arc<dyn Fn(&TooltipContext) -> String + Send + Sync>;
pub type CursorProvider = Arc<dyn Fn(f64, f64) -> String + Send + Sync>;

/// A plot widget that renders data series with interactive features.
pub struct PlotWidget {
    pub(crate) instance_id: u64,
    // Data
    pub(crate) series: Vec<Series>,
    pub(crate) vlines: Vec<VLine>,
    pub(crate) hlines: Vec<HLine>,
    pub(crate) hidden_labels: HashSet<String>,
    pub(crate) data_version: u64,
    // Configuration
    pub(crate) autoscale_on_updates: bool,
    pub(crate) legend_collapsed: bool,
    pub(crate) x_axis_label: String,
    pub(crate) y_axis_label: String,
    pub(crate) x_lim: Option<(f64, f64)>,
    pub(crate) y_lim: Option<(f64, f64)>,
    pub(crate) x_axis_link: Option<AxisLink>,
    pub(crate) y_axis_link: Option<AxisLink>,
    pub(crate) tooltips_enabled: bool,
    pub(crate) hover_radius_px: f32,
    pub(crate) tooltip_provider: Option<TooltipProvider>,
    pub(crate) cursor_overlay: bool,
    pub(crate) cursor_provider: Option<CursorProvider>,
    pub(crate) crosshairs_enabled: bool,
    pub(crate) x_axis_formatter: Option<TickFormatter>,
    pub(crate) y_axis_formatter: Option<TickFormatter>,
    pub(crate) x_tick_producer: Option<TickProducer>,
    pub(crate) y_tick_producer: Option<TickProducer>,
    pub(crate) tick_label_size: f32,
    pub(crate) axis_label_size: f32,
    // UI state
    pub(crate) tooltip_ui: Option<TooltipUiPayload>,
    pub(crate) cursor_ui: Option<CursorPositionUiPayload>,
    pub(crate) x_ticks: Vec<PositionedTick>,
    pub(crate) y_ticks: Vec<PositionedTick>,
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
            series: Vec::new(),
            vlines: Vec::new(),
            hlines: Vec::new(),
            hidden_labels: HashSet::new(),
            data_version: 0,
            autoscale_on_updates: false,
            legend_collapsed: false,
            x_axis_label: String::new(),
            y_axis_label: String::new(),
            x_lim: None,
            y_lim: None,
            x_axis_link: None,
            y_axis_link: None,
            tooltips_enabled: true,
            hover_radius_px: 8.0,
            tooltip_provider: None,
            cursor_overlay: true,
            cursor_provider: None,
            crosshairs_enabled: false,
            x_axis_formatter: Some(Arc::new(ticks::default_formatter)),
            y_axis_formatter: Some(Arc::new(ticks::default_formatter)),
            x_tick_producer: Some(Arc::new(ticks::default_tick_producer)),
            y_tick_producer: Some(Arc::new(ticks::default_tick_producer)),
            tick_label_size: 10.0,
            axis_label_size: 16.0,
            x_ticks: Vec::new(),
            y_ticks: Vec::new(),
            tooltip_ui: None,
            cursor_ui: None,
        }
    }

    /// Add a data series to the plot.
    pub fn add_series(&mut self, item: Series) -> Result<(), SeriesError> {
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
        self.data_version += 1;
        Ok(())
    }

    /// Remove a data series from the plot by its label.
    pub fn remove_series(&mut self, label: &str) -> bool {
        if let Some(idx) = self
            .series
            .iter()
            .position(|s| s.label.as_deref() == Some(label))
        {
            self.series.remove(idx);
            self.hidden_labels.remove(label);
            self.data_version += 1;
            return true;
        }
        false
    }

    /// Add a vertical reference line to the plot.
    pub fn add_vline(&mut self, vline: VLine) -> Result<(), SeriesError> {
        // Enforce unique (or empty) labels
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
        self.data_version += 1;
        Ok(())
    }

    /// Add a horizontal reference line to the plot.
    pub fn add_hline(&mut self, hline: HLine) -> Result<(), SeriesError> {
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
        self.data_version += 1;
        Ok(())
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

    /// Handle a message sent to the plot widget.
    pub fn update(&mut self, msg: PlotUiMessage) {
        match msg {
            PlotUiMessage::ToggleLegend => {
                self.legend_collapsed = !self.legend_collapsed;
            }
            PlotUiMessage::ToggleSeriesVisibility(label) => {
                self.toggle_visibility(&label);
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

        let elements = stack![
            inner_container,
            self.view_tooltip_overlay(),
            self.view_cursor_overlay(),
            self.view_tick_labels(),
            legend::legend(self, self.legend_collapsed),
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
    pub fn set_series_positions(&mut self, label: &str, positions: &[[f64; 2]]) {
        if let Some(idx) = self
            .series
            .iter()
            .position(|s| s.label.as_deref() == Some(label))
        {
            self.series[idx].positions = positions.to_vec();
            self.data_version += 1;
        }
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
                    let marker = if let Some(ref marker_style) = s.marker_style {
                        marker_style.marker_type as u32
                    } else {
                        u32::MAX
                    };
                    out.push(LegendEntry {
                        label: label.clone(),
                        color: s.color,
                        _marker: marker,
                        _line_style: s.line_style,
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
                    _marker: u32::MAX,
                    _line_style: Some(vline.line_style),
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
                    _marker: u32::MAX,
                    _line_style: Some(hline.line_style),
                    hidden: self.hidden_labels.contains(label),
                });
            }
        }
        out
    }

    fn view_tooltip_overlay(&self) -> Option<Element<'_, PlotUiMessage>> {
        let Some(payload) = &self.tooltip_ui else {
            return None;
        };

        // Offset a bit from cursor
        let offset_x = payload.x + 8.0;
        let offset_y = payload.y + 8.0;

        let bubble = container(widget::text(payload.text.clone()).size(14.0))
            .padding(6.0)
            .style(container::rounded_box);

        let overlay = container(bubble)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                left: offset_x,
                right: 0.0,
                top: offset_y,
                bottom: 0.0,
            })
            .align_x(Horizontal::Left)
            .align_y(alignment::Vertical::Top)
            .style(|_| container::background(Color::TRANSPARENT))
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

        let bubble = container(widget::text(payload.text.clone()).size(12.0))
            .padding(6.0)
            .style(container::rounded_box);

        Some(
            container(bubble)
                .width(Length::Fill)
                .height(Length::Shrink)
                .align_x(Horizontal::Right)
                .align_y(alignment::Vertical::Top)
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
                    .align_y(alignment::Vertical::Bottom)
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
                    .align_y(alignment::Vertical::Top)
                    .style(container::transparent);
                tick_elements.push(positioned_label.into());
            }
        }

        if tick_elements.is_empty() {
            return None;
        }

        Some(stack(tick_elements).into())
    }

    fn toggle_visibility(&mut self, label: &str) {
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
            println!("Toggle visibility: series not found: {label}");
            return;
        }

        if self.hidden_labels.contains(label) {
            self.hidden_labels.remove(label);
        } else {
            self.hidden_labels.insert(label.to_string());
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
        let mut clear_tooltip = false;

        if self.data_version != state.src_version {
            // Rebuild derived state from widget data
            state.rebuild_from_widget(self);

            // Invalidate hover cache when data changes so tooltips update
            state.last_hover_cache = None;
            state.hovered_world = None;
            state.hover_version = state.hover_version.wrapping_add(1);
            clear_tooltip = true;

            // Submit a picking request if we have a cursor position and hover is enabled
            if state.hover_enabled && !state.pan.active && !state.selection.active {
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
            }

            // Autoscale on first update or always if autoscale_on_updates is enabled.
            if state.src_version == 0 || self.autoscale_on_updates {
                state.autoscale();
            }

            state.src_version = self.data_version;
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
        state.hover_enabled = self.tooltips_enabled;
        state.hover_radius_px = self.hover_radius_px;
        state.crosshairs_enabled = self.crosshairs_enabled;

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
            }
            // CursorLeft is handled inside the Mouse(...) branch above via state.handle_mouse_event
            iced::Event::Keyboard(keyboard_event) => {
                needs_redraw |= state.handle_keyboard_event(keyboard_event.clone());
            }
            _ => {}
        }

        // Process picking results after event handling (works for both mouse events and data updates)
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
                            // Default: just show coordinates
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
                        if state.last_hover_cache.is_some() {
                            state.hovered_world = None;
                            state.last_hover_cache = None;
                            state.hover_version = state.hover_version.wrapping_add(1);
                            clear_tooltip = true;
                            needs_redraw = true;
                        }
                    }
                }
            }
        }

        let mut publish_x_ticks = None;
        let mut publish_y_ticks = None;

        if needs_redraw {
            // If we need to redraw, there's a good chance we need to update the ticks.
            state.update_ticks(self.x_tick_producer.as_ref(), self.y_tick_producer.as_ref());
            publish_x_ticks = Some(state.x_ticks.clone());
            publish_y_ticks = Some(state.y_ticks.clone());
        }

        let needs_publish = publish_tooltip.is_some()
            || publish_cursor.is_some()
            || publish_x_ticks.is_some()
            || publish_y_ticks.is_some()
            || clear_tooltip
            || clear_cursor_position;

        if needs_publish {
            return Some(shader::Action::publish(PlotUiMessage::RenderUpdate(
                PlotRenderUpdate {
                    clear_tooltip,
                    clear_cursor_position,
                    tooltip_ui: publish_tooltip,
                    cursor_position_ui: publish_cursor,
                    x_ticks: publish_x_ticks,
                    y_ticks: publish_y_ticks,
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

// Global unique ID generator for widget instances
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
