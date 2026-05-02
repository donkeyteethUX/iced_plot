//! Auto-detect the plot backend and expose simple offset controls.
use iced::{
    Element, Length, Task, Theme, alignment, border,
    widget::{Space, column, container, row, slider, stack, text},
};
use iced_plot::{
    Color, HoverPickEvent, LineStyle, MarkerStyle, PlotRenderStrategy, PlotUiMessage, PlotWidget,
    PlotWidgetBuilder, Series,
};

const OFFSET_RANGE: std::ops::RangeInclusive<f32> = 0.0..=240.0;
const OFFSET_STEP: f32 = 4.0;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

struct App {
    widget: PlotWidget,
    render_strategy: PlotRenderStrategy,
    x_offset: f32,
    y_offset: f32,
}

#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
    RenderStrategyDetected(PlotRenderStrategy),
    XOffsetChanged(f32),
    YOffsetChanged(f32),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                widget: new_plot(),
                render_strategy: PlotRenderStrategy::default(),
                x_offset: 0.0,
                y_offset: 0.0,
            },
            iced::system::information().map(|information| {
                Message::RenderStrategyDetected(PlotRenderStrategy::from_graphics_backend(
                    &information.graphics_backend,
                ))
            }),
        )
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Plot(message) => update_plot(&mut self.widget, message),
            Message::RenderStrategyDetected(strategy) => {
                self.set_render_strategy(strategy);
            }
            Message::XOffsetChanged(offset) => {
                self.x_offset = offset;
            }
            Message::YOffsetChanged(offset) => {
                self.y_offset = offset;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let plot = column![
            Space::new().height(Length::Fixed(self.y_offset)),
            row![
                Space::new().width(Length::Fixed(self.x_offset)),
                container(self.widget.view().map(Message::Plot))
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let overlay = container(self.controls_overlay())
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .align_x(alignment::Horizontal::Left)
            .align_y(alignment::Vertical::Top)
            .style(container::transparent);

        stack![plot, overlay].into()
    }

    fn controls_overlay(&self) -> Element<'_, Message> {
        let panel = column![
            text(format!("Render Strategy: {:?}", self.render_strategy)).size(16),
            text(if matches!(self.render_strategy, PlotRenderStrategy::Canvas) {
                "iced_tiny_skia::Renderer has bug with canvas's offset, which is fixed by iced v0.15.0-dev"
            } else {
                "Run with CPU backend:\nICED_BACKEND=\"tiny-skia\" cargo run --features canvas --example auto_backend"
            })
            .style(text::secondary),
            offset_slider("x offset", self.x_offset, Message::XOffsetChanged),
            offset_slider("y offset", self.y_offset, Message::YOffsetChanged),
        ]
        .spacing(8);

        container(panel)
            .padding(12)
            .width(Length::Fixed(360.0))
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.weak.color.scale_alpha(0.86).into()),
                    text_color: Some(palette.background.weak.text),
                    border: border::rounded(8).color(palette.background.strong.color),
                    ..container::Style::default()
                }
            })
            .into()
    }

    fn set_render_strategy(&mut self, strategy: PlotRenderStrategy) {
        self.render_strategy = strategy;
        self.widget.set_render_strategy(strategy);
    }
}

fn offset_slider(
    label: &'static str,
    value: f32,
    on_change: fn(f32) -> Message,
) -> Element<'static, Message> {
    row![
        text(format!("{label}: {value:.0}px"))
            .width(Length::Fixed(150.0))
            .wrapping(text::Wrapping::None),
        slider(OFFSET_RANGE, value, on_change)
            .step(OFFSET_STEP)
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn update_plot(widget: &mut PlotWidget, message: PlotUiMessage) {
    let hover_pick_event = message.get_hover_pick_event();
    widget.update(message);

    match hover_pick_event {
        Some(HoverPickEvent::Hover(point_id)) => {
            if let Some([x, _]) = widget.point_position(point_id) {
                for series_id in widget.series_ids() {
                    if series_id != point_id.series_id
                        && let Some(p) = widget.nearest_point_horizontal(series_id, x)
                    {
                        widget.add_hover_point(p);
                    }
                }
            }
        }
        Some(HoverPickEvent::Pick(point_id)) => {
            if let Some([x, _]) = widget.point_position(point_id) {
                for series_id in widget.series_ids() {
                    if series_id != point_id.series_id
                        && let Some(p) = widget.nearest_point_horizontal(series_id, x)
                    {
                        widget.add_pick_point(p);
                    }
                }
            }
        }
        _ => {}
    }
}

fn new_plot() -> PlotWidget {
    let positions = (0..100)
        .map(|i| {
            let x = i as f64 * 0.1;
            let y = (x * 0.5).sin();
            [x, y]
        })
        .collect();

    let s1 = Series::line_only(positions, LineStyle::solid().with_pixel_width(4.0))
        .with_label("sine_line_only")
        .with_color(Color::from_rgb(0.3, 0.3, 0.9));

    let positions = (0..50)
        .map(|i| {
            let x = i as f64 * 0.2;
            let y = (x * 0.3).cos() + 0.5;
            [x, y]
        })
        .collect();
    let s2 = Series::markers_only(positions, MarkerStyle::circle(6.0))
        .with_label("cosine_markers_only (not pickable)")
        .with_pickable(false)
        .with_color(Color::from_rgb(0.9, 0.3, 0.3));

    let positions = (0..30)
        .map(|i| {
            let x = i as f64 * 0.3;
            let y = (x * 0.8).sin() - 0.5;
            [x, y]
        })
        .collect();
    let s3 = Series::new(positions, MarkerStyle::square(4.0), LineStyle::dashed(10.0))
        .with_label("both_markers_and_lines")
        .with_color(Color::from_rgb(0.3, 0.9, 0.3));

    PlotWidgetBuilder::new()
        .with_hover_highlight_provider(|context, point| {
            if point.marker_style.is_none() {
                point.marker_style = Some(MarkerStyle::circle(6.0));
            }
            Some(format!(
                "Index: {}\nX: {:.2}\nY: {:.2}",
                context.point_index, point.x, point.y
            ))
        })
        .with_pick_highlight_provider(|ctx, point| {
            if point.marker_style.is_none() {
                // set plot 1 to star in pick highlight
                point.marker_style = Some(MarkerStyle::triangle(6.0));
            }
            point.mask_padding = None;
            point.resize_marker(1.5);
            point.color = Color::from_rgb(1.0, 0.0, 0.0);
            Some(format!(
                "Index: {}\nX: {:.2}\nY: {:.2}\n(Selected)",
                ctx.point_index, point.x, point.y
            ))
        })
        .add_series(s1)
        .add_series(s2)
        .add_series(s3)
        .with_cursor_overlay(true)
        .with_cursor_provider(|x, y| format!("Your cursor is at: X: {x:.2}, Y: {y:.2}"))
        .with_y_label("Y Axis (Custom Font Size)")
        .with_x_label("X Axis (Custom Font Size)")
        .with_x_tick_formatter(|tick| format!("{:.1}s", tick.value))
        .with_tick_label_size(12.0)
        .with_axis_label_size(18.0)
        .with_crosshairs(true)
        .build()
        .unwrap()
}
