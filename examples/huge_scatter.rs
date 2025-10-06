//! Show rendering and fast object picking with a lot of points.
use fastplot::Series;
use fastplot::message::PlotUiMessage;
use fastplot::widget::PlotWidget;
use fastplot::{MarkerStyle, MarkerType, PlotWidgetBuilder};

use iced::{Color, Element};

use rand_distr::{Distribution, Normal};

fn main() -> iced::Result {
    iced::application(new_scatter, update, view).run()
}

#[derive(Debug, Clone)]
enum Message {
    PlotMessage(PlotUiMessage),
}

fn update(widget: &mut PlotWidget, message: Message) {
    match message {
        Message::PlotMessage(plot_msg) => {
            widget.update(plot_msg);
        }
    }
}

fn view(widget: &PlotWidget) -> Element<'_, Message> {
    widget.view().map(Message::PlotMessage)
}

fn new_scatter() -> PlotWidget {
    let mut widget = PlotWidgetBuilder::new()
        .with_tooltip_provider(|ctx| {
            format!(
                "point: {}\nx: {:.3}, y: {:.3}",
                ctx.point_index, ctx.x, ctx.y
            )
        })
        .build()
        .unwrap();

    // Generate 5 million points from 2D Gaussian
    let mut rng = rand::rng();
    let normal = Normal::new(0.0f64, 1.0f64).unwrap();
    let mut positions = Vec::with_capacity(5_000_000);
    for _ in 0..5_000_000 {
        let x = normal.sample(&mut rng);
        let y = normal.sample(&mut rng);
        positions.push([x, y]);
    }

    let series = Series::markers_only(
        positions,
        MarkerStyle {
            color: Color::from_rgb(0.2, 0.6, 1.0),
            size: 1.0,
            marker_type: MarkerType::FilledCircle,
        },
    )
    .with_label("2d Gaussian scatter - 5M points");

    widget.add_series(series).unwrap();
    widget
}
