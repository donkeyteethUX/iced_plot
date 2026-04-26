mod canvas;
mod shader;
pub(crate) use canvas::draw as canvas_draw;
pub(crate) use shader::{PlotRenderer, RenderParams};

use crate::{
    axis_scale::data_point_to_plot, plot_state::PlotState, plot_widget::HighlightPoint,
    point::MarkerType, series::Size,
};
use iced::Color;

const SELECTION_FILL_RGBA: [f32; 4] = [0.2, 0.6, 1.0, 0.2];
const CROSSHAIR_RGBA: [f32; 4] = [0.5, 0.5, 0.5, 0.5];

fn color_to_rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

fn rgba_to_color(rgba: [f32; 4]) -> Color {
    Color::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn selection_fill_color() -> Color {
    rgba_to_color(SELECTION_FILL_RGBA)
}

fn crosshair_color() -> Color {
    rgba_to_color(CROSSHAIR_RGBA)
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

fn marker_type_from_u32(marker: u32) -> MarkerType {
    match marker {
        0 => MarkerType::FilledCircle,
        1 => MarkerType::EmptyCircle,
        2 => MarkerType::Square,
        3 => MarkerType::Star,
        4 => MarkerType::Triangle,
        _ => MarkerType::FilledCircle,
    }
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
