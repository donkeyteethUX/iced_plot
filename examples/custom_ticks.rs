//! This example shows how to:
//! - Use custom tick producers to control tick positions
//! - Format ticks with custom labels (e.g., time format, percentages, etc.)
//! - Control tick spacing
//! - Set per-point colors for a series
use iced_plot::{
    Color, LineStyle, MarkerStyle, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series, Tick,
    TickWeight,
};

use iced::Element;

fn main() -> iced::Result {
    iced::application(new, update, view)
        .theme(iced::theme::Theme::KanagawaDragon)
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
    // Generate some sample data representing temperature over time
    let mut positions: Vec<[f64; 2]> = Vec::new();
    let mut temps: Vec<f64> = Vec::new();
    for hour in 0..=24 {
        let t = hour as f64;
        // Temperature varies throughout the day
        let temp = 20.0 + 10.0 * (std::f64::consts::PI * (t - 6.0) / 12.0).sin();
        positions.push([t * 3600.0, temp]);
        temps.push(temp);
    }

    let (min_temp, max_temp) = temps.iter().fold((f64::MAX, f64::MIN), |acc, t| {
        (acc.0.min(*t), acc.1.max(*t))
    });
    let temp_span = (max_temp - min_temp).max(1e-6);
    let colors: Vec<Color> = temps
        .iter()
        .map(|t| {
            let n = ((*t - min_temp) / temp_span) as f32;
            let r = 0.2 + 0.8 * n;
            let g = 0.3 + 0.2 * (1.0 - n);
            let b = 1.0 - 0.8 * n;
            Color::from_rgb(r, g, b)
        })
        .collect();

    let series = Series::new(positions, MarkerStyle::circle(5.0), LineStyle::Solid)
        .with_label("Temperature")
        .with_point_colors(colors)
        .with_color(Color::from_rgb(1.0, 0.5, 0.2));

    PlotWidgetBuilder::new()
        .add_series(series)
        .with_x_label("Time of Day\n\n\n")
        .with_y_label("Temperature")
        // Custom tick producer for X axis: place ticks every 4 hours
        .with_x_tick_producer(|min, max| {
            let hour_in_seconds = 3600.0;
            let tick_interval = 4.0 * hour_in_seconds; // 4 hours

            let start = (min / tick_interval).floor() * tick_interval;
            let mut ticks = Vec::new();
            let mut value = start;

            while value <= max {
                if value >= min {
                    ticks.push(Tick {
                        value,
                        step_size: tick_interval,
                        line_type: TickWeight::Major,
                    });
                }
                value += tick_interval;
            }

            ticks
        })
        // Custom formatter for X axis: display as "HH:MM" time format
        .with_x_tick_formatter(|tick| {
            let total_seconds = tick.value as i64;
            let hours = (total_seconds / 3600) % 24;
            let minutes = (total_seconds % 3600) / 60;
            format!("{:02}:{:02}", hours, minutes)
        })
        // Custom tick producer for Y axis: place ticks every 5 degrees
        .with_y_tick_producer(|min, max| {
            let tick_interval = 5.0;
            let start = (min / tick_interval).floor() * tick_interval;
            let mut ticks = Vec::new();
            let mut value = start;

            while value <= max {
                if value >= min {
                    ticks.push(Tick {
                        value,
                        step_size: tick_interval,
                        line_type: TickWeight::Major,
                    });
                }
                value += tick_interval;
            }

            ticks
        })
        // Custom formatter for Y axis: display with degree symbol
        .with_y_tick_formatter(|tick| format!("{:.0}°C", tick.value))
        .with_cursor_provider(|x, y| {
            let hours = (x as i64 / 3600) % 24;
            let minutes = (x as i64 % 3600) / 60;
            format!("Time: {:02}:{:02}\nTemp: {:.1}°C", hours, minutes, y)
        })
        .with_hover_highlight_provider(|context, point| {
            point.color = Color::from_rgb(1.0, 0.5, 0.2);
            point.resize_marker(1.5);
            let hours = (point.x as i64 / 3600) % 24;
            let minutes = (point.x as i64 % 3600) / 60;
            Some(format!(
                "{}\nTime: {:02}:{:02}\nTemp: {:.1}°C",
                context.series_label, hours, minutes, point.y
            ))
        })
        .with_pick_highlight_provider(|context, point| {
            point.color = Color::from_rgb(1.0, 0.0, 0.0);
            point.resize_marker(3.0);
            point.mask_padding = None;
            let hours = (point.x as i64 / 3600) % 24;
            let minutes = (point.x as i64 % 3600) / 60;
            Some(format!(
                "{}\nTime: {:02}:{:02}\nTemp: {:.1}°C",
                context.series_label, hours, minutes, point.y
            ))
        })
        .build()
        .unwrap()
}
