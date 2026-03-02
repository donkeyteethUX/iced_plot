//! Interactive spline demo.
//!
//! Demonstrates selectively disabling plot controls and handling drag events for custom interactivity.
use iced::{
    Element,
    widget::{column, text},
};
use iced_plot::{
    Color, DragEvent, LineStyle, MarkerStyle, PanControls, PlotControls, PlotUiMessage, PlotWidget,
    PlotWidgetBuilder, Series, ShapeId,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
}

struct App {
    widget: PlotWidget,
    control_series_id: ShapeId,
    control_poly_id: ShapeId,
    spline_series_id: ShapeId,
    control_points: Vec<[f64; 2]>,
    active_control_point: Option<usize>,
}

impl App {
    fn new() -> Self {
        let control_points = vec![
            [-4.0, -0.5],
            [-2.0, 1.5],
            [0.0, -1.2],
            [2.5, 1.8],
            [4.0, 0.2],
        ];

        let control_poly =
            Series::line_only(control_points.clone(), LineStyle::Dashed { length: 8.0 })
                .with_label("control polygon")
                .with_color(Color::from_rgb(0.5, 0.5, 0.5));

        let control_series = Series::markers_only(control_points.clone(), MarkerStyle::circle(7.0))
            .with_label("control points")
            .with_color(Color::from_rgb(1.0, 0.5, 0.2));

        let spline = Series::line_only(sample_catmull_rom(&control_points, 28), LineStyle::Solid)
            .with_label("catmull-rom spline")
            .with_color(Color::from_rgb(0.2, 0.8, 1.0));

        let controls_cfg = PlotControls {
            pan: PanControls {
                scroll_to_pan: true,
                drag_to_pan: false,
            },
            ..Default::default()
        };

        let control_series_id = control_series.id;
        let control_poly_id = control_poly.id;
        let spline_series_id = spline.id;

        let widget = PlotWidgetBuilder::new()
            .with_controls(controls_cfg)
            .with_cursor_overlay(true)
            .with_x_label("x")
            .with_y_label("y")
            .add_series(control_poly)
            .add_series(spline)
            .add_series(control_series)
            .build()
            .unwrap();

        Self {
            widget,
            control_series_id,
            control_poly_id,
            spline_series_id,
            control_points,
            active_control_point: None,
        }
    }

    fn update(&mut self, message: Message) {
        const DRAG_PICK_RADIUS_WORLD: f64 = 0.5;

        match message {
            Message::Plot(plot_msg) => {
                let mut drag_event = None;

                if let PlotUiMessage::RenderUpdate(update) = &plot_msg {
                    drag_event = update.drag_event;
                }

                self.widget.update(plot_msg);

                if let Some(drag_event) = drag_event {
                    match drag_event {
                        DragEvent::Start { world } => {
                            self.active_control_point = nearest_control_point_index(
                                &self.control_points,
                                world,
                                DRAG_PICK_RADIUS_WORLD,
                            );
                            self.move_control_point(world);
                        }
                        DragEvent::Update { world } => {
                            self.move_control_point(world);
                        }
                        DragEvent::End { .. } => {
                            self.active_control_point = None;
                        }
                    }
                }
            }
        }
    }

    fn move_control_point(&mut self, world: [f64; 2]) {
        if let Some(index) = self.active_control_point
            && index < self.control_points.len()
        {
            self.control_points[index] = world;
            let spline = sample_catmull_rom(&self.control_points, 28);
            self.widget
                .set_series_positions(&self.control_series_id, &self.control_points);
            self.widget
                .set_series_positions(&self.control_poly_id, &self.control_points);
            self.widget
                .set_series_positions(&self.spline_series_id, &spline);
        }
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("Interactive spline: click and drag a control point.").size(16),
            self.widget.view().map(Message::Plot),
        ]
        .spacing(8)
        .into()
    }
}

fn sample_catmull_rom(control_points: &[[f64; 2]], samples_per_segment: usize) -> Vec<[f64; 2]> {
    if control_points.len() < 2 {
        return control_points.to_vec();
    }

    let mut out = Vec::new();

    for i in 0..(control_points.len() - 1) {
        let p0 = control_points[i.saturating_sub(1)];
        let p1 = control_points[i];
        let p2 = control_points[i + 1];
        let p3 = control_points[(i + 2).min(control_points.len() - 1)];

        for s in 0..samples_per_segment {
            let t = s as f64 / samples_per_segment as f64;
            out.push(catmull_rom_point(p0, p1, p2, p3, t));
        }
    }

    if let Some(last) = control_points.last().copied() {
        out.push(last);
    }

    out
}

fn catmull_rom_point(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let t2 = t * t;
    let t3 = t2 * t;

    let x = 0.5
        * ((2.0 * p1[0])
            + (-p0[0] + p2[0]) * t
            + (2.0 * p0[0] - 5.0 * p1[0] + 4.0 * p2[0] - p3[0]) * t2
            + (-p0[0] + 3.0 * p1[0] - 3.0 * p2[0] + p3[0]) * t3);

    let y = 0.5
        * ((2.0 * p1[1])
            + (-p0[1] + p2[1]) * t
            + (2.0 * p0[1] - 5.0 * p1[1] + 4.0 * p2[1] - p3[1]) * t2
            + (-p0[1] + 3.0 * p1[1] - 3.0 * p2[1] + p3[1]) * t3);

    [x, y]
}

fn nearest_control_point_index(
    control_points: &[[f64; 2]],
    world: [f64; 2],
    pick_radius_world: f64,
) -> Option<usize> {
    let max_distance_sq = pick_radius_world * pick_radius_world;

    control_points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a[0] - world[0]).powi(2) + (a[1] - world[1]).powi(2);
            let db = (b[0] - world[0]).powi(2) + (b[1] - world[1]).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .and_then(|(index, point)| {
            let distance_sq = (point[0] - world[0]).powi(2) + (point[1] - world[1]).powi(2);
            (distance_sq <= max_distance_sq).then_some(index)
        })
}
