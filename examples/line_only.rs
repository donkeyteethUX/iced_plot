//! Super simple plot with a few series types.
use fastplot::PlotWidgetBuilder;
use fastplot::message::PlotUiMessage;
use fastplot::widget::PlotWidget;
use fastplot::{Color, LineStyle, MarkerStyle, MarkerType, Series, TooltipContext};

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
    // Line-only series (no markers)
    let mut positions = Vec::new();
    for i in 0..100 {
        let x = i as f32 * 0.1;
        let y = (x * 0.5).sin();
        positions.push([x, y]);
    }
    let s1 = Series::line_only(positions, LineStyle::Solid).with_label("sine_line_only");

    // Marker-only series (no lines)
    let mut positions = Vec::new();
    for i in 0..50 {
        let x = i as f32 * 0.2;
        let y = (x * 0.3).cos() + 0.5;
        positions.push([x, y]);
    }
    let s2 = Series::markers_only(
        positions,
        MarkerStyle::new(
            Color::from_rgb(0.9, 0.3, 0.3),
            6.0,
            MarkerType::FilledCircle,
        ),
    )
    .with_label("cosine_markers_only");

    // Both markers and lines
    let mut positions = Vec::new();
    for i in 0..30 {
        let x = i as f32 * 0.3;
        let y = (x * 0.8).sin() - 0.5;
        positions.push([x, y]);
    }
    let s3 = Series::markers_and_line(
        positions,
        MarkerStyle::square(Color::from_rgb(0.3, 0.9, 0.3), 4.0),
        LineStyle::Dashed { length: 10.0 },
    )
    .with_label("both_markers_and_lines");

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
        .with_y_label("should wrap on word level if too long")
        .with_x_label("an x axis label")
        .build()
        .unwrap()
}
