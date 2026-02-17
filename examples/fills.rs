//! Demonstrates various fill configurations:
//! - between two series with different point counts (interpolated)
//! - between a series and a horizontal reference line
//! - between a series and a vertical reference line
//! - between two horizontal lines (band)
//! - between two vertical lines (band)

use iced::Element;
use iced_plot::{
    Color, Fill, HLine, LineStyle, MarkerStyle, PlotUiMessage, PlotWidget, PlotWidgetBuilder,
    Series, VLine,
};

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
    // Dense sine-like series
    let upper_positions: Vec<[f64; 2]> = (0..450)
        .map(|i| {
            let x = i as f64 * 0.03;
            let y = 1.1 + 0.8 * (x * 0.7).sin() + 0.12 * (x * 2.3).cos();
            [x, y]
        })
        .collect();

    // Sparser lower series with different sampling
    let lower_positions: Vec<[f64; 2]> = (0..95)
        .map(|i| {
            let x = i as f64 * 0.13;
            let y = 0.2 + 0.45 * (x * 0.9 + 0.3).sin() - 0.08 * (x * 1.7).cos();
            [x, y]
        })
        .collect();

    let upper_series = Series::line_only(upper_positions, LineStyle::Solid)
        .with_marker_style(MarkerStyle::circle(3.5))
        .with_color(Color::from_rgb(0.15, 0.55, 0.95))
        .with_label("upper series");
    let upper_id = upper_series.id;

    let lower_series = Series::line_only(lower_positions, LineStyle::Dashed { length: 8.0 })
        .with_marker_style(MarkerStyle::ring(3.0))
        .with_color(Color::from_rgb(0.95, 0.45, 0.15))
        .with_label("lower series (sparse)");
    let lower_id = lower_series.id;

    let baseline = HLine::new(0.0)
        .with_label("y = 0")
        .with_style(LineStyle::Dotted { spacing: 4.0 })
        .with_color(Color::from_rgb(0.7, 0.7, 0.7));
    let baseline_id = baseline.id;

    let threshold = VLine::new(8.0)
        .with_label("x = 8")
        .with_style(LineStyle::Dotted { spacing: 4.0 })
        .with_color(Color::from_rgb(0.7, 0.7, 0.7));
    let threshold_id = threshold.id;

    let hband_low = HLine::new(1.9)
        .with_label("h-band low")
        .with_color(Color::from_rgb(0.55, 0.65, 0.95));
    let hband_low_id = hband_low.id;

    let hband_high = HLine::new(2.4)
        .with_label("h-band high")
        .with_color(Color::from_rgb(0.55, 0.65, 0.95));
    let hband_high_id = hband_high.id;

    let vband_left = VLine::new(2.0)
        .with_label("v-band left")
        .with_color(Color::from_rgb(0.9, 0.65, 0.5));
    let vband_left_id = vband_left.id;

    let vband_right = VLine::new(3.1)
        .with_label("v-band right")
        .with_color(Color::from_rgb(0.9, 0.65, 0.5));
    let vband_right_id = vband_right.id;

    let fill_series_to_series = Fill::new(upper_id, lower_id)
        .with_label("fill: upper ↔ lower (interpolated)")
        .with_color(Color::from_rgba(0.2, 0.7, 1.0, 0.24));

    let fill_to_baseline = Fill::new(upper_id, baseline_id)
        .with_label("fill: upper ↔ y=0")
        .with_color(Color::from_rgba(0.25, 0.85, 0.45, 0.16));

    let fill_to_vline = Fill::new(lower_id, threshold_id)
        .with_label("fill: lower ↔ x=8")
        .with_color(Color::from_rgba(0.95, 0.55, 0.2, 0.12));

    let fill_hband = Fill::new(hband_low_id, hband_high_id)
        .with_label("fill: horizontal band")
        .with_color(Color::from_rgba(0.5, 0.6, 0.95, 0.16));

    let fill_vband = Fill::new(vband_left_id, vband_right_id)
        .with_label("fill: vertical band")
        .with_color(Color::from_rgba(0.95, 0.7, 0.5, 0.14));

    PlotWidgetBuilder::new()
        .with_x_label("x")
        .with_y_label("y")
        .with_x_lim(0.0, 13.5)
        .with_y_lim(-1.2, 3.0)
        .add_series(upper_series)
        .add_series(lower_series)
        .add_hline(baseline)
        .add_vline(threshold)
        .add_hline(hband_low)
        .add_hline(hband_high)
        .add_vline(vband_left)
        .add_vline(vband_right)
        .add_fill(fill_series_to_series)
        .add_fill(fill_to_baseline)
        .add_fill(fill_to_vline)
        .add_fill(fill_hband)
        .add_fill(fill_vband)
        .with_cursor_overlay(true)
        .with_crosshairs(true)
        .build()
        .unwrap()
}
