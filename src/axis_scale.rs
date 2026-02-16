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
        match self {
            Self::Linear => value.is_finite().then_some(value),
            Self::Log { base } => (value.is_finite() && value > 0.0)
                .then(|| value.log(base))
                .filter(|v| v.is_finite()),
        }
    }

    /// Transform plot-space value into raw data value.
    pub(crate) fn plot_to_data(self, value: f64) -> Option<f64> {
        match self {
            Self::Linear => value.is_finite().then_some(value),
            Self::Log { base } => {
                if !value.is_finite() {
                    return None;
                }
                let out = base.powf(value);
                (out.is_finite() && out > 0.0).then_some(out)
            }
        }
    }
}

pub(crate) fn data_point_to_plot(
    point: [f64; 2],
    x_scale: AxisScale,
    y_scale: AxisScale,
) -> Option<[f64; 2]> {
    Some([
        x_scale.data_to_plot(point[0])?,
        y_scale.data_to_plot(point[1])?,
    ])
}

pub(crate) fn plot_point_to_data(
    point: [f64; 2],
    x_scale: AxisScale,
    y_scale: AxisScale,
) -> Option<[f64; 2]> {
    Some([
        x_scale.plot_to_data(point[0])?,
        y_scale.plot_to_data(point[1])?,
    ])
}
