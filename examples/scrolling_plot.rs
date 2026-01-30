//! Example of a scrolling plot with new data points being added over time.
use iced_plot::PlotUiMessage;
use iced_plot::PlotWidget;
use iced_plot::{MarkerStyle, PlotWidgetBuilder};
use iced_plot::{Series, ShapeId};

use iced::window;
use iced::{Color, Element};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    PlotMessage(PlotUiMessage),
    Tick,
}

struct App {
    series_id: ShapeId,
    widget: PlotWidget,
    x: f64,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::PlotMessage(plot_msg) => {
                self.widget.update(plot_msg);
            }
            Message::Tick => {
                let y = (self.x * 0.5).sin();
                // Update the series
                self.widget
                    .update_series(&self.series_id, |series| {
                        // Add new point
                        series.positions.push([self.x, y]);
                        // Keep only last 300 points for scrolling effect
                        if series.positions.len() > 300 {
                            series.positions.remove(0);
                        }
                    })
                    .unwrap();
                self.x += 0.1f64;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.widget.view().map(Message::PlotMessage)
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        window::frames().map(|_| Message::Tick)
    }

    fn new() -> Self {
        let x = 0.0f64;
        let series = Series::markers_only(vec![[x, (x * 0.5).sin()]], MarkerStyle::ring(10.0))
            .with_label("scrolling")
            .with_color(Color::WHITE);
        Self {
            series_id: series.id,
            widget: PlotWidgetBuilder::new()
                .with_autoscale_on_updates(true)
                .add_series(series)
                .build()
                .unwrap(),
            x,
        }
    }
}
