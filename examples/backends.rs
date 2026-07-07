//! Compare marker rendering between the shader and canvas backends.

use iced::{
    Element, Length,
    widget::{column, container, row, text},
};
use iced_plot::{
    Color, MarkerStyle, PlotRenderStrategy, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series,
};

const MARKER_SIZE: f32 = 48.0;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

struct App {
    plot_a: PlotWidget,
    plot_b: PlotWidget,
}

#[derive(Debug, Clone)]
enum Message {
    PlotA(PlotUiMessage),
    PlotB(PlotUiMessage),
}

impl App {
    fn new() -> Self {
        Self {
            plot_a: marker_plot(PlotRenderStrategy::Shader),
            plot_b: marker_plot(PlotRenderStrategy::Canvas),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PlotA(message) => self.plot_a.update(message),
            Message::PlotB(message) => self.plot_b.update(message),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        row![
            plot_panel("Shader Plot:", self.plot_a.view().map(Message::PlotA)),
            plot_panel("Canvas Plot:", self.plot_b.view().map(Message::PlotB)),
        ]
        .spacing(12)
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn plot_panel<'a>(title: &'static str, plot: Element<'a, Message>) -> Element<'a, Message> {
    column![
        text(title).size(18),
        container(plot).width(Length::Fill).height(Length::Fill),
    ]
    .spacing(8)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn marker_plot(strategy: PlotRenderStrategy) -> PlotWidget {
    let markers = [
        (
            "circle",
            MarkerStyle::circle(MARKER_SIZE),
            Color::from_rgb(0.25, 0.50, 0.95),
        ),
        (
            "ring",
            MarkerStyle::ring(MARKER_SIZE),
            Color::from_rgb(0.10, 0.70, 0.45),
        ),
        (
            "square",
            MarkerStyle::square(MARKER_SIZE),
            Color::from_rgb(0.90, 0.55, 0.10),
        ),
        (
            "star",
            MarkerStyle::star(MARKER_SIZE),
            Color::from_rgb(0.95, 0.25, 0.15),
        ),
        (
            "triangle",
            MarkerStyle::triangle(MARKER_SIZE),
            Color::from_rgb(0.60, 0.35, 0.90),
        ),
    ];

    let mut builder = PlotWidgetBuilder::new()
        .with_render_strategy(strategy)
        .with_x_label("x")
        .with_y_label("y")
        .with_x_lim(-1.0, 1.0)
        .with_y_lim(-0.75, markers.len() as f64 - 0.25)
        .disable_controls_help();

    for (index, (label, style, color)) in markers.into_iter().enumerate() {
        let y = markers.len() as f64 - 1.0 - index as f64;
        let series = Series::markers_only(vec![[0.0, y]], style)
            .with_label(label)
            .with_color(color);
        builder = builder.add_series(series);
    }

    builder.build().unwrap()
}
