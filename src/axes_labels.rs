use iced::alignment::{Horizontal, Vertical};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, text};
use iced::{Element, Length};

/// Stack the element with the labels on the bottom and left.
pub(crate) fn stack_with_labels<'a, M: 'a>(
    widget: impl Into<Element<'a, M>>,
    x_label: &'a str,
    y_label: &'a str,
    axis_label_size: f32,
) -> Element<'a, M> {
    if x_label.is_empty() && y_label.is_empty() {
        widget.into()
    } else if x_label.is_empty() {
        row![y_axis_label(y_label, axis_label_size), widget.into()].into()
    } else if y_label.is_empty() {
        column![widget.into(), x_axis_label(x_label, axis_label_size)].into()
    } else {
        row![
            y_axis_label(y_label, axis_label_size),
            column![widget.into(), x_axis_label(x_label, axis_label_size)]
        ]
        .into()
    }
}

fn x_axis_label<'a, M: 'a>(label: &'a str, size: f32) -> Element<'a, M> {
    container(text(label).size(size))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Bottom)
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
}

fn y_axis_label<'a, M: 'a>(label: &'a str, size: f32) -> Element<'a, M> {
    container(text(label).size(size).wrapping(Wrapping::Word))
        .align_x(Horizontal::Left)
        .align_y(Vertical::Center)
        .width(Length::Shrink)
        .max_width(100.0)
        .height(Length::Fill)
        .into()
}
