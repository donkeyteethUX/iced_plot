//! Example demonstrating vertical and horizontal reference lines with
//! functions that have asymptotes aligned to those lines.
use iced_plot::PlotUiMessage;
use iced_plot::PlotWidget;
use iced_plot::{Color, HLine, LineStyle, MarkerStyle, PlotWidgetBuilder, Series, VLine};

use iced::Element;
use std::f64::consts::{PI, TAU};

fn main() -> iced::Result {
    iced::application(new, update, view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

fn update(widget: &mut PlotWidget, message: PlotUiMessage) {
    widget.update(message);
}

fn view(widget: &PlotWidget) -> Element<'_, PlotUiMessage> {
    widget.view()
}

fn new() -> PlotWidget {
    // Plot some interesting functions with asymptotes.
    let clamp = |y: f64| y.clamp(-20.0, 20.0);

    let mut tan_seg1 = Vec::new();
    let end1 = PI - 0.0001;
    let mut x = 0.0;
    while x <= end1 {
        tan_seg1.push([x, clamp((x - PI / 2.0).tan())]);
        x += 0.01;
    }

    let mut tan_seg2 = Vec::new();
    let start2 = PI + 0.0001;
    let end2 = TAU - 0.0001;
    x = start2;
    while x <= end2 {
        tan_seg2.push([x, clamp((x - PI / 2.0).tan())]);
        x += 0.01;
    }

    let tan1 = Series::squares(tan_seg1, 4.0)
        .with_label("tan_1")
        .with_color(Color::from_rgb(0.3, 0.6, 0.9));

    let tan2 = Series::line_only(tan_seg2, LineStyle::Solid)
        .with_marker_style(MarkerStyle::star(10.0))
        .with_color(Color::from_rgb(0.7, 0.2, 0.1))
        .with_label("tan_2");

    let k = 1.2;
    let mut tanh_positions = Vec::new();
    x = 0.0;
    while x <= TAU {
        tanh_positions.push([x, (k * (x - 1.5 * PI)).tanh()]);
        x += 0.01;
    }
    let tanh_s = Series::line_only(tanh_positions, LineStyle::Dashed { length: 8.0 })
        .with_label("y = tanh(1.2·(x - 1.5π))")
        .with_color(Color::from_rgb(0.2, 0.8, 0.5));

    // Add vertical reference lines at the asymptotes of tan(x - π/2)
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

    // Add horizontal reference lines at y = ±1 (asymptotes of tanh)
    let hline1 = HLine::new(1.0)
        .with_label("y=1.0")
        .with_color(Color::from_rgb(0.3, 0.9, 0.5))
        .with_width(2.5)
        .with_style(LineStyle::Dotted { spacing: 5.0 });

    let hline2 = HLine::new(-1.0)
        .with_label("y=-1.0")
        .with_color(Color::from_rgb(0.3, 0.9, 0.5))
        .with_width(2.5)
        .with_style(LineStyle::Dotted { spacing: 5.0 });

    PlotWidgetBuilder::new()
        .with_x_label("x")
        .with_y_label("y")
        .add_series(tan1)
        .add_series(tan2)
        .add_series(tanh_s)
        .add_vline(vline1)
        .add_vline(vline2)
        .add_hline(hline1)
        .add_hline(hline2)
        .with_cursor_overlay(true)
        .build()
        .unwrap()
}
