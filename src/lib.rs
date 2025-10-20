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
