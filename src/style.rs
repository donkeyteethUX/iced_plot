use std::sync::Arc;

use iced::{Color, Theme, border, widget::container};

/// Produces a [`PlotStyle`] from a given application theme.
pub(crate) type StyleFn = Arc<dyn Fn(&Theme) -> PlotStyle + Send + Sync>;

/// Configures the appearance of a [`PlotWidget`](crate::PlotWidget).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlotStyle {
    /// Style for the outer frame surrounding labels, ticks, overlays, and plot area.
    pub frame: container::Style,

    /// Style for the inner plot area behind the GPU-rendered canvas.
    pub plot_area: container::Style,

    /// Style for the legend panel.
    pub legend: container::Style,

    /// Style for the controls/help panel.
    pub controls_panel: container::Style,

    /// Style for the cursor-position overlay bubble.
    pub cursor_overlay: container::Style,

    /// Style for point tooltip bubbles.
    pub tooltip: container::Style,

    /// Style for grid lines rendered inside the plot area.
    pub grid: GridStyle,
}

/// Configures the appearance of grid lines inside the plot area.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GridStyle {
    /// Color of major grid lines.
    pub major: Color,

    /// Color of minor grid lines.
    pub minor: Color,

    /// Color of sub-minor grid lines.
    pub sub_minor: Color,
}

/// Returns the default plot style derived from the current application theme.
pub fn default_style(theme: &Theme) -> PlotStyle {
    let palette = theme.extended_palette();
    let grid_base = palette.background.base.text;

    PlotStyle {
        frame: container::background(theme.palette().background),
        plot_area: container::background(theme.palette().background),
        legend: container::Style {
            background: Some(palette.background.weakest.color.into()),
            text_color: Some(palette.background.weakest.text),
            border: iced::Border {
                width: 1.0,
                radius: 5.0.into(),
                color: palette.background.weak.color,
            },
            ..container::Style::default()
        },
        controls_panel: container::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: Some(palette.background.weak.text),
            border: border::rounded(2),
            ..container::Style::default()
        },
        cursor_overlay: container::Style {
            background: Some(palette.background.weak.color.into()),
            text_color: Some(palette.background.weak.text),
            border: border::rounded(2),
            ..container::Style::default()
        },
        tooltip: container::Style {
            background: Some(palette.background.weak.color.scale_alpha(0.7).into()),
            text_color: Some(palette.background.weak.text.scale_alpha(0.7)),
            border: border::rounded(2),
            ..container::Style::default()
        },
        grid: GridStyle {
            major: with_alpha(grid_base, 0.45),
            minor: with_alpha(grid_base, 0.28),
            sub_minor: with_alpha(grid_base, 0.10),
        },
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}
