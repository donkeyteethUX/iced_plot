//! Show multiple plot widgets in a single application.
//! All three plots have their x-axes linked, so panning or zooming the x-axis
//! on one plot will synchronize the others.
use iced::padding;
use iced_plot::HighlightPoint;
use iced_plot::HoverPickEvent;
use iced_plot::PlotUiMessage;
use iced_plot::PlotWidget;
use iced_plot::PlotWidgetBuilder;
use iced_plot::TooltipContext;
use iced_plot::{AxisLink, LineStyle, MarkerStyle, Series, ShapeId};

use iced::Color;
use iced::Element;
use iced::Length;
use iced::widget::{column, scrollable};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

struct App {
    s1_id: ShapeId,
    s2_id: ShapeId,
    s3_id: ShapeId,
    w1: PlotWidget,
    w2: PlotWidget,
    w3: PlotWidget,
}

#[derive(Debug)]
struct Message {
    msg: PlotUiMessage,
    plot_id: usize,
}

impl App {
    fn update(&mut self, Message { msg, plot_id }: Message) {
        match plot_id {
            1 => {
                match msg.get_hover_pick_event() {
                    Some(HoverPickEvent::Hover(p1)) => {
                        if let Some([x, _]) = self.w1.point_position(p1) {
                            if let Some(p2) = self.w2.nearest_point_horizontal(self.s2_id, x) {
                                self.w2.add_hover_point(p2);
                            }
                            if let Some(p3) = self.w3.nearest_point_horizontal(self.s3_id, x) {
                                self.w3.add_hover_point(p3);
                            }
                        }
                    }
                    Some(HoverPickEvent::Pick(p1)) => {
                        if let Some([x, _]) = self.w1.point_position(p1) {
                            if let Some(p2) = self.w2.nearest_point_horizontal(self.s2_id, x) {
                                self.w2.add_pick_point(p2);
                            }
                            if let Some(p3) = self.w3.nearest_point_horizontal(self.s3_id, x) {
                                self.w3.add_pick_point(p3);
                            }
                        }
                    }
                    Some(HoverPickEvent::ClearPick) => {
                        self.w2.clear_pick();
                        self.w3.clear_pick();
                    }
                    _ => {}
                }
                self.w1.update(msg);
            }
            2 => {
                match msg.get_hover_pick_event() {
                    Some(HoverPickEvent::Hover(p2)) => {
                        if let Some([x, _]) = self.w2.point_position(p2) {
                            if let Some(p1) = self.w1.nearest_point_horizontal(self.s1_id, x) {
                                self.w1.add_hover_point(p1);
                            }
                            if let Some(p3) = self.w3.nearest_point_horizontal(self.s3_id, x) {
                                self.w3.add_hover_point(p3);
                            }
                        }
                    }
                    Some(HoverPickEvent::Pick(p2)) => {
                        if let Some([x, _]) = self.w2.point_position(p2) {
                            if let Some(p1) = self.w1.nearest_point_horizontal(self.s1_id, x) {
                                self.w1.add_pick_point(p1);
                            }
                            if let Some(p3) = self.w3.nearest_point_horizontal(self.s3_id, x) {
                                self.w3.add_pick_point(p3);
                            }
                        }
                    }
                    Some(HoverPickEvent::ClearPick) => {
                        self.w1.clear_pick();
                        self.w3.clear_pick();
                    }
                    _ => {}
                }
                self.w2.update(msg);
            }
            3 => {
                match msg.get_hover_pick_event() {
                    Some(HoverPickEvent::Hover(p3)) => {
                        if let Some([x, _]) = self.w3.point_position(p3) {
                            if let Some(p1) = self.w1.nearest_point_horizontal(self.s1_id, x) {
                                self.w1.add_hover_point(p1);
                            }
                            if let Some(p2) = self.w2.nearest_point_horizontal(self.s2_id, x) {
                                self.w2.add_hover_point(p2);
                            }
                        }
                    }
                    Some(HoverPickEvent::Pick(p3)) => {
                        if let Some([x, _]) = self.w3.point_position(p3) {
                            if let Some(p1) = self.w1.nearest_point_horizontal(self.s1_id, x) {
                                self.w1.add_pick_point(p1);
                            }
                            if let Some(p2) = self.w2.nearest_point_horizontal(self.s2_id, x) {
                                self.w2.add_pick_point(p2);
                            }
                        }
                    }
                    Some(HoverPickEvent::ClearPick) => {
                        self.w1.clear_pick();
                        self.w2.clear_pick();
                    }
                    _ => {}
                }
                self.w3.update(msg)
            }
            _ => {}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        scrollable(
            column![
                self.w1.view().map(|msg| Message { msg, plot_id: 1 }),
                self.w2.view().map(|msg| Message { msg, plot_id: 2 }),
                self.w3.view().map(|msg| Message { msg, plot_id: 3 }),
            ]
            .padding(padding::right(10.0))
            .height(Length::Fixed(1200.0)),
        )
        .into()
    }

    fn new() -> Self {
        // Create a shared x-axis link so all three plots pan/zoom together on the x-axis
        let x_link = AxisLink::new();

        let positions = (0..100)
            .map(|i| {
                let x = i as f64 * 0.1;
                let y = (x * 0.5).sin();
                [x, y]
            })
            .collect();
        let s1 = Series::line_only(positions, LineStyle::Solid).with_label("sine_line_only");
        let s1_id = s1.id;
        let w1 = PlotWidgetBuilder::new()
            .disable_scroll_to_pan()
            .with_hover_highlight_provider(Self::hover_highlight_provider)
            .with_pick_highlight_provider(Self::pick_highlight_provider)
            .with_x_lim(-1.0, 10.0) // Set x-axis limits
            .with_y_lim(-2.0, 2.0) // Set y-axis limits
            .with_x_axis_link(x_link.clone()) // Link the x-axis
            .add_series(s1)
            .build()
            .unwrap();

        let positions = (0..50)
            .map(|i| {
                let x = i as f64 * 0.2;
                let y = (x * 0.3).cos() + 0.5;
                [x, y]
            })
            .collect();
        let s2 = Series::markers_only(positions, MarkerStyle::circle(6.0))
            .with_label("cosine_markers_only")
            .with_color([0.9, 0.3, 0.3]);
        let s2_id = s2.id;
        let w2 = PlotWidgetBuilder::new()
            .disable_scroll_to_pan()
            .with_hover_highlight_provider(Self::hover_highlight_provider)
            .with_pick_highlight_provider(Self::pick_highlight_provider)
            .with_x_axis_link(x_link.clone()) // Link the x-axis
            .with_x_tick_formatter(|_| String::new()) // Remove tick labels
            .with_y_tick_formatter(|_| String::new())
            .add_series(s2)
            .build()
            .unwrap();

        let positions = (0..30)
            .map(|i| {
                let x = i as f64 * 0.3;
                let y = (x * 0.8).sin() - 0.5;
                [x, y]
            })
            .collect();
        let s3 = Series::new(
            positions,
            MarkerStyle::square(4.0),
            LineStyle::Dashed { length: 10.0 },
        )
        .with_label("both_markers_and_lines")
        .with_color([0.3, 0.9, 0.3]);
        let s3_id = s3.id;
        let w3 = PlotWidgetBuilder::new()
            .disable_scroll_to_pan()
            .with_hover_highlight_provider(Self::hover_highlight_provider)
            .with_pick_highlight_provider(Self::pick_highlight_provider)
            .with_x_axis_link(x_link.clone()) // Link the x-axis
            .add_series(s3)
            .without_grid() // Disable grid lines and ticks
            .build()
            .unwrap();

        Self {
            w1,
            w2,
            w3,
            s1_id,
            s2_id,
            s3_id,
        }
    }
    fn hover_highlight_provider(ctx: TooltipContext, point: &mut HighlightPoint) -> Option<String> {
        if point.marker_style.is_none() {
            // set plot 1 to star in hover highlight
            point.marker_style = Some(MarkerStyle::square(6.0));
        }
        Some(format!(
            "Index: {}\nX: {:.2}\nY: {:.2}",
            ctx.point_index, point.x, point.y
        ))
    }
    fn pick_highlight_provider(ctx: TooltipContext, point: &mut HighlightPoint) -> Option<String> {
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
    }
}
