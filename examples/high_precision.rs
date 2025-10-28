//! Demonstrates rendering of large values with high precision made possible by using a
//! double-precision camera and offsetting coordinates to keep them near zero when rendering.
use std::time::Duration;

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
    // Suppose we're plotting some time-series data which is nanoseconds apart, but the first timestamp
    // is very large.
    let mut t = Duration::from_secs(60 * 60 * 24 * 7); // 1 week

    let mut positions = Vec::new();
    for i in 0..100 {
        let y = 100.0 + (i as f64 / 10.0).cos() / 10000.0;
        positions.push([t.as_secs_f64(), y]);
        t += Duration::from_nanos(10);
    }

    PlotWidgetBuilder::new()
        .with_tooltips(true)
        .with_tooltip_provider(|ctx: &TooltipContext| {
            format!("t = {:.9} s\nvalue = {:.4}", ctx.x, ctx.y)
        })
        .with_cursor_provider(|x, y| format!("Cursor:\nt = {:.9} s\nvalue = {:.6}", x, y))
        .add_series(
            Series::new(
                positions,
                MarkerStyle::square(4.0),
                LineStyle::Dashed { length: 10.0 },
            )
            .with_label("both_markers_and_lines")
            .with_color(Color::from_rgb(0.3, 0.9, 0.3)),
        )
        .with_cursor_overlay(true)
        .with_crosshairs(true)
        .with_y_label("cool data")
        .with_x_label("time (s)\n\n\n")
        .build()
        .unwrap()
}
