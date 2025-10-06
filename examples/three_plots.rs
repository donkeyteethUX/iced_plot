//! Show multiple plot widgets in a single application.
//! The first plot demonstrates axis limits using with_x_lim() and with_y_lim().
use fastplot::PlotWidgetBuilder;
use fastplot::message::PlotUiMessage;
use fastplot::widget::PlotWidget;
use fastplot::{Color, LineStyle, MarkerStyle, Series};

use iced::Element;
use iced::widget::column;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view).run()
}

struct App {
    w1: PlotWidget,
    w2: PlotWidget,
    w3: PlotWidget,
}

#[derive(Debug)]
struct Message {
    msg: PlotUiMessage,
    plot_id: usize,
}

impl App {
    fn update(&mut self, Message { msg, plot_id }: Message) {
        match plot_id {
            1 => self.w1.update(msg),
            2 => self.w2.update(msg),
            3 => self.w3.update(msg),
            _ => {}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            self.w1.view().map(|msg| Message { msg, plot_id: 1 }),
            self.w2.view().map(|msg| Message { msg, plot_id: 2 }),
            self.w3.view().map(|msg| Message { msg, plot_id: 3 }),
        ]
        .into()
    }

    fn new() -> Self {
        // Line-only series (no markers)
        let n = 100;
        let mut positions = Vec::with_capacity(n);
        for i in 0..n {
            let x = i as f64 * 0.1;
            let y = (x * 0.5).sin();
            positions.push([x, y]);
        }
        let s1 = Series::line_only(positions, LineStyle::Solid).with_label("sine_line_only");

        let w1 = PlotWidgetBuilder::new()
            .with_tooltips(true)
            .with_x_lim(-1.0, 10.0) // Set x-axis limits
            .with_y_lim(-2.0, 2.0) // Set y-axis limits
            .add_series(s1)
            .build()
            .unwrap();

        // Marker-only series (no lines)
        let mut positions = Vec::with_capacity(50);
        for i in 0..50 {
            let x = i as f64 * 0.2;
            let y = (x * 0.3).cos() + 0.5;
            positions.push([x, y]);
        }
        let s2 = Series::markers_only(
            positions,
            MarkerStyle::circle(Color::from_rgb(0.9, 0.3, 0.3), 6.0),
        )
        .with_label("cosine_markers_only");

        let w2 = PlotWidgetBuilder::new()
            .with_tooltips(true)
            .add_series(s2)
            .build()
            .unwrap();

        // Both markers and lines
        let mut positions = Vec::with_capacity(30);
        for i in 0..30 {
            let x = i as f64 * 0.3;
            let y = (x * 0.8).sin() - 0.5;
            positions.push([x, y]);
        }
        let s3 = Series::markers_and_line(
            positions,
            MarkerStyle::square(Color::from_rgb(0.3, 0.9, 0.3), 4.0),
            LineStyle::Dashed { length: 10.0 },
        )
        .with_label("both_markers_and_lines");

        let w3 = PlotWidgetBuilder::new()
            .with_tooltips(true)
            .add_series(s3)
            .build()
            .unwrap();

        Self { w1, w2, w3 }
    }
}
