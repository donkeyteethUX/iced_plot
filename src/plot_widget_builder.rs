use std::sync::Arc;

use crate::message::TooltipContext;
use crate::series::{Series, SeriesError};
use crate::widget::{CursorProvider, PlotWidget, TooltipProvider};

/// Builder for configuring and constructing a PlotWidget.
#[derive(Default)]
pub struct PlotWidgetBuilder {
    x_label: Option<String>,
    y_label: Option<String>,
    tooltips: Option<bool>,
    autoscale_on_updates: Option<bool>,
    hover_radius_px: Option<f32>,
    tooltip_provider: Option<TooltipProvider>,
    cursor_overlay: Option<bool>,
    cursor_provider: Option<CursorProvider>,
    x_lim: Option<(f32, f32)>,
    y_lim: Option<(f32, f32)>,
    series: Vec<Series>,
}

impl PlotWidgetBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_x_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.x_label = Some(l);
        }
        self
    }

    pub fn with_y_label(mut self, label: impl Into<String>) -> Self {
        let l = label.into();
        if !l.is_empty() {
            self.y_label = Some(l);
        }
        self
    }

    pub fn with_tooltips(mut self, enabled: bool) -> Self {
        self.tooltips = Some(enabled);
        self
    }

    pub fn with_autoscale_on_updates(mut self, enabled: bool) -> Self {
        self.autoscale_on_updates = Some(enabled);
        self
    }

    pub fn with_hover_radius_px(mut self, radius: f32) -> Self {
        self.hover_radius_px = Some(radius.max(0.0));
        self
    }

    /// Provide a custom tooltip text formatter. Passing `None` disables formatting.
    pub fn with_tooltip_provider<F>(mut self, provider: F) -> Self
    where
        F: Fn(&TooltipContext) -> String + Send + Sync + 'static,
    {
        self.tooltip_provider = Some(Arc::new(provider));
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
        F: Fn(f32, f32) -> String + Send + Sync + 'static,
    {
        self.cursor_provider = Some(Arc::new(provider));
        self
    }

    /// Set the x-axis limits (min, max) for the plot.
    /// If set, these will override autoscaling for the x-axis.
    pub fn with_x_lim(mut self, min: f32, max: f32) -> Self {
        self.x_lim = Some((min, max));
        self
    }

    /// Set the y-axis limits (min, max) for the plot.
    /// If set, these will override autoscaling for the y-axis.
    pub fn with_y_lim(mut self, min: f32, max: f32) -> Self {
        self.y_lim = Some((min, max));
        self
    }

    pub fn add_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Build the PlotWidget; validates series and duplicate labels via PlotWidget::add_series.
    pub fn build(self) -> Result<PlotWidget, SeriesError> {
        if let (Some((x_min, x_max)), Some((y_min, y_max))) = (self.x_lim, self.y_lim) {
            if x_min >= x_max || y_min >= y_max {
                return Err(SeriesError::InvalidAxisLimits);
            }
        }
        let mut w = PlotWidget::new();

        if let Some(enabled) = self.tooltips {
            w.tooltips(enabled);
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
        if let Some(p) = self.tooltip_provider {
            w.set_tooltip_provider(p.clone());
        }
        if let Some(c) = self.cursor_overlay {
            w.set_cursor_overlay(c);
        }
        if let Some(p) = self.cursor_provider {
            w.set_cursor_provider(p.clone());
        }

        for s in self.series {
            w.add_series(s)?;
        }

        Ok(w)
    }
}
