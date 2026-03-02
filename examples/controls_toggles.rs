//! Demonstrates toggling plot interaction controls at runtime.
use iced::{
    Element, Length,
    widget::{checkbox, column, container, row, text},
};
use iced_plot::{
    Color, LineStyle, MarkerStyle, PlotControls, PlotUiMessage, PlotWidget, PlotWidgetBuilder,
    Series,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .font(include_bytes!("fonts/FiraCodeNerdFont-Regular.ttf"))
        .default_font(iced::Font::with_name("FiraCode Nerd Font"))
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Plot(PlotUiMessage),
    ToggleScrollToPan(bool),
    ToggleDragToPan(bool),
    ToggleBoxZoom(bool),
    ToggleCtrlScrollZoom(bool),
    ToggleDoubleClickAutoscale(bool),
    ToggleClickToPick(bool),
    ToggleClearPickOnEscape(bool),
    ToggleHighlightOnHover(bool),
    ToggleShowControlsHelp(bool),
}

struct App {
    widget: PlotWidget,
    controls: PlotControls,
}

impl App {
    fn new() -> Self {
        let controls = PlotControls::default();

        let line = Series::line_only(
            (0..240)
                .map(|i| {
                    let x = i as f64 * 0.05;
                    [x, (x * 0.8).sin()]
                })
                .collect(),
            LineStyle::Solid,
        )
        .with_label("sin(x)")
        .with_color(Color::from_rgb(0.3, 0.7, 1.0));

        let markers = Series::markers_only(
            (0..80)
                .map(|i| {
                    let x = i as f64 * 0.15;
                    [x, (x * 0.6).cos() * 0.8]
                })
                .collect(),
            MarkerStyle::circle(5.0),
        )
        .with_label("0.8 * cos(x)")
        .with_color(Color::from_rgb(1.0, 0.5, 0.35));

        let widget = PlotWidgetBuilder::new()
            .with_controls(controls)
            .with_x_label("x")
            .with_y_label("y")
            .with_crosshairs(true)
            .with_cursor_overlay(true)
            .with_pick_highlight_provider(|ctx, point| {
                point.resize_marker(1.8);
                point.color = Color::from_rgb(1.0, 0.2, 0.2);
                Some(format!(
                    "{}\nindex: {}\nx: {:.2}\ny: {:.2}\n(selected)",
                    ctx.series_label, ctx.point_index, point.x, point.y
                ))
            })
            .add_series(line)
            .add_series(markers)
            .build()
            .unwrap();

        Self { widget, controls }
    }

    fn apply_controls(&mut self) {
        self.widget.set_controls(self.controls);
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Plot(msg) => self.widget.update(msg),
            Message::ToggleScrollToPan(enabled) => {
                self.controls.pan.scroll_to_pan = enabled;
                self.apply_controls();
            }
            Message::ToggleDragToPan(enabled) => {
                self.controls.pan.drag_to_pan = enabled;
                self.apply_controls();
            }
            Message::ToggleBoxZoom(enabled) => {
                self.controls.zoom.box_zoom = enabled;
                self.apply_controls();
            }
            Message::ToggleCtrlScrollZoom(enabled) => {
                self.controls.zoom.scroll_with_ctrl = enabled;
                self.apply_controls();
            }
            Message::ToggleDoubleClickAutoscale(enabled) => {
                self.controls.zoom.double_click_autoscale = enabled;
                self.apply_controls();
            }
            Message::ToggleClickToPick(enabled) => {
                self.controls.pick.click_to_pick = enabled;
                self.apply_controls();
            }
            Message::ToggleClearPickOnEscape(enabled) => {
                self.controls.pick.clear_on_escape = enabled;
                self.apply_controls();
            }
            Message::ToggleHighlightOnHover(enabled) => {
                self.controls.highlight_on_hover = enabled;
                self.apply_controls();
            }
            Message::ToggleShowControlsHelp(enabled) => {
                self.controls.show_controls_help = enabled;
                self.apply_controls();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls_panel = container(
            column![
                text("Plot controls").size(18),
                row![
                    checkbox(self.controls.pan.scroll_to_pan)
                        .label("Pan: scroll")
                        .on_toggle(Message::ToggleScrollToPan),
                    checkbox(self.controls.pan.drag_to_pan)
                        .label("Pan: drag")
                        .on_toggle(Message::ToggleDragToPan),
                ]
                .spacing(16),
                row![
                    checkbox(self.controls.zoom.box_zoom)
                        .label("Zoom: box zoom with right-drag")
                        .on_toggle(Message::ToggleBoxZoom),
                    checkbox(self.controls.zoom.scroll_with_ctrl)
                        .label("Zoom: Ctrl+scroll")
                        .on_toggle(Message::ToggleCtrlScrollZoom),
                    checkbox(self.controls.zoom.double_click_autoscale)
                        .label("Zoom: double-click autoscale")
                        .on_toggle(Message::ToggleDoubleClickAutoscale),
                ]
                .spacing(16),
                row![
                    checkbox(self.controls.pick.click_to_pick)
                        .label("Pick: click to pick")
                        .on_toggle(Message::ToggleClickToPick),
                    checkbox(self.controls.pick.clear_on_escape)
                        .label("Pick: Esc clears")
                        .on_toggle(Message::ToggleClearPickOnEscape),
                ]
                .spacing(16),
                row![
                    checkbox(self.controls.highlight_on_hover)
                        .label("Highlight on hover")
                        .on_toggle(Message::ToggleHighlightOnHover),
                    checkbox(self.controls.show_controls_help)
                        .label("Show controls help")
                        .on_toggle(Message::ToggleShowControlsHelp),
                ]
                .spacing(16),
            ]
            .spacing(10),
        )
        .padding(12)
        .width(Length::Fill);

        column![controls_panel, self.widget.view().map(Message::Plot)]
            .spacing(8)
            .into()
    }
}
