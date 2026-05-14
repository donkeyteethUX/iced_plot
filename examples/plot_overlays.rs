//! Demonstrates external Iced elements anchored to plot coordinates.
use std::f64::consts::{FRAC_PI_2, TAU};

use iced::{
    Element, Length, Theme, alignment, border,
    widget::{button, column, container, row, text},
};
use iced_plot::{
    Color, LineStyle, MarkerStyle, PlotOverlay, PlotUiMessage, PlotWidget, PlotWidgetBuilder,
    Series,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

struct App {
    widget: PlotWidget,
    annotation_expanded: bool,
    shape_vertical: alignment::Vertical,
    shape_horizontal: alignment::Horizontal,
}

#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
    Shape(ShapeMessage),
}

#[derive(Debug, Clone)]
enum ShapeMessage {
    ToggleAnnotation,
    CycleVertical,
    CycleHorizontal,
}

impl App {
    fn new() -> Self {
        Self {
            widget: plot_widget(),
            annotation_expanded: true,
            shape_vertical: alignment::Vertical::Top,
            shape_horizontal: alignment::Horizontal::Center,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Plot(message) => self.widget.update(message),
            Message::Shape(ShapeMessage::ToggleAnnotation) => {
                self.annotation_expanded = !self.annotation_expanded;
            }
            Message::Shape(ShapeMessage::CycleVertical) => {
                self.shape_vertical = next_vertical(self.shape_vertical);
            }
            Message::Shape(ShapeMessage::CycleHorizontal) => {
                self.shape_horizontal = next_horizontal(self.shape_horizontal);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.widget
            .view_with_shapes(self.bottom_shapes(), self.top_shapes(), Message::Plot)
    }

    fn bottom_shapes(&self) -> impl Iterator<Item = PlotOverlay<'_, Message>> {
        let region = container(text("axes-space region\n   (on bottom)").size(13.0))
            .width(Length::Fixed(190.0))
            .height(Length::Fixed(76.0))
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(region_style);

        std::iter::once(
            PlotOverlay::new(region, [0.5, 0.5])
                .with_axes_transform()
                .align_to_anchor(alignment::Horizontal::Center, alignment::Vertical::Center)
                .map(Message::Shape),
        )
    }

    fn top_shapes(&self) -> impl Iterator<Item = PlotOverlay<'_, Message>> {
        let label = if self.annotation_expanded {
            column![
                text("sin(x) peak").size(14.0),
                text("click to collapse").size(11.0).style(text::secondary),
            ]
            .spacing(2)
        } else {
            column![text("peak").size(14.0)]
        };

        let annotation = button(container(label).padding([4, 8]).style(annotation_style))
            .padding(0)
            .on_press(Message::Shape(ShapeMessage::ToggleAnnotation));

        let alignment_controls = container(
            column![
                text("sin(x) trough").size(14.0),
                row![
                    button(text(format!(
                        "Align vertical:\n{}",
                        vertical_label(self.shape_vertical)
                    )))
                    .padding([4, 8])
                    .on_press(Message::Shape(ShapeMessage::CycleVertical)),
                    button(text(format!(
                        "Align horizontal:\n{}",
                        horizontal_label(self.shape_horizontal)
                    )))
                    .padding([4, 8])
                    .on_press(Message::Shape(ShapeMessage::CycleHorizontal)),
                ]
                .spacing(6),
            ]
            .spacing(6),
        )
        .padding(8)
        .style(annotation_style);

        [
            PlotOverlay::new(annotation, [FRAC_PI_2, 1.0])
                .align_to_anchor(alignment::Horizontal::Center, alignment::Vertical::Top)
                .with_anchor_offset([20.0, 10.0]),
            PlotOverlay::new(alignment_controls, [FRAC_PI_2 * 3.0, -1.0])
                .align_to_anchor(self.shape_horizontal, self.shape_vertical),
        ]
        .into_iter()
    }
}

fn next_vertical(value: alignment::Vertical) -> alignment::Vertical {
    match value {
        alignment::Vertical::Top => alignment::Vertical::Center,
        alignment::Vertical::Center => alignment::Vertical::Bottom,
        alignment::Vertical::Bottom => alignment::Vertical::Top,
    }
}

fn next_horizontal(value: alignment::Horizontal) -> alignment::Horizontal {
    match value {
        alignment::Horizontal::Left => alignment::Horizontal::Center,
        alignment::Horizontal::Center => alignment::Horizontal::Right,
        alignment::Horizontal::Right => alignment::Horizontal::Left,
    }
}

fn vertical_label(value: alignment::Vertical) -> &'static str {
    match value {
        alignment::Vertical::Top => "Top",
        alignment::Vertical::Center => "Center",
        alignment::Vertical::Bottom => "Bottom",
    }
}

fn horizontal_label(value: alignment::Horizontal) -> &'static str {
    match value {
        alignment::Horizontal::Left => "Left",
        alignment::Horizontal::Center => "Center",
        alignment::Horizontal::Right => "Right",
    }
}

fn plot_widget() -> PlotWidget {
    let line = Series::line_only(
        (0..240)
            .map(|i| {
                let x = i as f64 / 239.0 * TAU;
                [x, x.sin()]
            })
            .collect(),
        LineStyle::solid().with_pixel_width(2.0),
    )
    .with_label("sin(x)")
    .with_color(Color::from_rgb(0.2, 0.55, 0.9));

    let peak = Series::markers_only(vec![[FRAC_PI_2, 1.0]], MarkerStyle::circle(6.0))
        .with_label("peak")
        .with_color(Color::from_rgb(0.95, 0.25, 0.25));

    let trough = Series::markers_only(vec![[FRAC_PI_2 * 3.0, -1.0]], MarkerStyle::circle(6.0))
        .with_label("trough")
        .with_color(Color::from_rgb(0.95, 0.45, 0.15));

    PlotWidgetBuilder::new()
        .with_x_label("x")
        .with_y_label("sin(x)")
        .with_x_lim(0.0, TAU)
        .with_y_lim(-1.25, 1.25)
        .add_series(line)
        .add_series(peak)
        .add_series(trough)
        .with_cursor_overlay(true)
        .build()
        .unwrap()
}

fn region_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Color::from_rgba(0.2, 0.8, 0.55, 0.14).into()),
        text_color: Some(palette.background.base.text.scale_alpha(0.65)),
        border: border::rounded(4)
            .width(1)
            .color(Color::from_rgba(0.2, 0.8, 0.55, 0.45)),
        ..container::Style::default()
    }
}

fn annotation_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.primary.weak.color.scale_alpha(0.3).into()),
        text_color: Some(palette.primary.weak.text),
        border: border::rounded(4)
            .width(1)
            .color(palette.primary.strong.color),
        ..container::Style::default()
    }
}
