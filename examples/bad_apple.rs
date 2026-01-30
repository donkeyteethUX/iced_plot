//! A totally necessary example.
use iced_plot::{
    Color, MarkerStyle, MarkerType, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series, ShapeId,
};

use iced::Element;
use iced::time::Instant;
use iced::window;
use std::time::Duration;

const FRAME_WIDTH: usize = 120;
const FRAME_HEIGHT: usize = 90;
const LABEL: &str = "bad_apple";
const FRAME_RATE: f64 = 12.0;

fn main() -> iced::Result {
    iced::application::timed(App::new, App::update, App::subscription, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .theme(iced::theme::Theme::KanagawaDragon)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    PlotMessage(PlotUiMessage),
    Tick,
}

struct App {
    series_id: ShapeId,
    widget: PlotWidget,
    frames: &'static [u8],
    frame_idx: usize,
    frame_count: usize,
    last_tick: Option<Instant>,
    accumulator: Duration,
    frame_duration: Duration,
}

impl App {
    fn new() -> Self {
        let frames = include_bytes!("assets/bad_apple_gray.bin");
        let pixels_per_frame = FRAME_WIDTH * FRAME_HEIGHT;
        let frame_count = frames.len() / pixels_per_frame;
        let positions = build_positions(FRAME_WIDTH, FRAME_HEIGHT);
        let initial_colors = if frame_count > 0 {
            frame_colors(0, frames, FRAME_WIDTH, FRAME_HEIGHT)
        } else {
            vec![Color::from_rgb(0.0, 0.0, 0.0); positions.len()]
        };
        let series = Series::markers_only(
            positions.clone(),
            MarkerStyle::new_world(1.0, MarkerType::Square),
        )
        .with_label(LABEL)
        .with_point_colors(initial_colors);
        let series_id = series.id;
        let widget = PlotWidgetBuilder::new()
            .with_data_aspect(1.0)
            .with_x_tick_labels(false)
            .with_y_tick_labels(false)
            .with_tooltips(false)
            .with_cursor_overlay(false)
            .add_series(series)
            .build()
            .unwrap();

        Self {
            series_id,
            widget,
            frames,
            frame_idx: 0,
            frame_count,
            last_tick: None,
            accumulator: Duration::ZERO,
            frame_duration: Duration::from_secs_f64(1.0 / FRAME_RATE),
        }
    }

    fn update(&mut self, message: Message, now: Instant) {
        match message {
            Message::PlotMessage(plot_msg) => {
                self.widget.update(plot_msg);
            }
            Message::Tick => {
                if self.frame_count == 0 {
                    return;
                }
                let elapsed = match self.last_tick {
                    Some(prev) => now.saturating_duration_since(prev),
                    None => Duration::ZERO,
                };
                self.last_tick = Some(now);
                self.accumulator += elapsed;

                let mut steps = 0usize;
                while self.accumulator >= self.frame_duration {
                    self.accumulator -= self.frame_duration;
                    steps += 1;
                }
                if steps > 0 {
                    self.frame_idx = (self.frame_idx + steps) % self.frame_count;
                    self.update_frame();
                }
            }
        }
    }

    fn update_frame(&mut self) {
        if self.frame_count == 0 {
            return;
        }

        let colors = frame_colors(self.frame_idx, self.frames, FRAME_WIDTH, FRAME_HEIGHT);
        self.widget.set_series_point_colors(&self.series_id, colors);
    }

    fn view(&self) -> Element<'_, Message> {
        self.widget.view().map(Message::PlotMessage)
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        window::frames().map(|_| Message::Tick)
    }
}

fn build_positions(width: usize, height: usize) -> Vec<[f64; 2]> {
    let mut positions = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let py = (height - 1 - y) as f64;
            positions.push([x as f64, py]);
        }
    }
    positions
}

fn frame_colors(frame_idx: usize, frames: &[u8], width: usize, height: usize) -> Vec<Color> {
    let pixels_per_frame = width * height;
    let start = frame_idx * pixels_per_frame;
    let end = start + pixels_per_frame;
    frames[start..end]
        .iter()
        .map(|&value| {
            let g = value as f32 / 255.0;
            Color::from_rgb(g, g, g)
        })
        .collect()
}
