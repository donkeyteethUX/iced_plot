use fastplot::message::PlotUiMessage;
use fastplot::widget::PlotWidget;
use fastplot::{Color, LineStyle, MarkerStyle, MarkerType, Series};

use iced::keyboard;
use iced::time::Instant;

use iced::window;
use iced::{Element, Event, Subscription};

use rand_distr::{Distribution, Normal};

fn main() -> iced::Result {
    iced::application(IcedPlot::default, IcedPlot::update, IcedPlot::view)
        .subscription(IcedPlot::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Tick(Instant),
    KeyEvent(keyboard::Event),
    PlotUi(PlotUiMessage),
}

struct IcedPlot {
    plot_scene: PlotWidget,
    animating: bool,
    phase: f32,
    cosine_xs: Vec<f32>,
}

impl IcedPlot {
    fn update(&mut self, message: Message) {
        match message {
            Message::Tick(_time) => {
                if self.animating {
                    self.phase += 0.02;
                    // Recompute cosine series positions with phase shift
                    let mut positions = Vec::with_capacity(self.cosine_xs.len());
                    for &x in &self.cosine_xs {
                        let y = (x * 1.2 + self.phase).cos();
                        positions.push([x, y]);
                    }

                    self.plot_scene.set_series_positions("cosine", &positions);
                }
            }
            Message::KeyEvent(event) => {
                if let keyboard::Event::KeyPressed { key, .. } = event
                    && key == keyboard::Key::Named(keyboard::key::Named::Space)
                {
                    self.animating = !self.animating;
                }
            }
            Message::PlotUi(plot_msg) => {
                self.plot_scene.update(plot_msg);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.plot_scene.view().map(Message::PlotUi)
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::batch(vec![
            window::frames().map(Message::Tick),
            iced::event::listen().map(|event| match event {
                Event::Keyboard(event) => Message::KeyEvent(event),
                _ => Message::Tick(Instant::now()),
            }),
        ])
    }
}

impl Default for IcedPlot {
    fn default() -> Self {
        let mut plot_scene = PlotWidget::new();

        // Create example data
        let mut rng = rand::rng();
        let n = 500usize;
        let x_min = -10.0f32;
        let x_max = 10.0f32;
        let mut positions = Vec::with_capacity(n);
        let mut cosine_xs = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f32 / (n - 1) as f32;
            let x = x_min + t * (x_max - x_min);
            let base_y = (x * 1.2).cos();
            let y = base_y;
            positions.push([x, y]);
            cosine_xs.push(x);
        }
        let _ = plot_scene.add_series(Series {
            label: Some("cosine".to_string()),
            positions,
            marker_style: Some(MarkerStyle {
                color: Color::from_rgb(0.3, 0.8, 0.9),
                size: 8.0,
                marker_type: MarkerType::Star,
            }),
            line_style: Some(LineStyle::Dashed { length: 20.0 }),
        });

        // Add a scatter dataset similar to the standalone scatter example.
        // Use a moderate number of points so the iced example stays responsive.
        let n_scatter = 1_000_000usize;
        let x_min = -20.0f32;
        let x_max = 20.0f32;
        let normal_sc = Normal::new(0.0f32, 0.15f32).unwrap();
        let mut buckets: [Vec<[f32; 2]>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for i in 0..n_scatter {
            let t = i as f32 / (n_scatter - 1) as f32;
            let x = x_min + t * (x_max - x_min);
            let base = (x * 0.8).sin();
            let y = base + normal_sc.sample(&mut rng);
            buckets[i % 4].push([x, y]);
        }

        let styles = [
            (
                "sine_a",
                Color::from_rgb(0.9, 0.3, 0.3),
                MarkerType::FilledCircle,
            ),
            (
                "sine_b",
                Color::from_rgb(0.3, 0.9, 0.3),
                MarkerType::EmptyCircle,
            ),
            (
                "sine_c",
                Color::from_rgb(0.3, 0.3, 0.9),
                MarkerType::Triangle,
            ),
            ("sine_d", Color::from_rgb(0.9, 0.8, 0.2), MarkerType::Star),
        ];

        for (i, (label, color, marker)) in styles.iter().enumerate() {
            let _ = plot_scene.add_series(Series {
                label: Some((*label).to_string()),
                positions: buckets[i].clone(),
                marker_style: Some(MarkerStyle {
                    color: *color,
                    size: 6.0,
                    marker_type: *marker,
                }),
                line_style: None,
            });
        }

        Self {
            plot_scene,
            animating: true,
            phase: 0.0,
            cosine_xs,
        }
    }
}
