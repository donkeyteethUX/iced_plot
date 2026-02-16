use std::f64::consts::E;

use iced::Element;
use iced_plot::{
    AxisScale, Color, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series, log_formatter,
    log_tick_producer,
};

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
    let mut values = Vec::new();
    for i in -10..=200 {
        let x = i as f64 * 0.1;
        let y = x * x + 0.05;
        values.push([x, y]);
    }

    let series = Series::circles(values, 4.0)
        .with_label("y = x² + 0.05")
        .with_color(Color::from_rgb(0.2, 0.8, 1.0));

    // Build a log-log plot.
    //
    // Note that we also override the tick producers to place ticks on powers,
    // and the formatters to use scientific notation. This is optional. You could
    // use the built-in ones if you don't need evenly spaced ticks, or provide your
    // own.
    PlotWidgetBuilder::new()
        .with_x_label("x")
        .with_y_label("y")
        .with_x_scale(AxisScale::Log { base: E })
        .with_y_scale(AxisScale::Log { base: E })
        .with_x_tick_producer(|min, max| log_tick_producer(E, min, max))
        .with_y_tick_producer(|min, max| log_tick_producer(E, min, max))
        .with_x_tick_formatter(|t| log_formatter(t, E))
        .with_y_tick_formatter(|t| log_formatter(t, E))
        .add_series(series)
        .build()
        .unwrap()
}
