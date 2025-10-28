//! This example shows how to:
//! - Use custom tick producers to control tick positions
//! - Format ticks with custom labels (e.g., time format, percentages, etc.)
//! - Control tick spacing
use iced_plot::{
    Color, LineStyle, MarkerStyle, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series, Tick,
    TickWeight,
};

use iced::Element;

fn main() -> iced::Result {
    iced::application(new, update, view)
        .theme(iced::theme::Theme::KanagawaDragon)
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
    let positions: Vec<[f64; 2]> = (0..=24)
        .map(|hour| {
            let t = hour as f64;
            // Temperature varies throughout the day
            let temp = 20.0 + 10.0 * (std::f64::consts::PI * (t - 6.0) / 12.0).sin();
            [t * 3600.0, temp]
        })
        .collect();

    let series = Series::new(positions, MarkerStyle::circle(5.0), LineStyle::Solid)
        .with_label("Temperature")
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
        .build()
        .unwrap()
}
