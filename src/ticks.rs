use std::sync::Arc;

use crate::grid::TickWeight;

/// A tick with an assigned screen position.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedTick {
    /// Screen position (x for vertical ticks, y for horizontal ticks)
    pub screen_pos: f32,
    /// The tick itself.
    pub tick: Tick,
}

/// A position along an axis where a grid line and tick label is placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    /// The value at this tick in world coordinates
    pub value: f64,

    /// The step size between ticks
    pub step_size: f64,

    /// The visual weight of the grid line at this tick
    pub line_type: TickWeight,
}

impl Tick {
    /// Create a new tick.
    pub fn new(value: f64, step_size: f64, line_type: TickWeight) -> Self {
        Self {
            value,
            step_size,
            line_type,
        }
    }
}

/// A function which formats tick values into strings for display on the axis.
pub type TickFormatter = Arc<dyn Fn(Tick) -> String + Send + Sync>;

/// A function which generates tick positions along an axis.
/// Takes a range (min, max) and returns a vector of ticks with their values and weights.
pub type TickProducer = Arc<dyn Fn(f64, f64) -> Vec<Tick> + Send + Sync>;

/// A default formatter that displays values with reasonable precision.
pub fn default_formatter(mark: Tick) -> String {
    let log_step = mark.step_size.log10();
    if log_step >= 0.0 {
        format!("{:.0}", mark.value)
    } else {
        let decimal_places = (-log_step).ceil() as usize;
        format!("{:.*}", decimal_places, mark.value)
    }
}

/// A simple formatter for logarithmic ticks with an arbitrary base.
///
/// Expects positive `tick.value` and renders labels as `b^n`, where `b` is the provided base.
pub fn log_formatter(mark: Tick, base: f64) -> String {
    if !mark.value.is_finite() || mark.value <= 0.0 {
        return String::new();
    }
    let exp = mark.value.log(base).round() as i32;

    if base == std::f64::consts::E {
        format!("e^{exp}") // Seems like a ~natural~ special case.
    } else {
        format!("{base}^{:.1}", exp)
    }
}

/// A default tick producer that generates tick positions with appropriate spacing.
pub fn default_tick_producer(min: f64, max: f64) -> Vec<Tick> {
    const GRID_TARGET_LINES: f64 = 20.0;
    const GRID_MAJOR_INTERVAL: i64 = 10;
    const GRID_MINOR_INTERVAL: i64 = 5;

    let span = max - min;
    if !span.is_finite() || span <= 0.0 {
        return Vec::new();
    }

    let step = nice_step(span / GRID_TARGET_LINES);
    let start = (min / step).ceil() * step;

    let mut ticks = Vec::new();
    let mut value = start;

    while value <= max {
        // Calculate the index based on the value's position relative to zero
        // This ensures that the same value always gets the same weight
        let idx = (value / step).round() as i64;

        let weight = if idx % GRID_MAJOR_INTERVAL == 0 {
            TickWeight::Major
        } else if idx % GRID_MINOR_INTERVAL == 0 {
            TickWeight::Minor
        } else {
            TickWeight::SubMinor
        };

        ticks.push(Tick::new(value, step, weight));

        value += step;
    }

    ticks
}

/// A simple powers-only base-10 tick producer.
///
/// Inputs are raw data-space bounds and must be positive.
pub fn log_tick_producer(base: f64, min: f64, max: f64) -> Vec<Tick> {
    let mut lo = min.min(max);
    let hi = min.max(max);
    if !lo.is_finite() || !hi.is_finite() || hi <= 0.0 {
        return Vec::new();
    }
    lo = lo.max(f64::MIN_POSITIVE);
    if lo > hi {
        return Vec::new();
    }

    let start_exp = lo.log(base).ceil() as i32;
    let end_exp = hi.log(base).floor() as i32;
    if start_exp > end_exp {
        return Vec::new();
    }

    let mut out = Vec::with_capacity((end_exp - start_exp + 1) as usize);
    for exp in start_exp..=end_exp {
        let value = base.powi(exp);
        if value.is_finite() {
            out.push(Tick::new(value, base, TickWeight::Major));
        }
    }
    out
}

/// Calculate a "nice" step size for grid lines based on the desired number of divisions.
/// Returns a value that is a multiple of 1, 2, 5, or 10 times a power of 10.
pub fn nice_step(raw: f64) -> f64 {
    const NICE_STEP_BASES: [f64; 4] = [1.0, 2.0, 5.0, 10.0];
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10.0_f64.powf(exp);
    for &m in &NICE_STEP_BASES {
        if raw <= m * base {
            return m * base;
        }
    }
    base * 10.0
}
