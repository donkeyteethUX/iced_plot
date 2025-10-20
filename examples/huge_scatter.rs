//! Show rendering and fast object picking with a lot of points.
use iced_plot::Series;
use iced_plot::message::PlotUiMessage;
use iced_plot::plot_widget::PlotWidget;
use iced_plot::{MarkerStyle, MarkerType, PlotWidgetBuilder};

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
    // Generate 5 million points from 2D Gaussian
    let mut rng = rand::rng();
    let normal = Normal::new(0.0f64, 1.0f64).unwrap();
    let positions = (0..5_000_000)
        .map(|_| [normal.sample(&mut rng), normal.sample(&mut rng)])
        .collect::<Vec<[f64; 2]>>();

    let series = Series::markers_only(
        positions,
        MarkerStyle {
            size: 1.0,
            marker_type: MarkerType::FilledCircle,
        },
    )
    .with_label("2d Gaussian scatter - 5M points")
    .with_color(Color::from_rgb(0.2, 0.6, 1.0));

    PlotWidgetBuilder::new()
        .with_tooltip_provider(|ctx| {
            format!(
                "point: {}\nx: {:.3}, y: {:.3}",
                ctx.point_index, ctx.x, ctx.y
            )
        })
        .add_series(series.clone())
        .build()
        .unwrap()
}
