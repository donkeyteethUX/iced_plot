/// Axis scaling mode.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AxisScale {
    /// Linear axis: displayed value is the raw data value.
    #[default]
    Linear,

    /// Logarithmic axis: displayed value is `log_{base}(raw)`.
    ///
    /// Only positive values are representable on this axis.
    Log {
        /// The base of the logarithm.
        base: f64,
    },
}

impl AxisScale {
    /// Transform raw data value into plot-space value.
    pub(crate) fn data_to_plot(self, value: f64) -> Option<f64> {
        crate::transform::data_value_to_plot(value, self, None)
    }

    /// Transform plot-space value into raw data value.
    pub(crate) fn plot_to_data(self, value: f64) -> Option<f64> {
        crate::transform::plot_value_to_data(value, self)
    }
}

pub(crate) fn plot_point_to_data(
    point: [f64; 2],
    x_scale: AxisScale,
    y_scale: AxisScale,
) -> Option<[f64; 2]> {
    crate::transform::plot_point_to_data(point, x_scale, y_scale)
}
