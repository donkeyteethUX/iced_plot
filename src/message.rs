use iced::{Rectangle, keyboard, mouse};

use crate::{camera::Camera, series::ShapeId, ticks::PositionedTick};

/// Messages sent by the plot widget to the application.
///
/// These messages are generated in response to user interactions with the plot.
#[derive(Debug, Clone)]
pub enum PlotUiMessage {
    /// Toggle the legend visibility.
    ToggleLegend,
    /// Toggle the in-canvas controls/help overlay.
    ToggleControlsOverlay,
    /// Toggle visibility of a series or reference line by label.
    ToggleSeriesVisibility(ShapeId),
    /// Plot event payload (input + render updates).
    Event(PlotEvent),
    /// Apply a plot command (used to forward default interactions in override mode).
    Command(PlotCommand),
}

impl PlotUiMessage {
    /// Get the hover or pick event from the render update.
    /// If the plot widget is not in hover or pick mode, this will return None.
    pub fn get_hover_pick_event(&self) -> Option<HoverPickEvent> {
        if let PlotUiMessage::Event(event) = self {
            event.render.as_ref().and_then(|update| update.hover_pick)
        } else {
            None
        }
    }
}

/// Input handling policy for the plot widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPolicy {
    /// Use the built-in pan/zoom/hover/pick interactions.
    Default,
    /// Emit input events and suppress built-in interactions.
    Override,
}

/// A plot event payload containing input and render updates.
#[derive(Debug, Clone)]
pub struct PlotEvent {
    pub input: Option<PlotInputEvent>,
    pub render: Option<PlotRenderUpdate>,
}

/// Pointer input event emitted by the plot in override mode.
#[derive(Debug, Clone)]
pub enum PlotInputEvent {
    CursorMoved(PlotPointerEvent),
    CursorEntered(PlotPointerEvent),
    CursorLeft(PlotPointerEvent),
    ButtonPressed {
        button: mouse::Button,
        pointer: PlotPointerEvent,
    },
    ButtonReleased {
        button: mouse::Button,
        pointer: PlotPointerEvent,
    },
    WheelScrolled {
        delta: mouse::ScrollDelta,
        pointer: PlotPointerEvent,
    },
}

/// Shared pointer event data for plot input.
#[derive(Debug, Clone, Copy)]
pub struct PlotPointerEvent {
    /// Cursor position in window coordinates.
    pub screen: [f32; 2],
    /// Cursor position in plot-local coordinates (relative to plot bounds).
    pub local: [f32; 2],
    /// Whether the cursor is inside the plot bounds.
    pub inside: bool,
    /// Cursor position in world/data coordinates (if inside bounds).
    pub world: Option<[f64; 2]>,
    /// Current keyboard modifiers.
    pub modifiers: keyboard::Modifiers,
}

/// Commands that can be applied to the plot (for custom input handling).
#[derive(Debug, Clone)]
pub enum PlotCommand {
    /// Apply a pointer event with explicit interaction enable/disable.
    ApplyInputEvent {
        input: PlotInputEvent,
        interactions_enabled: bool,
    },
    /// Apply the built-in default input behavior for a synthetic pointer event.
    ApplyDefaultMouseEvent(PlotInputEvent),
    /// Pan the camera by the given world-space delta.
    PanByWorld { delta: [f64; 2] },
    /// Zoom the camera by a factor around an optional world-space anchor point.
    ZoomBy {
        factor: f64,
        anchor_world: Option<[f64; 2]>,
    },
    /// Zoom the camera to the given world-space rectangle.
    ZoomToWorldRect {
        min: [f64; 2],
        max: [f64; 2],
        padding_frac: f64,
    },
    /// Autoscale the camera to fit data.
    Autoscale { update_axis_links: bool },
    /// Clear hovered points.
    ClearHover,
    /// Clear picked points.
    ClearPick,
    /// Request a hover pick at the current cursor position.
    RequestHover,
    /// Request a pick at the current cursor position.
    RequestPick,
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

/// Coordinate snapshot for screen/world conversions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotCoordinateSnapshot {
    pub camera_position: [f64; 2],
    pub camera_half_extents: [f64; 2],
    pub camera_render_offset: [f64; 2],
    pub bounds: Rectangle,
}

impl PlotCoordinateSnapshot {
    /// Convert a plot-local screen position to world coordinates.
    pub fn screen_to_world(&self, screen: [f32; 2]) -> [f64; 2] {
        let camera = Camera::from_parts(
            self.camera_position,
            self.camera_half_extents,
            self.camera_render_offset,
        );
        camera.screen_to_world_from_bounds(screen, self.bounds)
    }

    /// Convert world coordinates to plot-local screen coordinates.
    pub fn world_to_screen(&self, world: [f64; 2]) -> Option<[f32; 2]> {
        let camera = Camera::from_parts(
            self.camera_position,
            self.camera_half_extents,
            self.camera_render_offset,
        );
        camera.world_to_screen_with_bounds(world, self.bounds)
    }
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct PlotRenderUpdate {
    pub hover_pick: Option<HoverPickEvent>,
    pub clear_cursor_position: bool,
    pub cursor_position_ui: Option<CursorPositionUiPayload>,
    pub x_ticks: Option<Vec<PositionedTick>>,
    pub y_ticks: Option<Vec<PositionedTick>>,
    /// Camera and bounds for coordinate conversion.
    pub camera_bounds: Option<PlotCoordinateSnapshot>,
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
