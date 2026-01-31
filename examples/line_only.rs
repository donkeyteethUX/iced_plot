//! Super simple plot with a few series types.
use iced_plot::HoverPickEvent;
use iced_plot::PlotUiMessage;
use iced_plot::PlotWidget;
use iced_plot::PlotWidgetBuilder;
use iced_plot::{Color, LineStyle, MarkerStyle, Series};

use iced::Element;

fn main() -> iced::Result {
    iced::application(new, update, view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

fn update(widget: &mut PlotWidget, message: PlotUiMessage) {
    let hover_pick_event = message.get_hover_pick_event();
    widget.update(message);
    // after PlotWidget's update, update the hover and pick points for the other series
    match hover_pick_event {
        Some(HoverPickEvent::Hover(point_id)) => {
            if let Some([x, _]) = widget.point_position(point_id) {
                for series_id in widget.series_ids() {
                    if series_id != point_id.series_id {
                        if let Some(p) = widget.nearest_point_horizontal(series_id, x) {
                            widget.add_hover_point(p);
                        }
                    }
                }
            }
        }
        Some(HoverPickEvent::Pick(point_id)) => {
            if let Some([x, _]) = widget.point_position(point_id) {
                for series_id in widget.series_ids() {
                    if series_id != point_id.series_id {
                        if let Some(p) = widget.nearest_point_horizontal(series_id, x) {
                            widget.add_pick_point(p);
                        }
                    }
                }
            }
        }
        _ => {}
    }
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
        .with_hover_highlight_provider(|context, point| {
            if point.marker_style.is_none() {
                point.marker_style = Some(MarkerStyle::circle(6.0));
            }
            Some(format!(
                "Index: {}\nX: {:.2}\nY: {:.2}",
                context.point_index, point.x, point.y
            ))
        })
        .with_pick_highlight_provider(|ctx, point| {
            if point.marker_style.is_none() {
                // set plot 1 to star in pick highlight
                point.marker_style = Some(MarkerStyle::triangle(6.0));
            }
            point.mask_padding = None;
            point.resize_marker(1.5);
            point.color = Color::from_rgb(1.0, 0.0, 0.0);
            Some(format!(
                "Index: {}\nX: {:.2}\nY: {:.2}\n(Selected)",
                ctx.point_index, point.x, point.y
            ))
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
