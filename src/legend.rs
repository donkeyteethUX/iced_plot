use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Container, button, column, container, row, text};
use iced::{Border, Element, Length, color};

use crate::message::PlotUiMessage;
use crate::widget::PlotData;

const LEGEND_PADDING: f32 = 8.0;
const ENTRY_SPACING: f32 = 6.0;
const COLUMN_SPACING: f32 = 4.0;
const LABEL_TEXT_SIZE: f32 = 14.0;
const SWATCH_SIZE: f32 = 14.0;

pub(crate) fn legend(data: &PlotData, collapsed: bool) -> Element<'_, PlotUiMessage> {
    let entries = data.legend_entries();

    if entries.is_empty() {
        return legend_container(label_button("Legend")).into();
    } else if collapsed {
        return legend_container(label_button("▶ Legend")).into();
    }

    let mut col = column![label_button("▼ Legend")]
        .spacing(COLUMN_SPACING)
        .width(Length::Shrink)
        .height(Length::Shrink);

    for e in entries {
        let label_text = e.label.clone();
        let series_color = e.color;
        let swatch_color = if e.hidden {
            color!(120, 120, 120)
        } else {
            series_color
        };

        let swatch = container("")
            .width(Length::Fixed(SWATCH_SIZE))
            .height(Length::Fixed(SWATCH_SIZE))
            .style(move |_| swatch_color.into());

        let swatch_btn: Element<'_, PlotUiMessage> = button(swatch)
            .padding(2.0)
            .on_press(PlotUiMessage::ToggleSeriesVisibility(label_text.clone()))
            .into();

        let row = row![
            swatch_btn,
            text(label_text).size(LABEL_TEXT_SIZE).color(series_color)
        ]
        .spacing(ENTRY_SPACING)
        .width(Length::Shrink);

        col = col.push(row);
    }

    legend_container(col)
        .style(|theme| {
            container::rounded_box(theme)
                .background(color!(12, 12, 15, 0.55))
                .border(Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: color!(255, 255, 255, 0.08),
                })
        })
        .into()
}

fn label_button(label: &str) -> Element<'_, PlotUiMessage> {
    button(text(label).size(LABEL_TEXT_SIZE))
        .on_press(PlotUiMessage::ToggleLegend)
        .into()
}

fn legend_container<'a>(
    content: impl Into<Element<'a, PlotUiMessage>>,
) -> Container<'a, PlotUiMessage> {
    container(content)
        .padding(LEGEND_PADDING)
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top)
}
