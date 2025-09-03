#[derive(Debug, Clone)]
pub enum PlotUiMessage {
    ToggleLegend,
    ToggleSeriesVisibility(String),
    // /// UI payload to render a tooltip overlay. None clears it.
    // TooltipUiChanged(Option<TooltipUiPayload>),
    // /// UI payload to render a small cursor-position overlay in the plot.
    // /// None clears it.
    // CursorPositionUiChanged(Option<CursorPositionUiPayload>),
    RenderUpdate(PlotRenderUpdate),
}

/// Context passed to a tooltip formatting callback.
/// Public so applications can implement a custom formatter.
#[derive(Debug, Clone)]
pub struct TooltipContext {
    /// Label of the series, if any (empty string means none)
    pub series_label: String,
    /// Index within the series [0..len)
    pub point_index: usize,
    /// Data-space coordinates
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub struct TooltipUiPayload {
    pub x: f32, // screen-space logical px relative to shader area
    pub y: f32,
    pub text: String,
}

/// Payload for the small cursor-position overlay shown in the corner.
#[derive(Debug, Clone)]
pub struct CursorPositionUiPayload {
    /// World/data-space coordinates for the cursor
    pub x: f32,
    pub y: f32,
    /// Formatted text to render
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PlotRenderUpdate {
    pub clear_tooltip: bool,
    pub tooltip_ui: Option<TooltipUiPayload>,
    pub clear_cursor_position: bool,
    pub cursor_position_ui: Option<CursorPositionUiPayload>,
}
