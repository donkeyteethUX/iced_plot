//! Heatmap example using colored world-spacesquare markers.
use iced_plot::{
    Color, MarkerStyle, MarkerType, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series,
};

use iced::Element;

fn main() -> iced::Result {
    iced::application(new, update, view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .theme(iced::theme::Theme::TokyoNightStorm)
        .run()
}

fn update(widget: &mut PlotWidget, message: PlotUiMessage) {
    widget.update(message);
}

fn view(widget: &PlotWidget) -> Element<'_, PlotUiMessage> {
    widget.view()
}

fn new() -> PlotWidget {
    let cols = 40;
    let rows = 30;
    let mut positions = Vec::with_capacity(cols * rows);
    let mut values = Vec::with_capacity(cols * rows);

    for y in 0..rows {
        for x in 0..cols {
            let nx = x as f64 / (cols - 1) as f64;
            let ny = y as f64 / (rows - 1) as f64;
            let value = heat_value(nx, ny);
            positions.push([x as f64, y as f64]);
            values.push(value);
        }
    }

    let (min_value, max_value) = values.iter().fold((f64::MAX, f64::MIN), |acc, v| {
        (acc.0.min(*v), acc.1.max(*v))
    });
    let span = (max_value - min_value).max(1e-12);
    let colors = values
        .iter()
        .map(|v| heat_color((v - min_value) / span))
        .collect::<Vec<Color>>();

    let heatmap = Series::markers_only(positions, MarkerStyle::new_world(0.9, MarkerType::Square))
        .with_label("heatmap")
        .with_point_colors(colors);

    PlotWidgetBuilder::new()
        .add_series(heatmap)
        .with_x_label("X")
        .with_y_label("Y")
        .with_tick_label_size(12.0)
        .with_axis_label_size(18.0)
        .with_data_aspect(1.0) // keep the pixels square
        .with_hover_highlight_provider(move |_ctx, point| {
            // point.mask_padding = Some(1.0);
            let nx = point.x / (cols - 1) as f64;
            let ny = point.y / (rows - 1) as f64;
            let value = heat_value(nx, ny);
            Some(format!(
                "cell: ({:.0}, {:.0})\nvalue: {:.3}",
                point.x, point.y, value
            ))
        })
        .with_cursor_overlay(true)
        .build()
        .unwrap()
}

fn heat_value(nx: f64, ny: f64) -> f64 {
    let gx = (nx - 0.35) / 0.18;
    let gy = (ny - 0.65) / 0.22;
    let gaussian = (-0.5 * (gx * gx + gy * gy)).exp();
    let waves = (nx * 4.5).sin() * (ny * 3.5).cos();
    gaussian + 0.35 * waves
}

// An ad-hoc colormap for the heatmap values.
fn heat_color(value: f64) -> Color {
    let v = value.clamp(0.0, 1.0) as f32;
    let c0 = (0.10, 0.15, 0.45);
    let c1 = (0.15, 0.60, 0.75);
    let c2 = (0.85, 0.85, 0.25);
    let c3 = (0.90, 0.20, 0.12);

    let (a, b, t) = if v < 0.33 {
        (c0, c1, v / 0.33)
    } else if v < 0.66 {
        (c1, c2, (v - 0.33) / 0.33)
    } else {
        (c2, c3, (v - 0.66) / 0.34)
    };

    Color::from_rgb(lerp(a.0, b.0, t), lerp(a.1, b.1, t), lerp(a.2, b.2, t))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
