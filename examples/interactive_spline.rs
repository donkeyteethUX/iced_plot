//! Interactive spline editor: click to add control points, drag to move them.
use iced::{Element, mouse};
use iced_plot::{
    Color, InputPolicy, LineStyle, MarkerStyle, PlotCommand, PlotCoordinateSnapshot,
    PlotInputEvent, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series, ShapeId,
};

fn main() -> iced::Result {
    iced::application(new_app, update, view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .theme(iced::theme::Theme::SolarizedDark)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
}

struct App {
    plot: PlotWidget,
    control_points: Vec<[f64; 2]>,
    drag_index: Option<usize>,
    control_series_id: ShapeId,
    spline_series_id: ShapeId,
    snapshot: Option<PlotCoordinateSnapshot>,
}

fn new_app() -> App {
    let control_points = vec![[0.0, 0.0], [1.0, 0.5], [2.0, 0.2], [3.0, 1.2], [4.0, 0.8]];

    let control_series = Series::markers_only(control_points.clone(), MarkerStyle::circle(8.0))
        .with_color(Color::from_rgb(0.9, 0.5, 0.2))
        .with_label("control points");

    let spline_series = Series::line_only(build_spline(&control_points), LineStyle::Solid)
        .with_color(Color::from_rgb(0.2, 0.7, 1.0))
        .with_label("spline");

    let control_series_id = control_series.id;
    let spline_series_id = spline_series.id;

    let plot = PlotWidgetBuilder::new()
        .with_x_label("time")
        .with_y_label("temperature")
        .with_input_policy(InputPolicy::Override)
        .add_series(spline_series)
        .add_series(control_series)
        .build()
        .unwrap();

    App {
        plot,
        control_points,
        drag_index: None,
        control_series_id,
        spline_series_id,
        snapshot: None,
    }
}

fn update(app: &mut App, message: Message) {
    match message {
        Message::Plot(msg) => {
            if let PlotUiMessage::Event(event) = &msg {
                if let Some(snapshot) = event
                    .render
                    .as_ref()
                    .and_then(|render| render.camera_bounds)
                {
                    app.snapshot = Some(snapshot);
                }
                if let Some(input) = &event.input {
                    handle_input(app, input);
                }
            }
            app.plot.update(msg);
        }
    }
}

fn view(app: &App) -> Element<'_, Message> {
    app.plot.view().map(Message::Plot)
}

fn handle_input(app: &mut App, input: &PlotInputEvent) {
    match input {
        PlotInputEvent::ButtonPressed {
            button: mouse::Button::Left,
            pointer,
        } => {
            if pointer.inside
                && let Some(world) = pointer.world
            {
                if let Some(index) = find_nearest_control_point(app, pointer) {
                    app.drag_index = Some(index);
                } else {
                    app.control_points.push(world);
                    app.drag_index = Some(app.control_points.len() - 1);
                    sync_series(app);
                }
            }
        }
        PlotInputEvent::ButtonReleased {
            button: mouse::Button::Left,
            ..
        } => {
            app.drag_index = None;
        }
        PlotInputEvent::CursorMoved(pointer) => {
            if let Some(index) = app.drag_index
                && let Some(world) = pointer.world
                && let Some(p) = app.control_points.get_mut(index)
            {
                *p = world;
                sync_series(app);
            }
        }
        PlotInputEvent::WheelScrolled { .. } => {
            // Forward wheel interactions to default zoom/pan behavior.
            app.plot
                .update(PlotUiMessage::Command(PlotCommand::ApplyDefaultMouseEvent(
                    input.clone(),
                )));
        }
        _ => {}
    }
}

fn find_nearest_control_point(app: &App, pointer: &iced_plot::PlotPointerEvent) -> Option<usize> {
    let snapshot = app.snapshot.as_ref()?;
    let threshold_px = 10.0f32;
    let mut best: Option<(usize, f32)> = None;
    for (index, point) in app.control_points.iter().enumerate() {
        if let Some(screen) = snapshot.world_to_screen(*point) {
            let dx = screen[0] - pointer.local[0];
            let dy = screen[1] - pointer.local[1];
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= threshold_px && best.map(|(_, best_dist)| dist < best_dist).unwrap_or(true) {
                best = Some((index, dist));
            }
        }
    }
    best.map(|(index, _)| index)
}

fn sync_series(app: &mut App) {
    let spline = build_spline(&app.control_points);
    let _ = app.plot.update_series(&app.spline_series_id, |series| {
        series.positions = spline.clone();
    });
    let _ = app.plot.update_series(&app.control_series_id, |series| {
        series.positions = app.control_points.clone();
    });
}

fn build_spline(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if points.len() < 2 {
        return points.to_vec();
    }

    let mut output = Vec::new();
    let segments_per = 16;

    for i in 0..(points.len() - 1) {
        let p0 = if i == 0 { points[i] } else { points[i - 1] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() {
            points[i + 2]
        } else {
            points[i + 1]
        };

        for s in 0..=segments_per {
            let t = s as f64 / segments_per as f64;
            output.push(catmull_rom(p0, p1, p2, p3, t));
        }
    }

    output
}

fn catmull_rom(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
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
