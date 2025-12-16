//! Super simple plot with a few series types.
use iced_plot::PlotUiMessage;
use iced_plot::PlotWidget;
use iced_plot::PlotWidgetBuilder;
use iced_plot::{Color, LineStyle, MarkerStyle, Series, TooltipContext};

use iced::Element;

fn main() -> iced::Result {
    iced::application(new, update, view).run()
}

fn update(widget: &mut PlotWidget, message: PlotUiMessage) {
    widget.update(message);
}

fn view(widget: &PlotWidget) -> Element<'_, PlotUiMessage> {
    widget.view()
}

fn new() -> PlotWidget {
    let positions = (0..100)
        .map(|i| {
            let x = i as f64 * 0.1;
            let y = (x * 0.5).sin();
            [x, y]
        })
        .collect();

    let s1 = Series::line_only(positions, LineStyle::Solid)
        .with_label("sine_line_only")
        .with_color(Color::from_rgb(0.3, 0.3, 0.9));

    let positions = (0..50)
        .map(|i| {
            let x = i as f64 * 0.2;
            let y = (x * 0.3).cos() + 0.5;
            [x, y]
        })
        .collect();
    let s2 = Series::markers_only(positions, MarkerStyle::circle(6.0))
        .with_label("cosine_markers_only")
        .with_color(Color::from_rgb(0.9, 0.3, 0.3));

    let positions = (0..30)
        .map(|i| {
            let x = i as f64 * 0.3;
            let y = (x * 0.8).sin() - 0.5;
            [x, y]
        })
        .collect();
    let s3 = Series::new(
        positions,
        MarkerStyle::square(4.0),
        LineStyle::Dashed { length: 10.0 },
    )
    .with_label("both_markers_and_lines")
    .with_color(Color::from_rgb(0.3, 0.9, 0.3));

    PlotWidgetBuilder::new()
        .with_tooltips(true)
        .with_tooltip_provider(|ctx: &TooltipContext| {
            format!(
                "{}\nIndex: {}\nX: {:.2}\nY: {:.2}",
                ctx.series_label, ctx.point_index, ctx.x, ctx.y
            )
        })
        .add_series(s1)
        .add_series(s2)
        .add_series(s3)
        .with_cursor_overlay(true)
        .with_cursor_provider(|x, y| format!("Your cursor is at: X: {x:.2}, Y: {y:.2}"))
        .with_y_label("Y Axis (Custom Font Size)")
        .with_x_label("X Axis (Custom Font Size)")
        .with_x_tick_formatter(|tick| format!("{:.1}s", tick.value))
        .with_tick_label_size(12.0)
        .with_axis_label_size(18.0)
        .with_crosshairs(true)
        .build()
        .unwrap()
}
