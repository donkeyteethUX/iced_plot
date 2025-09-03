use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, text};
use iced::{Color, Element, Length};

/// Stack the element with the labels on the bottom and left.
pub fn stack_with_labels<'a, M: 'a>(
    widget: impl Into<Element<'a, M>>,
    x_label: &'a str,
    y_label: &'a str,
) -> Element<'a, M> {
    if x_label.is_empty() && y_label.is_empty() {
        widget.into()
    } else if x_label.is_empty() {
        row![y_axis_label(y_label), widget.into()].into()
    } else if y_label.is_empty() {
        column![widget.into(), x_axis_label(x_label)].into()
    } else {
        row![
            y_axis_label(y_label),
            column![widget.into(), x_axis_label(x_label)]
        ]
        .into()
    }
}

fn x_axis_label<'a, M: 'a>(label: &'a str) -> Element<'a, M> {
    container(text(label).size(16.0).color(Color::WHITE))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Bottom)
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

fn y_axis_label<'a, M: 'a>(label: &'a str) -> Element<'a, M> {
    container(
        text(label)
            .size(16.0)
            .color(Color::WHITE)
            .wrapping(Wrapping::Word),
    )
    .align_x(Horizontal::Left)
    .align_y(Vertical::Center)
    .width(Length::Shrink)
    .max_width(100.0)
    .height(Length::Fill)
    .into()
}
