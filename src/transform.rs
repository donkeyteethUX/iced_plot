//! Coordinate transforms for series and reference lines.
//!
//! A [`Transform`] describes how one x or y value should be interpreted before
//! it is drawn. Most values use data coordinates: the raw value is optionally
//! converted, then the plot axis scale is applied. `Transform::axes()` instead
//! uses normalized plot coordinates, where `0.0` is the low edge of the plot and
//! `1.0` is the high edge.

use crate::axis_scale::AxisScale;

/// The source coordinate system consumed by a [`Transform`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordinateSystem {
    /// Data coordinates: convert the raw value, then apply the axis scale.
    #[default]
    Data,
    /// Axes coordinates, where `0.0` is the low edge of the plot and `1.0` is the high edge.
    Axes,
}

#[derive(Debug, Clone, PartialEq)]
enum TransformOperation {
    Identity,
    Affine { scale: f64, translate: f64 },
    Log { base: f64 },
    Exp { base: f64 },
    Then(Box<TransformOperation>, Box<TransformOperation>),
}

impl Default for TransformOperation {
    fn default() -> Self {
        Self::Identity
    }
}

impl TransformOperation {
    fn then(self, next: TransformOperation) -> Self {
        match (self, next) {
            (Self::Identity, next) => next,
            (this, Self::Identity) => this,
            (this, next) => Self::Then(Box::new(this), Box::new(next)),
        }
    }

    fn transform_value(&self, value: f64) -> Option<f64> {
        match self {
            Self::Identity => value.is_finite().then_some(value),
            Self::Affine { scale, translate } => {
                if !(value.is_finite() && scale.is_finite() && translate.is_finite()) {
                    return None;
                }
                let out = value.mul_add(*scale, *translate);
                out.is_finite().then_some(out)
            }
            Self::Log { base } => {
                if !(value.is_finite() && value > 0.0 && valid_log_base(*base)) {
                    return None;
                }
                let out = value.log(*base);
                out.is_finite().then_some(out)
            }
            Self::Exp { base } => {
                if !(value.is_finite() && valid_log_base(*base)) {
                    return None;
                }
                let out = base.powf(value);
                (out.is_finite() && out > 0.0).then_some(out)
            }
            Self::Then(first, second) => second.transform_value(first.transform_value(value)?),
        }
    }

    fn inverted(&self) -> Option<Self> {
        match self {
            Self::Identity => Some(Self::Identity),
            Self::Affine { scale, translate } => {
                if !scale.is_finite() || scale.abs() <= f64::EPSILON || !translate.is_finite() {
                    return None;
                }
                Some(Self::Affine {
                    scale: 1.0 / scale,
                    translate: -translate / scale,
                })
            }
            Self::Log { base } => Some(Self::Exp { base: *base }),
            Self::Exp { base } => Some(Self::Log { base: *base }),
            Self::Then(first, second) => Some(second.inverted()?.then(first.inverted()?)),
        }
    }
}

/// A one-dimensional coordinate transform for a series or reference line.
///
/// For normal data coordinates, the transform is a value converter that runs
/// before the plot's axis scale. For example, `Transform::affine(2.0, -1.0)`
/// draws `raw * 2 - 1`, then lets the x/y axis scale map that value into plot
/// space.
///
/// `Transform::axes()` is different: it treats the input as a normalized
/// position inside the plot area. `0.4` means 40% from the low edge, so the
/// position stays fixed while the user pans or zooms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transform {
    coordinate_system: CoordinateSystem,
    operation: TransformOperation,
}

impl From<AxisScale> for Transform {
    fn from(scale: AxisScale) -> Self {
        match scale {
            AxisScale::Linear => Self::identity(),
            AxisScale::Log { base } => Self::log(base),
        }
    }
}

impl Transform {
    /// Leave a data value unchanged before the axis scale is applied.
    pub const fn identity() -> Self {
        Self {
            coordinate_system: CoordinateSystem::Data,
            operation: TransformOperation::Identity,
        }
    }

    /// Interpret values as normalized positions inside the plot area.
    ///
    /// `0.0` is the low edge of the axis and `1.0` is the high edge. A point
    /// using `PositionTransform::axes()` therefore remains fixed in the plot
    /// area during pan and zoom.
    pub const fn axes() -> Self {
        Self {
            coordinate_system: CoordinateSystem::Axes,
            operation: TransformOperation::Identity,
        }
    }

    /// Convert a data value with `value * scale + translate` before axis scaling.
    pub const fn affine(scale: f64, translate: f64) -> Self {
        Self {
            coordinate_system: CoordinateSystem::Data,
            operation: TransformOperation::Affine { scale, translate },
        }
    }

    /// Convert a data value with `log_base(value)` before axis scaling.
    pub const fn log(base: f64) -> Self {
        Self {
            coordinate_system: CoordinateSystem::Data,
            operation: TransformOperation::Log { base },
        }
    }

    /// Convert a data value with `base.powf(value)` before axis scaling.
    pub const fn exp(base: f64) -> Self {
        Self {
            coordinate_system: CoordinateSystem::Data,
            operation: TransformOperation::Exp { base },
        }
    }

    /// Return the transform's source coordinate system.
    pub fn coordinate_system(&self) -> CoordinateSystem {
        self.coordinate_system
    }

    /// Run this value conversion, then run another one.
    ///
    /// The returned transform applies `self` first, then `next`. The source
    /// coordinate system of `self` is retained.
    pub fn then(self, next: Transform) -> Self {
        Self {
            coordinate_system: self.coordinate_system,
            operation: self.operation.then(next.operation),
        }
    }

    /// Convert a raw value using only this transform's operation.
    pub fn transform_value(&self, value: f64) -> Option<f64> {
        self.operation.transform_value(value)
    }

    /// Create the inverse operation, if it is representable.
    pub fn inverted(&self) -> Option<Self> {
        Some(Self {
            coordinate_system: self.coordinate_system,
            operation: self.operation.inverted()?,
        })
    }

    /// Convert a data-coordinate value into plot-space.
    ///
    /// This applies the transform first, then the axis scale. `Transform::axes()`
    /// needs the current axis range, which is only available internally during
    /// rendering.
    pub fn transform_data(&self, pos: f64, axis_scale: AxisScale) -> Option<f64> {
        data_value_to_plot(pos, axis_scale, Some(self))
    }

    /// Convert a value into a normalized `[0, 1]` position along an axis.
    ///
    /// Data-coordinate transforms use `axis_range` as raw data coordinates.
    /// Axes-coordinate transforms already store normalized positions.
    pub fn transform_position(
        &self,
        pos: f64,
        axis_scale: AxisScale,
        axis_range: [f64; 2],
    ) -> Option<f64> {
        if self.coordinate_system == CoordinateSystem::Axes {
            return self.transform_value(pos);
        }

        let pos = self.transform_data(pos, axis_scale)?;
        let min = self.transform_data(axis_range[0], axis_scale)?;
        let max = self.transform_data(axis_range[1], axis_scale)?;
        let span = max - min;
        (span.is_finite() && span.abs() > f64::EPSILON).then_some((pos - min) / span)
    }

    pub(crate) fn uses_axes_coordinates(&self) -> bool {
        self.coordinate_system == CoordinateSystem::Axes
    }
}

/// Separate x and y coordinate transforms for a two-dimensional point.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PositionTransform {
    /// How to interpret or convert x values. `None` means normal data coordinates.
    pub x: Option<Transform>,
    /// How to interpret or convert y values. `None` means normal data coordinates.
    pub y: Option<Transform>,
}

impl PositionTransform {
    /// Create a new x/y transform pair.
    pub fn new(x: Option<Transform>, y: Option<Transform>) -> Self {
        Self { x, y }
    }

    /// Use normal data coordinates for both axes.
    pub const fn identity() -> Self {
        Self { x: None, y: None }
    }

    /// Interpret both axes as normalized plot positions.
    pub const fn axes() -> Self {
        Self {
            x: Some(Transform::axes()),
            y: Some(Transform::axes()),
        }
    }

    /// Convert a raw point into plot-space coordinates.
    pub fn transform_point(
        &self,
        point: [f64; 2],
        x_axis_scale: AxisScale,
        y_axis_scale: AxisScale,
    ) -> Option<[f64; 2]> {
        data_point_to_plot_with_transform(point, x_axis_scale, y_axis_scale, self, None)
    }

    pub(crate) fn uses_axes_coordinates(&self) -> bool {
        self.x
            .as_ref()
            .is_some_and(Transform::uses_axes_coordinates)
            || self
                .y
                .as_ref()
                .is_some_and(Transform::uses_axes_coordinates)
    }
}

pub(crate) fn data_value_to_plot(
    value: f64,
    axis_scale: AxisScale,
    transform: Option<&Transform>,
) -> Option<f64> {
    data_value_to_plot_with_axis_range(value, axis_scale, transform, None)
}

pub(crate) fn data_value_to_plot_with_axis_range(
    value: f64,
    axis_scale: AxisScale,
    transform: Option<&Transform>,
    axis_range: Option<[f64; 2]>,
) -> Option<f64> {
    let Some(transform) = transform else {
        return Transform::from(axis_scale).transform_value(value);
    };

    let value = transform.transform_value(value)?;
    match transform.coordinate_system {
        CoordinateSystem::Data => Transform::from(axis_scale).transform_value(value),
        CoordinateSystem::Axes => {
            let [min, max] = axis_range?;
            if !(min.is_finite() && max.is_finite()) {
                return None;
            }
            Some(min + value * (max - min))
        }
    }
}

pub(crate) fn plot_value_to_data(value: f64, axis_scale: AxisScale) -> Option<f64> {
    Transform::from(axis_scale)
        .inverted()?
        .transform_value(value)
}

pub(crate) fn data_point_to_plot_with_transform(
    point: [f64; 2],
    x_scale: AxisScale,
    y_scale: AxisScale,
    transform: &PositionTransform,
    axis_ranges: Option<([f64; 2], [f64; 2])>,
) -> Option<[f64; 2]> {
    Some([
        data_value_to_plot_with_axis_range(
            point[0],
            x_scale,
            transform.x.as_ref(),
            axis_ranges.map(|ranges| ranges.0),
        )?,
        data_value_to_plot_with_axis_range(
            point[1],
            y_scale,
            transform.y.as_ref(),
            axis_ranges.map(|ranges| ranges.1),
        )?,
    ])
}

pub(crate) fn plot_point_to_data(
    point: [f64; 2],
    x_scale: AxisScale,
    y_scale: AxisScale,
) -> Option<[f64; 2]> {
    Some([
        plot_value_to_data(point[0], x_scale)?,
        plot_value_to_data(point[1], y_scale)?,
    ])
}

fn valid_log_base(base: f64) -> bool {
    base.is_finite() && base > 0.0 && (base - 1.0).abs() > f64::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_transform_normalizes_axis_position() {
        let transform = Transform::affine(2.0, 10.0);
        assert_eq!(
            transform.transform_position(5.0, AxisScale::Linear, [0.0, 10.0]),
            Some(0.5)
        );
    }

    #[test]
    fn axis_scale_is_applied_after_data_transform() {
        let transform = Transform::affine(10.0, 0.0);
        assert_eq!(
            transform.transform_data(10.0, AxisScale::Log { base: 10.0 }),
            Some(2.0)
        );
    }

    #[test]
    fn composite_transform_inverts_in_reverse_order() {
        let transform = Transform::affine(2.0, 10.0).then(Transform::log(10.0));
        let inverted = transform
            .inverted()
            .expect("transform should be invertible");
        let value = transform.transform_value(45.0).unwrap();
        let round_trip = inverted.transform_value(value).unwrap();
        assert!((round_trip - 45.0).abs() < 1e-12);
    }

    #[test]
    fn axes_transform_maps_normalized_value_into_axis_range() {
        let value = data_value_to_plot_with_axis_range(
            0.4,
            AxisScale::Log { base: 10.0 },
            Some(&Transform::axes()),
            Some([10.0, 20.0]),
        );
        assert_eq!(value, Some(14.0));
    }
}
