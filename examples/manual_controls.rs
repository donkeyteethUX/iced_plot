//! Plot with override input policy, manually implementing default controls.
use iced::{Element, Theme, keyboard};
use iced_plot::{
    Color, InputPolicy, LineStyle, PlotCommand, PlotCoordinateSnapshot, PlotInputEvent,
    PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series,
};

fn main() -> iced::Result {
    iced::application(new_app, update, view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .theme(Theme::SolarizedDark)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
}

struct App {
    plot: PlotWidget,
    snapshot: Option<PlotCoordinateSnapshot>,
    dragging: bool,
    last_world: Option<[f64; 2]>,
    selection_start: Option<[f64; 2]>,
}

fn new_app() -> App {
    let positions = (0..200)
        .map(|i| {
            let x = i as f64 * 0.05;
            let y = (x * 0.6).sin();
            [x, y]
        })
        .collect();

    let series = Series::line_only(positions, LineStyle::Solid)
        .with_label("sine")
        .with_color(Color::from_rgb(0.2, 0.6, 1.0));

    let plot = PlotWidgetBuilder::new()
        .with_x_label("time")
        .with_y_label("value")
        .with_input_policy(InputPolicy::Override)
        .add_series(series)
        .build()
        .unwrap();

    App {
        plot,
        snapshot: None,
        dragging: false,
        last_world: None,
        selection_start: None,
    }
}

fn update(app: &mut App, message: Message) {
    match message {
        Message::Plot(plot_msg) => {
            if let PlotUiMessage::Event(event) = &plot_msg
                && let Some(input) = &event.input
            {
                if let Some(render) = &event.render
                    && let Some(snapshot) = render.camera_bounds
                {
                    app.snapshot = Some(snapshot);
                }
                handle_input(app, input);
            }
            app.plot.update(plot_msg);
        }
    }
}

fn view(app: &App) -> Element<'_, Message> {
    app.plot.view().map(Message::Plot)
}

fn handle_input(app: &mut App, input: &PlotInputEvent) {
    // Update cursor state without built-in interactions.
    app.plot.update(PlotUiMessage::Command(PlotCommand::ApplyInputEvent {
        input: *input,
        interactions_enabled: false,
    }));

    match input {
        PlotInputEvent::CursorMoved(pointer) => {
            if app.dragging
                && let Some(world) = pointer.world
                && let Some(last_world) = app.last_world
            {
                let delta = [last_world[0] - world[0], last_world[1] - world[1]];
                app.plot
                    .update(PlotUiMessage::Command(PlotCommand::PanByWorld { delta }));
                app.last_world = Some(world);
            } else {
                app.last_world = pointer.world;
                app.plot.update(PlotUiMessage::Command(PlotCommand::RequestHover));
            }
        }
        PlotInputEvent::CursorEntered(pointer) => {
            app.last_world = pointer.world;
            app.plot.update(PlotUiMessage::Command(PlotCommand::RequestHover));
        }
        PlotInputEvent::CursorLeft(_) => {
            app.dragging = false;
            app.last_world = None;
            app.selection_start = None;
            app.plot.update(PlotUiMessage::Command(PlotCommand::ClearHover));
        }
        PlotInputEvent::ButtonPressed { button, pointer } => match button {
            iced::mouse::Button::Left => {
                app.dragging = true;
                app.last_world = pointer.world;
                app.plot.update(PlotUiMessage::Command(PlotCommand::RequestPick));
            }
            iced::mouse::Button::Right => {
                app.selection_start = pointer.world;
            }
            _ => {}
        },
        PlotInputEvent::ButtonReleased { button, pointer } => match button {
            iced::mouse::Button::Left => {
                app.dragging = false;
                app.last_world = pointer.world;
            }
            iced::mouse::Button::Right => {
                if let (Some(start), Some(end)) = (app.selection_start, pointer.world) {
                    let min = [start[0].min(end[0]), start[1].min(end[1])];
                    let max = [start[0].max(end[0]), start[1].max(end[1])];
                    app.plot.update(PlotUiMessage::Command(
                        PlotCommand::ZoomToWorldRect {
                            min,
                            max,
                            padding_frac: 0.02,
                        },
                    ));
                }
                app.selection_start = None;
            }
            _ => {}
        },
        PlotInputEvent::WheelScrolled { delta, pointer } => {
            let (x, y) = match delta {
                iced::mouse::ScrollDelta::Lines { x, y } => (*x as f64, *y as f64),
                iced::mouse::ScrollDelta::Pixels { x, y } => (*x as f64, *y as f64),
            };
            let ctrl = pointer.modifiers.contains(keyboard::Modifiers::CTRL);
            if ctrl {
                if let Some(world) = pointer.world {
                    let factor = if y > 0.0 { 0.95 } else { 1.05 };
                    app.plot.update(PlotUiMessage::Command(PlotCommand::ZoomBy {
                        factor,
                        anchor_world: Some(world),
                    }));
                }
            } else if let Some(snapshot) = app.snapshot {
                let world_pan_x =
                    -x * (snapshot.camera_half_extents[0] / (snapshot.bounds.width as f64 / 2.0));
                let world_pan_y =
                    y * (snapshot.camera_half_extents[1] / (snapshot.bounds.height as f64 / 2.0));
                app.plot.update(PlotUiMessage::Command(PlotCommand::PanByWorld {
                    delta: [world_pan_x, world_pan_y],
                }));
            }
        }
    }
}
