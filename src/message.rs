use iced::Rectangle;

use crate::{camera::Camera, series::ShapeId, ticks::PositionedTick};

/// Messages sent by the plot widget to the application.
///
/// These messages are generated in response to user interactions with the plot.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum PlotUiMessage {
    /// Toggle the legend visibility.
    ToggleLegend,
    /// Toggle the in-canvas controls/help overlay.
    ToggleControlsOverlay,
    /// Toggle visibility of a series or reference line by label.
    ToggleSeriesVisibility(ShapeId),
    /// Internal render update message.
    RenderUpdate(PlotRenderUpdate),
}

impl PlotUiMessage {
    /// Get the hover or pick event from the render update.
    /// If the plot widget is not in hover or pick mode, this will return None.
    pub fn get_hover_pick_event(&self) -> Option<HoverPickEvent> {
        if let PlotUiMessage::RenderUpdate(update) = self {
            update.hover_pick
        } else {
            None
        }
    }

    /// Get the drag event from the render update.
    pub fn get_drag_event(&self) -> Option<DragEvent> {
        if let PlotUiMessage::RenderUpdate(update) = self {
            update.drag_event
        } else {
            None
        }
    }
}

/// Context passed to hover/pick highlight callbacks.
///
/// Contains information identifying the point being highlighted.
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
    /// screen coordinates of the tooltip.
    /// `screen_xy = None` means the tooltip is outside of the plot widget
    pub screen_xy: Option<[f32; 2]>,
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
    pub drag_event: Option<DragEvent>,
    pub clear_cursor_position: bool,
    pub cursor_position_ui: Option<CursorPositionUiPayload>,
    pub x_ticks: Option<Vec<PositionedTick>>,
    pub y_ticks: Option<Vec<PositionedTick>>,
    /// Internal: Camera and bounds for coordinate conversion (only used internally, not part of public API)
    pub(crate) camera_bounds: Option<Box<(Camera, Rectangle)>>,
}

/// Drag interaction event in data/world coordinates.
#[derive(Debug, Clone, Copy)]
pub enum DragEvent {
    /// A drag gesture started inside the plot.
    Start {
        /// Current cursor world/data coordinate.
        world: [f64; 2],
    },
    /// Cursor moved while drag is active.
    Update {
        /// Current cursor world/data coordinate.
        world: [f64; 2],
    },
    /// Active drag gesture ended.
    End {
        /// Current cursor world/data coordinate.
        world: [f64; 2],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Identifier for a point in a series.
pub struct PointId {
    /// ID of the series
    pub series_id: ShapeId,
    /// Index within the series [0..len)
    pub point_index: usize,
}

/// The hover or pick event.
#[derive(Debug, Clone, Copy)]
pub enum HoverPickEvent {
    /// Hover a point.
    Hover(PointId),
    /// Clear all hovered points.
    ClearHover,
    /// Pick a point.
    Pick(PointId),
    /// Clear all picked points.
    ClearPick,
}
