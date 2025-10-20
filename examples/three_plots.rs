//! Show multiple plot widgets in a single application.
//! All three plots have their x-axes linked, so panning or zooming the x-axis
//! on one plot will synchronize the others.
use iced_plot::PlotWidgetBuilder;
use iced_plot::message::PlotUiMessage;
use iced_plot::plot_widget::PlotWidget;
use iced_plot::{AxisLink, Color, LineStyle, MarkerStyle, Series};

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
        // Create a shared x-axis link so all three plots pan/zoom together on the x-axis
        let x_link = AxisLink::new();

        let positions = (0..100)
            .map(|i| {
                let x = i as f64 * 0.1;
                let y = (x * 0.5).sin();
                [x, y]
            })
            .collect();
        let s1 = Series::line_only(positions, LineStyle::Solid).with_label("sine_line_only");

        let w1 = PlotWidgetBuilder::new()
            .with_tooltips(true)
            .with_x_lim(-1.0, 10.0) // Set x-axis limits
            .with_y_lim(-2.0, 2.0) // Set y-axis limits
            .with_x_axis_link(x_link.clone()) // Link the x-axis
            .add_series(s1)
            .build()
            .unwrap();

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

        let w2 = PlotWidgetBuilder::new()
            .with_tooltips(true)
            .with_x_axis_link(x_link.clone()) // Link the x-axis
            .add_series(s2)
            .build()
            .unwrap();

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

        let w3 = PlotWidgetBuilder::new()
            .with_tooltips(true)
            .with_x_axis_link(x_link.clone()) // Link the x-axis
            .add_series(s3)
            .build()
            .unwrap();

        Self { w1, w2, w3 }
    }
}
