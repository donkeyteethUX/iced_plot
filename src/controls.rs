//! Controls for user interaction with the plot.

/// Configures user interaction behavior for [`crate::PlotWidget`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlotControls {
    /// Controls how panning is performed.
    pub pan: PanControls,

    /// Controls how zooming is performed.
    pub zoom: ZoomControls,

    /// Controls how points are picked and cleared.
    pub pick: PickControls,

    /// Enables point highlighting while hovering.
    pub highlight_on_hover: bool,

    /// Shows the in-canvas controls/help UI (`?` button).
    pub show_controls_help: bool,
}

/// Configures panning interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanControls {
    /// Enables panning using the mouse wheel or trackpad scroll gesture.
    pub scroll_to_pan: bool,

    /// Enables panning by dragging with the left mouse button.
    pub drag_to_pan: bool,
}

/// Configures zoom interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomControls {
    /// Enables box zoom via right-button drag.
    pub box_zoom: bool,

    /// Enables zooming at cursor while Ctrl is held during scroll.
    pub scroll_with_ctrl: bool,

    /// Enables double-click reset/autoscale behavior.
    pub double_click_autoscale: bool,
}

/// Configures pick interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickControls {
    /// Enables picking by left-clicking a highlighted point.
    pub click_to_pick: bool,

    /// Enables clearing picked points with the Escape key.
    pub clear_on_escape: bool,
}

// In keeping with our batteries-included philosophy, most everything is enabled by default.

impl Default for PlotControls {
    fn default() -> Self {
        Self {
            pan: PanControls::default(),
            zoom: ZoomControls::default(),
            pick: PickControls::default(),
            highlight_on_hover: true,
            show_controls_help: true,
        }
    }
}

impl Default for ZoomControls {
    fn default() -> Self {
        Self {
            box_zoom: true,
            scroll_with_ctrl: true,
            double_click_autoscale: true,
        }
    }
}

impl Default for PanControls {
    fn default() -> Self {
        Self {
            scroll_to_pan: true,
            drag_to_pan: true,
        }
    }
}

impl Default for PickControls {
    fn default() -> Self {
        Self {
            click_to_pick: true,
            clear_on_escape: true,
        }
    }
}
