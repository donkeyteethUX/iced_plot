use std::sync::Arc;

use crate::axis_link::AxisLink;
use crate::message::TooltipContext;
use crate::plot_widget::{CursorProvider, HighlightPoint, HighlightPointProvider, PlotWidget};
use crate::reference_lines::{HLine, VLine};
use crate::series::{Series, SeriesError};
use crate::ticks::{Tick, TickFormatter, TickProducer};

/// Builder for configuring and constructing a PlotWidget.
///
/// # Example
///
/// ```ignore
/// let plot = PlotWidgetBuilder::new()
///     .with_x_label("Time (s)")
///     .with_y_label("Value (V)")
///     .with_autoscale_on_updates(false)
///     .with_x_lim(0.0, 10.0)
///     .with_y_lim(-1.0, 1.0)
///     .add_series(series)
///     .build()?;
/// ```
#[derive(Default)]
pub struct PlotWidgetBuilder {
    x_label: Option<String>,
    y_label: Option<String>,
    autoscale_on_updates: Option<bool>,
    hover_radius_px: Option<f32>,
    pick_highlight_provider: Option<HighlightPointProvider>,
    hover_highlight_provider: Option<HighlightPointProvider>,
    cursor_overlay: Option<bool>,
    cursor_provider: Option<CursorProvider>,
    crosshairs: Option<bool>,
    disable_controls_help: bool,
    disable_legend: bool,
    x_lim: Option<(f64, f64)>,
    y_lim: Option<(f64, f64)>,
    x_axis_link: Option<AxisLink>,
    y_axis_link: Option<AxisLink>,
    x_tick_formatter: Option<TickFormatter>,
    y_tick_formatter: Option<TickFormatter>,
    x_tick_producer: Option<TickProducer>,
    y_tick_producer: Option<TickProducer>,
    enable_x_tick_labels: Option<bool>,
    enable_y_tick_labels: Option<bool>,
    tick_label_size: Option<f32>,
    axis_label_size: Option<f32>,
    data_aspect: Option<f64>,
    series: Vec<Series>,
    vlines: Vec<VLine>,
    hlines: Vec<HLine>,
}

impl PlotWidgetBuilder {
    /// Create a new PlotWidgetBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the x-axis label for the plot.
    pub fn with_x_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.x_label = Some(l);
        }
        self
    }

    /// Set the y-axis label for the plot.
    pub fn with_y_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.y_label = Some(l);
        }
        self
    }

    /// Enable or disable autoscaling of the plot when new data is added.
    pub fn with_autoscale_on_updates(mut self, enabled: bool) -> Self {
        self.autoscale_on_updates = Some(enabled);
        self
    }

    /// Set the hover radius in pixels for detecting nearby points for highlighting.
    pub fn with_hover_radius_px(mut self, radius: f32) -> Self {
        self.hover_radius_px = Some(radius.max(0.0));
        self
    }

    /// Provide a custom highlighter for pick point.
    pub fn with_pick_highlight_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(TooltipContext<'_>, &mut HighlightPoint) -> Option<String> + Send + Sync + 'static,
    {
        self.pick_highlight_provider = Some(Arc::new(provider));
        self
    }

    /// Provide a custom highlighter for hovered point.
    ///
    /// If not provided, a default hover highlight provider will be used that shows the tooltip text with
    /// series label, x and y coordinates of the point ([`PlotWidgetBuilder::default_hover_highlight_provider`]).
    pub fn with_hover_highlight_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(TooltipContext<'_>, &mut HighlightPoint) -> Option<String> + Send + Sync + 'static,
    {
        self.hover_highlight_provider = Some(Arc::new(provider));
        self
    }

    /// Enable or disable the small cursor position overlay shown in the
    /// lower-left corner of the plot. By default it's disabled when not set.
    pub fn with_cursor_overlay(mut self, enabled: bool) -> Self {
        self.cursor_overlay = Some(enabled);
        self
    }

    /// Provide a custom formatter for the cursor overlay. Called with
    /// (x, y) world coordinates and should return the formatted string.
    pub fn with_cursor_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(f64, f64) -> String + Send + Sync + 'static,
    {
        self.cursor_provider = Some(Arc::new(provider));
        self
    }

    /// Enable or disable crosshairs that follow the cursor position.
    pub fn with_crosshairs(mut self, enabled: bool) -> Self {
        self.crosshairs = Some(enabled);
        self
    }

    /// Disable the in-canvas controls/help UI (`?` button + panel).
    ///
    /// Useful if your application provides its own help UI or you want a cleaner canvas.
    pub fn disable_controls_help(mut self) -> Self {
        self.disable_controls_help = true;
        self
    }

    /// Disable the legend.
    ///
    /// Useful if your application provides its own help UI or you want a cleaner canvas.
    ///
    /// By default, when plot contains at least one labeled series, the legend is enabled.
    pub fn disable_legend(mut self) -> Self {
        self.disable_legend = true;
        self
    }

    /// Set the x-axis limits (min, max) for the plot.
    /// If set, these will override autoscaling for the x-axis.
    pub fn with_x_lim(mut self, min: f64, max: f64) -> Self {
        self.x_lim = Some((min, max));
        self
    }

    /// Set the y-axis limits (min, max) for the plot.
    /// If set, these will override autoscaling for the y-axis.
    pub fn with_y_lim(mut self, min: f64, max: f64) -> Self {
        self.y_lim = Some((min, max));
        self
    }

    /// Link the x-axis to other plots. When the x-axis is panned or zoomed,
    /// all plots sharing this link will update synchronously.
    pub fn with_x_axis_link(mut self, link: AxisLink) -> Self {
        self.x_axis_link = Some(link);
        self
    }

    /// Link the y-axis to other plots. When the y-axis is panned or zoomed,
    /// all plots sharing this link will update synchronously.
    pub fn with_y_axis_link(mut self, link: AxisLink) -> Self {
        self.y_axis_link = Some(link);
        self
    }

    /// Set a custom formatter for the x-axis tick labels.
    pub fn with_x_tick_formatter<F>(mut self, formatter: F) -> Self
    where
        F: Fn(Tick) -> String + Send + Sync + 'static,
    {
        self.x_tick_formatter = Some(Arc::new(formatter));
        self
    }

    /// Set a custom formatter for the y-axis tick labels.
    pub fn with_y_tick_formatter<F>(mut self, formatter: F) -> Self
    where
        F: Fn(Tick) -> String + Send + Sync + 'static,
    {
        self.y_tick_formatter = Some(Arc::new(formatter));
        self
    }

    /// Set a custom tick producer for generating tick positions along the x-axis.
    pub fn with_x_tick_producer<F>(mut self, producer: F) -> Self
    where
        F: Fn(f64, f64) -> Vec<Tick> + Send + Sync + 'static,
    {
        self.x_tick_producer = Some(Arc::new(producer));
        self
    }

    /// Set a custom tick producer for generating tick positions along the y-axis.
    pub fn with_y_tick_producer<F>(mut self, producer: F) -> Self
    where
        F: Fn(f64, f64) -> Vec<Tick> + Send + Sync + 'static,
    {
        self.y_tick_producer = Some(Arc::new(producer));
        self
    }

    /// Set whether tick labels for the x axis will be shown.
    pub fn with_x_tick_labels(mut self, enabled: bool) -> Self {
        self.enable_x_tick_labels = Some(enabled);
        self
    }

    /// Set whether tick labels for the y axis will be shown.
    pub fn with_y_tick_labels(mut self, enabled: bool) -> Self {
        self.enable_y_tick_labels = Some(enabled);
        self
    }

    /// Set the font size for tick labels (the numbers on the axes).
    pub fn with_tick_label_size(mut self, size: f32) -> Self {
        self.tick_label_size = Some(size.max(1.0));
        self
    }

    /// Set the font size for axis labels.
    pub fn with_axis_label_size(mut self, size: f32) -> Self {
        self.axis_label_size = Some(size.max(1.0));
        self
    }

    /// Set the width-to-height aspect ratio of the data in the plot.
    ///
    /// For example, you may want to use 1.0 if both axes are in the same units.
    pub fn with_data_aspect(mut self, aspect: f64) -> Self {
        if aspect.is_sign_positive() {
            self.data_aspect = Some(aspect);
        } else {
            self.data_aspect = None;
        }
        self
    }

    /// Add a [`Series`] to the plot.
    pub fn add_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Add a vertical reference line to the plot.
    pub fn add_vline(mut self, vline: VLine) -> Self {
        self.vlines.push(vline);
        self
    }

    /// Add a horizontal reference line to the plot.
    pub fn add_hline(mut self, hline: HLine) -> Self {
        self.hlines.push(hline);
        self
    }

    /// Disable background grid lines and ticks on both axes.
    pub fn without_grid(self) -> Self {
        self.with_x_tick_producer(|_, _| Vec::new())
            .with_y_tick_producer(|_, _| Vec::new())
    }

    /// Default hover highlight provider that shows the tooltip text with
    /// series label, x and y coordinates of the point.
    pub fn default_hover_highlight_provider(
        ctx: TooltipContext<'_>,
        point: &mut HighlightPoint,
    ) -> Option<String> {
        Some(format!(
            "{}\nx: {:.2}, y: {:.2}",
            ctx.series_label, point.x, point.y
        ))
    }

    /// Build the PlotWidget; validates series and duplicate labels via PlotWidget::add_series.
    pub fn build(self) -> Result<PlotWidget, SeriesError> {
        if let (Some((x_min, x_max)), Some((y_min, y_max))) = (self.x_lim, self.y_lim)
            && (x_min >= x_max || y_min >= y_max)
        {
            return Err(SeriesError::InvalidAxisLimits);
        }
        let mut w = PlotWidget::new();
        if self.disable_controls_help {
            w.controls_help_enabled = false;
            w.controls_overlay_open = false;
        }
        if self.disable_legend {
            w.legend_enabled = false;
        }

        if let Some(enabled) = self.autoscale_on_updates {
            w.autoscale_on_updates(enabled);
        }
        if let Some(r) = self.hover_radius_px {
            w.hover_radius_px(r);
        }
        if let Some(x) = self.x_label {
            w.set_x_axis_label(x);
        }
        if let Some(y) = self.y_label {
            w.set_y_axis_label(y);
        }
        if let Some((min, max)) = self.x_lim {
            w.set_x_lim(min, max);
        }
        if let Some((min, max)) = self.y_lim {
            w.set_y_lim(min, max);
        }
        if let Some(c) = self.cursor_overlay {
            w.set_cursor_overlay(c);
        }
        if let Some(p) = self.pick_highlight_provider {
            w.set_pick_highlight_provider(p);
        }
        if let Some(p) = self.hover_highlight_provider {
            w.set_hover_highlight_provider(p);
        } else {
            w.set_hover_highlight_provider(Arc::new(Self::default_hover_highlight_provider));
        }
        if let Some(p) = self.cursor_provider {
            w.set_cursor_provider(p);
        }
        if let Some(enabled) = self.crosshairs {
            w.set_crosshairs(enabled);
        }
        if let Some(link) = self.x_axis_link {
            w.set_x_axis_link(link);
        }
        if let Some(link) = self.y_axis_link {
            w.set_y_axis_link(link);
        }
        if let Some(formatter) = self.x_tick_formatter {
            w.set_x_axis_formatter(formatter);
        }
        if let Some(formatter) = self.y_tick_formatter {
            w.set_y_axis_formatter(formatter);
        }
        if self.enable_x_tick_labels == Some(false) {
            w.x_axis_formatter = None;
        }
        if self.enable_y_tick_labels == Some(false) {
            w.y_axis_formatter = None;
        }
        if let Some(producer) = self.x_tick_producer {
            w.set_x_tick_producer(producer);
        }
        if let Some(producer) = self.y_tick_producer {
            w.set_y_tick_producer(producer);
        }
        if let Some(size) = self.tick_label_size {
            w.tick_label_size = size;
        }
        if let Some(size) = self.axis_label_size {
            w.axis_label_size = size;
        }
        if let Some(aspect) = self.data_aspect {
            w.set_data_aspect(aspect);
        }
        for s in self.series {
            w.add_series(s)?;
        }
        for vline in self.vlines {
            w.add_vline(vline);
        }
        for hline in self.hlines {
            w.add_hline(hline);
        }

        Ok(w)
    }
}
