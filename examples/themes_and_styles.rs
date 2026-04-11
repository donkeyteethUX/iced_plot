//! Demonstrates theme and style configuration.
use iced::{
    Border, Color, Element, Length, Theme,
    widget::{column, container, pick_list, row, text},
};
use iced_plot::{
    GridStyle, LineStyle, MarkerStyle, PlotStyle, PlotUiMessage, PlotWidget, PlotWidgetBuilder,
    Series, default_style,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
    ThemeSelected(ThemeChoice),
    StyleSelected(StyleChoice),
}

struct App {
    widget: PlotWidget,
    theme_choice: ThemeChoice,
    style_choice: StyleChoice,
}

impl App {
    fn new() -> Self {
        let theme_choice = ThemeChoice::TokyoNightStorm;
        let style_choice = StyleChoice::Default;

        Self {
            widget: plot_widget(style_choice),
            theme_choice,
            style_choice,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Plot(message) => self.widget.update(message),
            Message::ThemeSelected(theme_choice) => {
                self.theme_choice = theme_choice;
            }
            Message::StyleSelected(style_choice) => {
                self.style_choice = style_choice;
                self.widget.set_style(style_choice.style_fn());
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let toolbar = container(
            column![
                text("Application theme and plot style can be configured independently.")
                    .size(16.0),
                row![
                    control(
                        "App theme",
                        ThemeChoice::ALL,
                        self.theme_choice,
                        Message::ThemeSelected
                    ),
                    control(
                        "Plot style",
                        StyleChoice::ALL,
                        self.style_choice,
                        Message::StyleSelected
                    ),
                ]
                .spacing(16.0),
            ]
            .spacing(10.0),
        )
        .padding(12.0)
        .width(Length::Fill);

        column![toolbar, self.widget.view().map(Message::Plot)]
            .spacing(12.0)
            .padding(12.0)
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme_choice.theme()
    }
}

fn control<'a, T: Copy + Eq + std::fmt::Display + 'a>(
    label: &'a str,
    options: &'a [T],
    selected: T,
    on_selected: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message> {
    column![
        text(label).size(14.0),
        pick_list(options, Some(selected), on_selected).width(Length::Fixed(220.0)),
    ]
    .spacing(6.0)
    .into()
}

fn plot_widget(style_choice: StyleChoice) -> PlotWidget {
    let line = Series::line_only(
        (0..320)
            .map(|i| {
                let x = i as f64 * 0.04;
                [x, (x * 0.85).sin() + 0.2 * (x * 0.2).cos()]
            })
            .collect(),
        LineStyle::solid(),
    )
    .with_label("signal")
    .with_color(Color::from_rgb(0.2, 0.72, 0.98));

    let markers = Series::markers_only(
        (0..70)
            .map(|i| {
                let x = i as f64 * 0.18;
                [x, 0.65 * (x * 0.6).cos() - 0.1]
            })
            .collect(),
        MarkerStyle::square(5.5),
    )
    .with_label("samples")
    .with_color(Color::from_rgb(1.0, 0.55, 0.3));

    PlotWidgetBuilder::new()
        .with_x_label("X label")
        .with_cursor_overlay(true)
        .with_crosshairs(true)
        .with_style(style_choice.style_fn())
        .add_series(line)
        .add_series(markers)
        .build()
        .unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeChoice {
    Light,
    Dark,
    TokyoNightStorm,
    KanagawaDragon,
    SolarizedDark,
    Nord,
}

impl ThemeChoice {
    const ALL: &'static [Self] = &[
        Self::Light,
        Self::Dark,
        Self::TokyoNightStorm,
        Self::KanagawaDragon,
        Self::SolarizedDark,
        Self::Nord,
    ];

    fn theme(self) -> Theme {
        match self {
            Self::Light => Theme::Light,
            Self::Dark => Theme::Dark,
            Self::TokyoNightStorm => Theme::TokyoNightStorm,
            Self::KanagawaDragon => Theme::KanagawaDragon,
            Self::SolarizedDark => Theme::SolarizedDark,
            Self::Nord => Theme::Nord,
        }
    }
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::TokyoNightStorm => "Tokyo Night Storm",
            Self::KanagawaDragon => "Kanagawa Dragon",
            Self::SolarizedDark => "Solarized Dark",
            Self::Nord => "Nord",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleChoice {
    Default,
    Custom1,
    Custom2,
    Custom3,
}

impl StyleChoice {
    const ALL: &'static [Self] = &[Self::Default, Self::Custom1, Self::Custom2, Self::Custom3];

    fn style_fn(self) -> fn(&Theme) -> PlotStyle {
        match self {
            Self::Default => default_style,
            Self::Custom1 => style1,
            Self::Custom2 => style2,
            Self::Custom3 => style3,
        }
    }
}

impl std::fmt::Display for StyleChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Default => "Default",
            Self::Custom1 => "Custom 1",
            Self::Custom2 => "Custom 2",
            Self::Custom3 => "Custom 3",
        })
    }
}

fn style1(theme: &Theme) -> PlotStyle {
    let palette = theme.extended_palette();
    let base = default_style(theme);
    let controls_panel = styled_panel(
        base.controls_panel,
        with_alpha(palette.background.base.color, 0.94),
        with_alpha(palette.primary.weak.color, 0.7),
        8.0,
        1.0,
    );

    PlotStyle {
        frame: base.frame.background(palette.background.base.color),
        plot_area: styled_panel(
            base.plot_area,
            with_alpha(palette.background.weak.color, 0.5),
            with_alpha(palette.background.strong.color, 0.65),
            8.0,
            1.0,
        ),
        legend: styled_panel(
            base.legend,
            with_alpha(palette.background.base.color, 0.95),
            with_alpha(palette.secondary.strong.color, 0.55),
            8.0,
            1.0,
        ),
        controls_panel,
        cursor_overlay: controls_panel,
        tooltip: styled_panel(
            base.tooltip,
            with_alpha(palette.background.base.color, 0.9),
            with_alpha(palette.primary.base.color, 0.5),
            6.0,
            1.0,
        ),
        grid: grid_style(
            with_alpha(palette.secondary.strong.color, 0.7),
            with_alpha(palette.secondary.base.color, 0.5),
            with_alpha(palette.secondary.weak.color, 0.2),
        ),
        tick_label_color: palette.secondary.strong.color,
        axis_label_color: palette.primary.strong.color,
    }
}

fn style2(theme: &Theme) -> PlotStyle {
    let palette = theme.extended_palette();
    let base = default_style(theme);

    PlotStyle {
        frame: base
            .frame
            .background(with_alpha(palette.background.weakest.color, 0.95)),
        plot_area: styled_panel(
            base.plot_area,
            with_alpha(palette.background.base.color, 0.88),
            palette.primary.strong.color,
            10.0,
            2.0,
        ),
        legend: restyled_border(base.legend, palette.primary.strong.color, 10.0, 1.0),
        controls_panel: restyled_border(
            base.controls_panel,
            palette.success.strong.color,
            10.0,
            1.0,
        ),
        cursor_overlay: restyled_border(
            base.cursor_overlay,
            palette.secondary.strong.color,
            10.0,
            1.0,
        ),
        tooltip: restyled_border(base.tooltip, palette.warning.base.color, 8.0, 1.0),
        grid: grid_style(
            with_alpha(palette.primary.strong.color, 0.8),
            with_alpha(palette.success.base.color, 0.6),
            with_alpha(palette.secondary.base.color, 0.4),
        ),
        tick_label_color: palette.success.strong.color,
        axis_label_color: palette.primary.strong.color,
    }
}

fn style3(theme: &Theme) -> PlotStyle {
    let palette = theme.extended_palette();
    let base = default_style(theme);

    PlotStyle {
        frame: base
            .frame
            .background(with_alpha(palette.background.base.color, 0.98)),
        plot_area: styled_panel(
            base.plot_area,
            with_alpha(palette.primary.base.color, 0.11),
            palette.primary.strong.color,
            6.0,
            1.0,
        ),
        legend: styled_panel(
            base.legend,
            with_alpha(palette.secondary.base.color, 0.16),
            palette.secondary.strong.color,
            6.0,
            1.0,
        ),
        controls_panel: styled_panel(
            base.controls_panel,
            with_alpha(palette.success.base.color, 0.14),
            palette.success.strong.color,
            6.0,
            1.0,
        ),
        cursor_overlay: styled_panel(
            base.cursor_overlay,
            with_alpha(palette.warning.base.color, 0.18),
            palette.warning.strong.color,
            6.0,
            1.0,
        ),
        tooltip: styled_panel(
            base.tooltip,
            with_alpha(palette.danger.base.color, 0.18),
            palette.danger.strong.color,
            6.0,
            1.0,
        ),
        grid: grid_style(
            with_alpha(palette.primary.strong.color, 0.42),
            with_alpha(palette.warning.strong.color, 0.24),
            with_alpha(palette.danger.base.color, 0.20),
        ),
        tick_label_color: palette.warning.strong.color,
        axis_label_color: palette.danger.strong.color,
    }
}

fn styled_panel(
    style: iced::widget::container::Style,
    background: Color,
    border_color: Color,
    radius: f32,
    width: f32,
) -> iced::widget::container::Style {
    let style = style.background(background);
    restyled_border(style, border_color, radius, width)
}

fn restyled_border(
    style: iced::widget::container::Style,
    color: Color,
    radius: f32,
    width: f32,
) -> iced::widget::container::Style {
    style.border(Border {
        width,
        radius: radius.into(),
        color,
    })
}

fn grid_style(major: Color, minor: Color, sub_minor: Color) -> GridStyle {
    GridStyle {
        major,
        minor,
        sub_minor,
    }
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}
