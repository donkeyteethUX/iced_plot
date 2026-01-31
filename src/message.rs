use iced::Rectangle;

use crate::{camera::Camera, series::ShapeId, ticks::PositionedTick};

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

impl PlotUiMessage {
    pub fn get_hover_pick_event(&self) -> Option<HoverPickEvent> {
        if let PlotUiMessage::RenderUpdate(update) = self {
            update.hover_pick
        } else {
            None
        }
    }
}

/// Context passed to a tooltip formatting callback.
///
/// Contains information about the point being hovered over.
#[derive(Debug, Clone, Copy)]
pub struct TooltipContext<'a> {
    /// ID of the series
    pub series_id: ShapeId,
    /// Label of the series, if any (empty string means none)
    pub series_label: &'a str,
    /// Index within the series [0..len)
    pub point_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipUiPayload {
    pub screen_x: f32,
    pub screen_y: f32,
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
    pub hover_pick: Option<HoverPickEvent>,
    pub clear_cursor_position: bool,
    pub cursor_position_ui: Option<CursorPositionUiPayload>,
    pub x_ticks: Option<Vec<PositionedTick>>,
    pub y_ticks: Option<Vec<PositionedTick>>,
    /// Internal: Camera and bounds for coordinate conversion (only used internally, not part of public API)
    pub(crate) camera_bounds: Option<(Camera, Rectangle)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointId {
    pub series_id: ShapeId,
    pub point_index: usize,
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum HoverPickEvent {
    Hover(PointId),
    ClearHover,
    Pick(PointId),
    ClearPick,
}
