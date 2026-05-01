#[cfg(feature = "canvas")]
pub mod canvas;
mod shader;
pub(crate) use shader::{PlotRenderer, RenderParams};

use crate::{
    axis_scale::data_point_to_plot, plot_state::PlotState, plot_widget::HighlightPoint,
    series::Size,
};
use iced::Color;

/// Ways in which the widget can be rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlotRenderStrategy {
    /// Use the `wgpu` shader path.
    #[default]
    Shader,
    /// Use the `canvas` path for CPU renderers such as `tiny-skia`.
    #[cfg(feature = "canvas")]
    Canvas,
}

impl PlotRenderStrategy {
    /// From `iced::system::Information`'s `graphics_backend`.
    pub fn from_graphics_backend(graphics_backend: &str) -> Self {
        #[cfg(feature = "canvas")]
        {
            match graphics_backend.trim() {
                "tiny-skia" => Self::Canvas,
                _ => Self::Shader,
            }
        }
        #[cfg(not(feature = "canvas"))]
        {
            _ = graphics_backend;
            Self::Shader
        }
    }
    pub(crate) fn force_cpu_picking(self) -> bool {
        #[cfg(feature = "canvas")]
        {
            matches!(self, Self::Canvas)
        }
        #[cfg(not(feature = "canvas"))]
        {
            let _ = self;
            false
        }
    }
}

const SELECTION_FILL_RGBA: [f32; 4] = [0.2, 0.6, 1.0, 0.2];
const CROSSHAIR_RGBA: [f32; 4] = [0.5, 0.5, 0.5, 0.5];

fn color_to_rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn highlight_mask_color(color: Color) -> Color {
    if color.relative_luminance() > 0.9 {
        Color::from_rgba(0.0, 0.0, 0.0, 0.25)
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.25)
    }
}

fn highlight_mask_rgba(color: Color) -> [f32; 4] {
    color_to_rgba(highlight_mask_color(color))
}

fn highlight_marker_plot_position(
    highlight: &HighlightPoint,
    state: &PlotState,
) -> Option<[f64; 2]> {
    data_point_to_plot(
        [highlight.x, highlight.y],
        state.x_axis_scale,
        state.y_axis_scale,
    )
}

fn highlight_mask_plot_position(highlight: &HighlightPoint, state: &PlotState) -> Option<[f64; 2]> {
    let marker_style = highlight.marker_style?;
    let mut world = [highlight.x, highlight.y];

    if let Size::World(size) = marker_style.size {
        let half = size * 0.5;
        world[0] += half;
        world[1] += half;
    }

    data_point_to_plot(world, state.x_axis_scale, state.y_axis_scale)
}
