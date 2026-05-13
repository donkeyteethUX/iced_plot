//! Example demonstrating coordinate transforms.
//!
//! The plot axes stay linear. Each series or reference line can choose how its
//! own x/y values are converted before drawing. `Transform::axes()` uses
//! normalized plot positions, so `0.4` means 40% across the plot area.

use std::f64::consts::{E, TAU};

use iced::Element;
use iced_plot::{
    Color, HLine, LineStyle, MarkerStyle, PlotUiMessage, PlotWidget, PlotWidgetBuilder,
    PositionTransform, Series, Transform, VLine,
};

fn main() -> iced::Result {
    iced::application(new, update, view)
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

fn sample(count: usize, mut f: impl FnMut(f64) -> [f64; 2]) -> Vec<[f64; 2]> {
    (0..count)
        .map(|i| {
            let t = i as f64 / (count.saturating_sub(1).max(1)) as f64;
            f(t)
        })
        .collect()
}

fn new() -> PlotWidget {
    let identity = Series::line_only(
        sample(160, |t| {
            let x = t * 2.0;
            [x, (TAU * t).sin() * 0.55]
        }),
        LineStyle::solid().with_pixel_width(2.0),
    )
    .with_transform(PositionTransform::new(Some(Transform::identity()), None))
    .with_label("identity data")
    .with_color(Color::from_rgb(0.2, 0.6, 1.0));

    let affine_y = Series::line_only(
        sample(160, |t| {
            let x = t * 2.0;
            let normalized = ((TAU * t).cos() * 0.5 + 0.5).clamp(0.0, 1.0);
            [x, normalized]
        }),
        LineStyle::dashed(8.0).with_pixel_width(2.0),
    )
    .with_transform_y(Transform::affine(2.0, -1.0))
    .with_label("y affine: y * 2 - 1")
    .with_color(Color::from_rgb(0.9, 0.45, 0.15));

    let log_x = Series::line_only(
        sample(160, |t| {
            let x_plot = t * 2.0;
            [10.0_f64.powf(x_plot), 1.05 + (TAU * t).sin() * 0.25]
        }),
        LineStyle::solid().with_pixel_width(2.0),
    )
    .with_transform_x(Transform::log(10.0))
    .with_label("x log10(raw)")
    .with_color(Color::from_rgb(0.35, 0.85, 0.4));

    let exp_y = Series::line_only(
        sample(160, |t| {
            let x = t * 2.0;
            let y_after_transform = 1.25 + x * 0.45;
            [x, y_after_transform.ln()]
        }),
        LineStyle::dotted(5.0).with_pixel_width(2.5),
    )
    .with_transform_y(Transform::exp(E))
    .with_label("y exp(raw)")
    .with_color(Color::from_rgb(0.85, 0.2, 0.55));

    let composed_x = Series::line_only(
        sample(160, |t| [t, 2.45 + (TAU * t).cos() * 0.22]),
        LineStyle::dashed(5.0).with_pixel_width(2.0),
    )
    .with_transform_x(Transform::affine(99.0, 1.0).then(Transform::log(10.0)))
    .with_label("x (raw * 99 + 1) then log10")
    .with_color(Color::from_rgb(0.65, 0.45, 0.95));

    let axes_line = Series::line_only(
        vec![[0.05, 0.88], [0.95, 0.88]],
        LineStyle::solid().with_pixel_width(3.0),
    )
    .with_axes_transform()
    .with_label("axes line at 88%")
    .with_color(Color::from_rgb(0.0, 0.9, 0.95));

    let mixed_axes_x = Series::line_only(
        vec![[0.5, -1.25], [0.5, 3.15]],
        LineStyle::dotted(3.0).with_pixel_width(3.0),
    )
    .with_transform_x(Transform::axes())
    .with_label("x axes=50%, y data")
    .with_color(Color::from_rgb(0.95, 0.8, 0.15));

    let axes_marker = Series::new(
        vec![[0.92, 0.14]],
        MarkerStyle::star(14.0),
        LineStyle::solid(),
    )
    .with_transform(PositionTransform::new(
        Some(Transform::axes()),
        Some(Transform::axes()),
    ))
    .with_label("right-lower marker")
    .with_color(Color::from_rgb(1.0, 0.2, 0.2));

    let center_vline = VLine::new(0.25)
        .with_axes_transform()
        .with_label("vline x=25% axes")
        .with_color(Color::from_rgb(0.8, 0.8, 0.8))
        .with_width(1.5)
        .with_style(LineStyle::dashed(4.0));

    let center_hline = HLine::new(0.5)
        .with_axes_transform()
        .with_label("hline y=50% axes")
        .with_color(Color::from_rgb(0.8, 0.8, 0.8))
        .with_width(1.5)
        .with_style(LineStyle::dashed(4.0));

    PlotWidgetBuilder::new()
        .with_x_label("plot x after transform")
        .with_y_label("plot y after transform")
        .with_x_lim(-0.2, 2.2)
        .with_y_lim(-1.4, 3.3)
        .add_series(identity)
        .add_series(affine_y)
        .add_series(log_x)
        .add_series(exp_y)
        .add_series(composed_x)
        .add_series(axes_line)
        .add_series(mixed_axes_x)
        .add_series(axes_marker)
        .add_vline(center_vline)
        .add_hline(center_hline)
        .with_cursor_overlay(true)
        .build()
        .unwrap()
}
