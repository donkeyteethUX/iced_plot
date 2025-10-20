//! A GPU-accelerated plotting widget for Iced.
//!
//! - Works with large datasets (up to millions of points)
//! - Retains GPU buffers between frames for fast redraws and picking
//! - Axes/labels, legends, reference lines, tooltips, crosshairs, axis linking, etc.
//!
//! Quick start:
//!
//! ```no_run
//! let series = Series::circles((0..100).map(|i| [i as f64, i as f64]).collect(), 2.0)
//!     .with_color(Color::from_rgb(0.2, 0.6, 1.0))
//!     .with_label("points");
//!
//! PlotWidgetBuilder::new()
//!     .with_x_label("x")
//!     .with_y_label("y")
//!     .add_series(series)
//!     .build()
//!     .unwrap()
//! ```
//!
//! See `examples/` for more.
pub mod axis_link;
pub mod message;
pub mod plot_widget;
pub mod plot_widget_builder;
pub mod point;
pub mod reference_lines;
pub mod series;

pub(crate) mod axes_labels;
pub(crate) mod camera;
pub(crate) mod grid;
pub(crate) mod legend;
pub(crate) mod picking;
pub(crate) mod plot_renderer;
pub(crate) mod plot_state;

// Iced re-exports.
pub use iced::Color;

// Re-exports of public types.
pub use axis_link::AxisLink;
pub use message::{PlotUiMessage, TooltipContext};
pub use plot_widget::PlotWidget;
pub use plot_widget_builder::PlotWidgetBuilder;
pub use point::{MarkerType, Point};
pub use reference_lines::{HLine, VLine};
pub use series::{LineStyle, MarkerStyle, Series};
