//! Demonstrates toggling plot interaction controls at runtime.
use iced::{
    Element, Length, keyboard, mouse,
    widget::{checkbox, column, container, row, text},
};
use iced_plot::{
    ClickAction, Color, DragAction, KeyAction, LineStyle, MarkerStyle, PlotUiMessage, PlotWidget,
    PlotWidgetBuilder, ScrollAction, Series,
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
    ToggleArrowsToPan(bool),
    ToggleBoxZoom(bool),
    ToggleCtrlScrollZoom(bool),
    ToggleDoubleClickAutoscale(bool),
    ToggleKeyAutoscale(bool),
    ToggleClickToPick(bool),
    ToggleClearPickOnEscape(bool),
    ToggleHighlightOnHover(bool),
    ToggleShowControlsHelp(bool),
}

struct App {
    widget: PlotWidget,
}

impl App {
    fn new() -> Self {
        let line = Series::line_only(
            (0..240)
                .map(|i| {
                    let x = i as f64 * 0.05;
                    [x, (x * 0.8).sin()]
                })
                .collect(),
            LineStyle::solid(),
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
        Self { widget }
    }

    fn set_drag(&mut self, button: mouse::Button, action: DragAction, enabled: bool) {
        if enabled {
            self.widget
                .get_controls_mut()
                .interaction
                .bind_drag(button, action);
        } else {
            self.widget
                .get_controls_mut()
                .interaction
                .unbind_drag(button);
        }
    }

    fn drag_enabled(&self, button: mouse::Button, action: DragAction) -> bool {
        self.widget
            .get_controls()
            .interaction
            .drag_is_bound(button, action)
    }

    fn set_scroll(&mut self, modifiers: keyboard::Modifiers, action: ScrollAction, enabled: bool) {
        if enabled {
            self.widget
                .get_controls_mut()
                .interaction
                .bind_scroll(modifiers, action);
        } else {
            self.widget
                .get_controls_mut()
                .interaction
                .unbind_scroll(modifiers);
        }
    }

    fn scroll_enabled(&self, modifiers: keyboard::Modifiers, action: ScrollAction) -> bool {
        self.widget
            .get_controls()
            .interaction
            .scroll_is_bound(modifiers, action)
    }

    fn set_click(&mut self, button: mouse::Button, action: ClickAction, enabled: bool) {
        if enabled {
            self.widget
                .get_controls_mut()
                .interaction
                .bind_click(button, action);
        } else {
            self.widget
                .get_controls_mut()
                .interaction
                .unbind_click(button);
        }
    }

    fn click_enabled(&self, button: mouse::Button, action: ClickAction) -> bool {
        self.widget
            .get_controls()
            .interaction
            .click_is_bound(button, action)
    }

    fn set_double_click(&mut self, button: mouse::Button, action: ClickAction, enabled: bool) {
        if enabled {
            self.widget
                .get_controls_mut()
                .interaction
                .bind_double_click(button, action);
        } else {
            self.widget
                .get_controls_mut()
                .interaction
                .unbind_double_click(button);
        }
    }

    fn double_click_enabled(&self, button: mouse::Button, action: ClickAction) -> bool {
        self.widget
            .get_controls()
            .interaction
            .double_click_is_bound(button, action)
    }

    fn set_key(&mut self, key: keyboard::Key, action: KeyAction, enabled: bool) {
        if enabled {
            self.widget
                .get_controls_mut()
                .interaction
                .bind_key(key, action);
        } else {
            self.widget.get_controls_mut().interaction.unbind_key(&key);
        }
    }

    fn key_enabled(&self, key: &keyboard::Key, action: KeyAction) -> bool {
        self.widget
            .get_controls()
            .interaction
            .key_is_bound(key, action)
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Plot(msg) => self.widget.update(msg),
            Message::ToggleScrollToPan(enabled) => {
                self.set_scroll(keyboard::Modifiers::NONE, ScrollAction::Pan, enabled);
            }
            Message::ToggleDragToPan(enabled) => {
                self.set_drag(mouse::Button::Right, DragAction::Pan, enabled);
            }
            Message::ToggleArrowsToPan(enabled) => {
                if enabled {
                    self.widget
                        .get_controls_mut()
                        .interaction
                        .bind_arrow_pan(0.1);
                } else {
                    self.widget
                        .get_controls_mut()
                        .interaction
                        .unbind_arrow_pan();
                }
            }
            Message::ToggleBoxZoom(enabled) => {
                self.set_drag(mouse::Button::Left, DragAction::BoxZoom, enabled);
            }
            Message::ToggleCtrlScrollZoom(enabled) => {
                self.set_scroll(keyboard::Modifiers::CTRL, ScrollAction::Zoom, enabled);
            }
            Message::ToggleDoubleClickAutoscale(enabled) => {
                self.set_double_click(mouse::Button::Left, ClickAction::Autoscale, enabled);
            }
            Message::ToggleKeyAutoscale(enabled) => {
                self.set_key(
                    keyboard::Key::Character("f".into()),
                    KeyAction::Autoscale,
                    enabled,
                );
            }
            Message::ToggleClickToPick(enabled) => {
                self.set_click(mouse::Button::Left, ClickAction::Pick, enabled);
            }
            Message::ToggleClearPickOnEscape(enabled) => {
                self.set_key(
                    keyboard::Key::Named(keyboard::key::Named::Escape),
                    KeyAction::ClearPick,
                    enabled,
                );
            }
            Message::ToggleHighlightOnHover(enabled) => {
                self.widget.get_controls_mut().highlight_on_hover = enabled;
            }
            Message::ToggleShowControlsHelp(enabled) => {
                self.widget.get_controls_mut().show_controls_help = enabled;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls_panel = container(
            column![
                text("Plot controls").size(18),
                row![
                    text("Pan:"),
                    checkbox(self.scroll_enabled(keyboard::Modifiers::NONE, ScrollAction::Pan))
                        .label("scroll")
                        .on_toggle(Message::ToggleScrollToPan),
                    checkbox(self.drag_enabled(mouse::Button::Left, DragAction::Pan))
                        .label("drag")
                        .on_toggle(Message::ToggleDragToPan),
                    checkbox(
                        self.widget
                            .get_controls()
                            .interaction
                            .arrows_to_pan_enabled()
                    )
                    .label("arrows")
                    .on_toggle(Message::ToggleArrowsToPan),
                ]
                .spacing(16),
                row![
                    text("Zoom:"),
                    checkbox(self.drag_enabled(mouse::Button::Right, DragAction::BoxZoom))
                        .label("box zoom with right-drag")
                        .on_toggle(Message::ToggleBoxZoom),
                    checkbox(self.scroll_enabled(keyboard::Modifiers::CTRL, ScrollAction::Zoom))
                        .label("Ctrl+scroll")
                        .on_toggle(Message::ToggleCtrlScrollZoom),
                    checkbox(
                        self.double_click_enabled(mouse::Button::Left, ClickAction::Autoscale)
                    )
                    .label("double-click autoscale")
                    .on_toggle(Message::ToggleDoubleClickAutoscale),
                    checkbox(
                        self.key_enabled(
                            &keyboard::Key::Character("f".into()),
                            KeyAction::Autoscale,
                        )
                    )
                    .label("key \"f\" to autoscale")
                    .on_toggle(Message::ToggleKeyAutoscale),
                ]
                .spacing(16),
                row![
                    text("Pick:"),
                    checkbox(self.click_enabled(mouse::Button::Left, ClickAction::Pick))
                        .label("click to pick")
                        .on_toggle(Message::ToggleClickToPick),
                    checkbox(self.key_enabled(
                        &keyboard::Key::Named(keyboard::key::Named::Escape),
                        KeyAction::ClearPick,
                    ))
                    .label("Esc clears")
                    .on_toggle(Message::ToggleClearPickOnEscape),
                ]
                .spacing(16),
                row![
                    checkbox(self.widget.get_controls().highlight_on_hover)
                        .label("Highlight on hover")
                        .on_toggle(Message::ToggleHighlightOnHover),
                    checkbox(self.widget.get_controls().show_controls_help)
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
