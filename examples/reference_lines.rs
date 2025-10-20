//! Example demonstrating vertical and horizontal reference lines.
use fastplot::message::PlotUiMessage;
use fastplot::widget::PlotWidget;
use fastplot::{Color, HLine, LineStyle, MarkerStyle, PlotWidgetBuilder, Series, VLine};

use iced::Element;
use std::f64::consts::{PI, TAU};

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
    // Create a sine wave series
    let mut positions = Vec::new();
    for i in 0..500 {
        let x = i as f64 * 0.05;
        let y = (x * 0.5).sin();
        positions.push([x, y]);
    }
    let s1 = Series::markers_and_line(positions, MarkerStyle::circle(3.0), LineStyle::Solid)
        .with_label("sine wave")
        .with_color(Color::from_rgb(0.3, 0.6, 0.9));

    // Add vertical reference lines at specific x-values
    let vline1 = VLine::new(PI)
        .with_label("π")
        .with_color(Color::from_rgb(0.9, 0.3, 0.3))
        .with_width(2.0)
        .with_style(LineStyle::Solid);

    let vline2 = VLine::new(TAU)
        .with_label("2π")
        .with_color(Color::from_rgb(0.9, 0.5, 0.3))
        .with_width(2.0)
        .with_style(LineStyle::Dashed { length: 1.0 });

    // Add horizontal reference lines
    let hline1 = HLine::new(1.0)
        .with_label("y=1.0")
        .with_color(Color::from_rgb(0.3, 0.9, 0.5))
        .with_width(2.5)
        .with_style(LineStyle::Dotted { spacing: 5.0 });

    let hline2 = HLine::new(1.0)
        .with_label("y=-1.0")
        .with_color(Color::from_rgb(0.3, 0.9, 0.5))
        .with_width(2.5)
        .with_style(LineStyle::Dotted { spacing: 5.0 });

    PlotWidgetBuilder::new()
        .with_x_label("x")
        .with_y_label("y")
        .add_series(s1)
        .add_vline(vline1)
        .add_vline(vline2)
        .add_hline(hline1)
        .add_hline(hline2)
        .with_cursor_overlay(true)
        .build()
        .unwrap()
}
