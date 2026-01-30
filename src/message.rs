use crate::{series::ShapeId, ticks::PositionedTick};

#[derive(Debug, Clone)]
/// Messages sent by the plot widget to the application.
///
/// These messages are generated in response to user interactions with the plot.
pub enum PlotUiMessage {
    /// Toggle the legend visibility.
    ToggleLegend,
    /// Toggle visibility of a series or reference line by label.
    ToggleSeriesVisibility(ShapeId),
    /// Internal render update message.
    RenderUpdate(PlotRenderUpdate),
}

/// Context passed to a tooltip formatting callback.
///
/// Contains information about the point being hovered over.
#[derive(Debug, Clone)]
pub struct TooltipContext {
    /// Label of the series, if any (empty string means none)
    pub series_label: String,
    /// Index within the series [0..len)
    pub point_index: usize,
    /// Data-space coordinates
    pub x: f64,
    /// Data-space coordinates
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct TooltipUiPayload {
    pub x: f32,
    pub y: f32,
    pub text: String,
}

/// Payload for the small cursor-position overlay shown in the corner.
#[derive(Debug, Clone)]
pub struct CursorPositionUiPayload {
    /// World/data-space coordinates for the cursor
    pub x: f64,
    pub y: f64,
    /// Formatted text to render
    pub text: String,
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct PlotRenderUpdate {
    pub clear_tooltip: bool,
    pub tooltip_ui: Option<TooltipUiPayload>,
    pub clear_cursor_position: bool,
    pub cursor_position_ui: Option<CursorPositionUiPayload>,
    pub x_ticks: Option<Vec<PositionedTick>>,
    pub y_ticks: Option<Vec<PositionedTick>>,
}
