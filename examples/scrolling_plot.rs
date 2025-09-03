//! Show a scrolling plot with new data points being added over time.
//! This uses the `autoscale_on_updates` option.
use fastplot::Series;
use fastplot::message::PlotUiMessage;
use fastplot::widget::PlotWidget;
use fastplot::{MarkerStyle, PlotWidgetBuilder};

use iced::window;
use iced::{Color, Element};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    PlotMessage(PlotUiMessage),
    Tick,
}

struct App {
    widget: PlotWidget,
    positions: Vec<[f32; 2]>,
    x: f32,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::PlotMessage(plot_msg) => {
                self.widget.update(plot_msg);
            }
            Message::Tick => {
                // Add new point
                let y = (self.x * 0.5).sin();
                self.positions.push([self.x, y]);
                self.x += 0.1f32;

                // Keep only last 300 points for scrolling effect
                if self.positions.len() > 300 {
                    self.positions.remove(0);
                }

                // Update the series
                self.widget.remove_series("scrolling");
                let series = Series::markers_only(
                    self.positions.clone(),
                    MarkerStyle::ring(Color::WHITE, 10.0),
                )
                .with_label("scrolling");
                self.widget.add_series(series).unwrap();
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
        Self {
            widget: PlotWidgetBuilder::new()
                .with_autoscale_on_updates(true)
                .build()
                .unwrap(),
            positions: Vec::new(),
            x: 0.0f32,
        }
    }
}
