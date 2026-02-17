use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Container, button, column, container, row, text};
use iced::{Color, Element, Length, color};

use crate::LineStyle;
use crate::series::ShapeId;
use crate::{message::PlotUiMessage, plot_widget::PlotWidget};

#[derive(Debug, Clone)]
/// An entry in the plot legend.
pub(crate) struct LegendEntry {
    pub(crate) id: ShapeId,
    pub(crate) label: String,
    pub(crate) color: Color,
    pub(crate) _marker: u32,
    pub(crate) _line_style: Option<LineStyle>,
    pub(crate) hidden: bool,
}

pub(crate) fn legend(widget: &PlotWidget, collapsed: bool) -> Option<Element<'_, PlotUiMessage>> {
    let entries: Vec<LegendEntry> = widget.legend_entries();

    if entries.is_empty() {
        return None;
    } else if collapsed {
        return Some(legend_container(label_button("▶ Legend")).into());
    }

    let mut col = column![label_button("▼ Legend")]
        .spacing(4.0)
        .width(Length::Shrink)
        .height(Length::Shrink);

    for e in entries {
        let mut series_color = e.color;
        series_color.a = 1.0;
        let swatch_color = if e.hidden {
            color!(120, 120, 120)
        } else {
            series_color
        };

        let swatch = container("")
            .width(Length::Fixed(12.0))
            .height(Length::Fixed(12.0))
            .style(move |_| swatch_color.into());

        let swatch_btn: Element<'_, PlotUiMessage> = button(swatch)
            .padding(2.0)
            .on_press(PlotUiMessage::ToggleSeriesVisibility(e.id))
            .into();

        let row = row![swatch_btn, text(e.label).size(12.0).color(series_color)]
            .spacing(4.0)
            .width(Length::Shrink);

        col = col.push(row);
    }

    Some(legend_container(col).style(container::bordered_box).into())
}
fn label_button(label: &str) -> Element<'_, PlotUiMessage> {
    button(text(label).size(12.0))
        .on_press(PlotUiMessage::ToggleLegend)
        .into()
}

fn legend_container<'a>(
    content: impl Into<Element<'a, PlotUiMessage>>,
) -> Container<'a, PlotUiMessage> {
    container(content)
        .padding(4.0)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top)
}
