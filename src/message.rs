#[derive(Debug, Clone)]
pub enum PlotUiMessage {
    ToggleLegend,
    ToggleSeriesVisibility(String),
    RenderUpdate(PlotRenderUpdate),
}

/// Context passed to a tooltip formatting callback.
#[derive(Debug, Clone)]
pub struct TooltipContext {
    /// Label of the series, if any (empty string means none)
    pub series_label: String,
    /// Index within the series [0..len)
    pub point_index: usize,
    /// Data-space coordinates
    pub x: f64,
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
pub struct PlotRenderUpdate {
    pub clear_tooltip: bool,
    pub tooltip_ui: Option<TooltipUiPayload>,
    pub clear_cursor_position: bool,
    pub cursor_position_ui: Option<CursorPositionUiPayload>,
}
