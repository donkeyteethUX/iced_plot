//! Controls for user interaction with the plot.

use crate::message::PlotUiMessage;
use iced::{Element, keyboard, mouse, widget};
use std::collections::HashMap;

/// Configures user interaction behavior for [`crate::PlotWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct PlotControls {
    /// Controls how input triggers map to plot actions.
    pub interaction: InteractionControls,

    /// Enables point highlighting while hovering.
    pub highlight_on_hover: bool,

    /// Shows the in-canvas controls/help UI (`?` button).
    pub show_controls_help: bool,
}

/// Trigger-to-action configuration grouped by input family.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionControls {
    /// Mouse button drag bindings.
    pub drag: HashMap<mouse::Button, DragAction>,

    /// Scroll bindings keyed by active keyboard modifiers.
    pub scroll: HashMap<keyboard::Modifiers, ScrollAction>,

    /// Mouse click bindings.
    pub click: HashMap<mouse::Button, ClickAction>,

    /// Mouse double-click bindings.
    pub double_click: HashMap<mouse::Button, ClickAction>,

    /// Keyboard key bindings.
    pub key: HashMap<keyboard::Key, KeyAction>,

    /// Minimum drag distance, in screen pixels, before a drag gesture is treated
    /// as intentional instead of a click.
    pub drag_delta_threshold: f32,

    /// Fractional padding added around a completed box-zoom selection.
    ///
    /// For example, the default `0.02` expands the selected world-space bounds
    /// by 2% before applying the new camera bounds.
    pub selection_padding: f64,
}

/// Action that can be performed during a mouse drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragAction {
    /// Pan the camera while dragging.
    Pan,

    /// Draw a selection rectangle and zoom to it on release.
    BoxZoom,
}

/// Action that can be performed by scrolling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAction {
    /// Pan the camera by the scroll delta.
    Pan,

    /// Zoom at the cursor by the scroll delta.
    Zoom,
}

/// Action that can be performed by a mouse click or double-click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction {
    /// Pick the currently highlighted point.
    Pick,

    /// Reset/autoscale the plot.
    Autoscale,

    /// Clear picked points.
    ClearPick,
}

/// Action that can be performed by a key press.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyAction {
    /// Reset/autoscale the plot.
    Autoscale,

    /// Clear picked points.
    ClearPick,

    /// Pan by a fraction of the current visible camera span.
    PanBy {
        /// Direction to pan.
        direction: PanDirection,

        /// Fraction of the visible camera span to pan.
        fraction: f64,
    },
}

/// Direction for keyboard-style panning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanDirection {
    /// Pan toward smaller x values.
    Left,
    /// Pan toward larger x values.
    Right,
    /// Pan toward larger y values.
    Up,
    /// Pan toward smaller y values.
    Down,
}

// In keeping with our batteries-included philosophy, most everything is enabled by default.

impl Default for PlotControls {
    fn default() -> Self {
        Self {
            interaction: InteractionControls::default(),
            highlight_on_hover: true,
            show_controls_help: true,
        }
    }
}

impl Default for InteractionControls {
    fn default() -> Self {
        let mut controls = Self {
            drag: HashMap::new(),
            scroll: HashMap::new(),
            click: HashMap::new(),
            double_click: HashMap::new(),
            key: HashMap::new(),
            drag_delta_threshold: 4.0,
            selection_padding: 0.02,
        };

        controls
            .bind_drag(mouse::Button::Left, DragAction::Pan)
            .bind_drag(mouse::Button::Right, DragAction::BoxZoom)
            .bind_scroll(keyboard::Modifiers::NONE, ScrollAction::Pan)
            .bind_scroll(keyboard::Modifiers::CTRL, ScrollAction::Zoom)
            .bind_click(mouse::Button::Left, ClickAction::Pick)
            .bind_double_click(mouse::Button::Left, ClickAction::Autoscale)
            .bind_key(
                keyboard::Key::Named(keyboard::key::Named::Escape),
                KeyAction::ClearPick,
            );

        controls.bind_arrow_pan(0.1);
        controls
    }
}

impl InteractionControls {
    /// Bind a mouse drag trigger to an action.
    pub fn bind_drag(&mut self, button: mouse::Button, action: DragAction) -> &mut Self {
        self.drag.insert(button, action);
        self
    }

    /// Remove a mouse drag binding.
    pub fn unbind_drag(&mut self, button: mouse::Button) -> Option<DragAction> {
        self.drag.remove(&button)
    }

    /// Return the action bound to a mouse drag trigger.
    pub fn drag_action(&self, button: mouse::Button) -> Option<DragAction> {
        self.drag.get(&button).copied()
    }

    /// Return whether a mouse drag trigger is bound to an action.
    pub fn drag_is_bound(&self, button: mouse::Button, action: DragAction) -> bool {
        self.drag_action(button) == Some(action)
    }

    /// Bind a scroll trigger to an action.
    pub fn bind_scroll(
        &mut self,
        modifiers: keyboard::Modifiers,
        action: ScrollAction,
    ) -> &mut Self {
        self.scroll.insert(modifiers, action);
        self
    }

    /// Remove a scroll binding.
    pub fn unbind_scroll(&mut self, modifiers: keyboard::Modifiers) -> Option<ScrollAction> {
        self.scroll.remove(&modifiers)
    }

    /// Return the scroll action for active modifiers.
    ///
    /// Exact modifier bindings win. If no exact binding exists, a Ctrl binding
    /// still matches when additional modifiers are held.
    pub fn scroll_action(&self, modifiers: keyboard::Modifiers) -> Option<ScrollAction> {
        self.scroll.get(&modifiers).copied().or_else(|| {
            modifiers
                .contains(keyboard::Modifiers::CTRL)
                .then(|| self.scroll.get(&keyboard::Modifiers::CTRL).copied())
                .flatten()
        })
    }

    /// Return whether a scroll trigger is bound to an action.
    pub fn scroll_is_bound(&self, modifiers: keyboard::Modifiers, action: ScrollAction) -> bool {
        self.scroll.get(&modifiers).copied() == Some(action)
    }

    /// Bind a mouse click trigger to an action.
    pub fn bind_click(&mut self, button: mouse::Button, action: ClickAction) -> &mut Self {
        self.click.insert(button, action);
        self
    }

    /// Remove a mouse click binding.
    pub fn unbind_click(&mut self, button: mouse::Button) -> Option<ClickAction> {
        self.click.remove(&button)
    }

    /// Return the action bound to a mouse click trigger.
    pub fn click_action(&self, button: mouse::Button) -> Option<ClickAction> {
        self.click.get(&button).copied()
    }

    /// Return whether a mouse click trigger is bound to an action.
    pub fn click_is_bound(&self, button: mouse::Button, action: ClickAction) -> bool {
        self.click_action(button) == Some(action)
    }

    /// Bind a mouse double-click trigger to an action.
    pub fn bind_double_click(&mut self, button: mouse::Button, action: ClickAction) -> &mut Self {
        self.double_click.insert(button, action);
        self
    }

    /// Remove a mouse double-click binding.
    pub fn unbind_double_click(&mut self, button: mouse::Button) -> Option<ClickAction> {
        self.double_click.remove(&button)
    }

    /// Return the action bound to a mouse double-click trigger.
    pub fn double_click_action(&self, button: mouse::Button) -> Option<ClickAction> {
        self.double_click.get(&button).copied()
    }

    /// Return whether a mouse double-click trigger is bound to an action.
    pub fn double_click_is_bound(&self, button: mouse::Button, action: ClickAction) -> bool {
        self.double_click_action(button) == Some(action)
    }

    /// Bind a key trigger to an action.
    pub fn bind_key(&mut self, key: keyboard::Key, action: KeyAction) -> &mut Self {
        self.key.insert(key, action);
        self
    }

    /// Remove a key binding.
    pub fn unbind_key(&mut self, key: &keyboard::Key) -> Option<KeyAction> {
        self.key.remove(key)
    }

    /// Return the key-press action for a key.
    pub fn key_action(&self, key: &keyboard::Key) -> Option<KeyAction> {
        self.key.get(key).copied()
    }

    /// Return whether a key trigger is bound to an action.
    pub fn key_is_bound(&self, key: &keyboard::Key, action: KeyAction) -> bool {
        self.key_action(key) == Some(action)
    }

    /// Remove all drag bindings for a given action.
    pub fn remove_drag_action(&mut self, action: DragAction) -> &mut Self {
        self.drag.retain(|_, bound| *bound != action);
        self
    }

    /// Set the mouse button used for a drag action.
    ///
    /// Passing `None` disables the drag action.
    pub fn set_drag_action(
        &mut self,
        action: DragAction,
        button: Option<mouse::Button>,
    ) -> &mut Self {
        self.remove_drag_action(action);
        if let Some(button) = button {
            self.bind_drag(button, action);
        }
        self
    }

    /// Remove all key bindings for a given action.
    pub fn remove_key_action(&mut self, action: KeyAction) -> &mut Self {
        self.key.retain(|_, bound| *bound != action);
        self
    }

    /// Set the key used for a key action.
    ///
    /// Passing `None` disables the key action.
    pub fn set_key_action(&mut self, action: KeyAction, key: Option<keyboard::Key>) -> &mut Self {
        self.remove_key_action(action);
        if let Some(key) = key {
            self.bind_key(key, action);
        }
        self
    }

    /// Bind the four arrow keys to pan by the given fraction of the visible span.
    pub fn bind_arrow_pan(&mut self, fraction: f64) -> &mut Self {
        self.bind_key(
            keyboard::Key::Named(keyboard::key::Named::ArrowLeft),
            KeyAction::PanBy {
                direction: PanDirection::Left,
                fraction,
            },
        )
        .bind_key(
            keyboard::Key::Named(keyboard::key::Named::ArrowRight),
            KeyAction::PanBy {
                direction: PanDirection::Right,
                fraction,
            },
        )
        .bind_key(
            keyboard::Key::Named(keyboard::key::Named::ArrowUp),
            KeyAction::PanBy {
                direction: PanDirection::Up,
                fraction,
            },
        )
        .bind_key(
            keyboard::Key::Named(keyboard::key::Named::ArrowDown),
            KeyAction::PanBy {
                direction: PanDirection::Down,
                fraction,
            },
        )
    }

    /// Remove all arrow-key pan bindings.
    pub fn unbind_arrow_pan(&mut self) -> &mut Self {
        for key in [
            keyboard::key::Named::ArrowLeft,
            keyboard::key::Named::ArrowRight,
            keyboard::key::Named::ArrowUp,
            keyboard::key::Named::ArrowDown,
        ] {
            let key = keyboard::Key::Named(key);
            if matches!(self.key_action(&key), Some(KeyAction::PanBy { .. })) {
                self.unbind_key(&key);
            }
        }
        self
    }

    /// Return whether any trigger is bound to point picking.
    pub fn has_pick_action(&self) -> bool {
        self.click
            .values()
            .any(|action| *action == ClickAction::Pick)
            || self
                .double_click
                .values()
                .any(|action| *action == ClickAction::Pick)
    }

    /// Return whether all arrow keys are currently bound to pan actions.
    pub fn arrows_to_pan_enabled(&self) -> bool {
        [
            keyboard::key::Named::ArrowLeft,
            keyboard::key::Named::ArrowRight,
            keyboard::key::Named::ArrowUp,
            keyboard::key::Named::ArrowDown,
        ]
        .into_iter()
        .all(|key| {
            matches!(
                self.key_action(&keyboard::Key::Named(key)),
                Some(KeyAction::PanBy { .. })
            )
        })
    }
}

impl PlotControls {
    pub(crate) fn view_controls_overlay_panel(
        &self,
        has_legend: bool,
    ) -> Element<'_, PlotUiMessage> {
        let txt = |t| widget::text(t).size(12.0).style(widget::text::base);
        let mut content =
            widget::column![txt("Controls").style(widget::text::primary)].spacing(2.0);

        let mut bindings = self.interaction.binding_descriptions();
        bindings.sort();

        for binding in bindings {
            content = content.push(widget::text(binding).size(12.0).style(widget::text::base));
        }
        if has_legend {
            content = content.push(txt("Click icon in legend to toggle visibility."));
        }

        content.into()
    }
}

impl InteractionControls {
    fn binding_descriptions(&self) -> Vec<String> {
        let mut bindings = Vec::new();

        bindings.extend(self.drag.iter().map(|(button, action)| {
            format!(
                "{}-drag: {}",
                mouse_button_label(*button),
                drag_action_label(*action)
            )
        }));
        bindings.extend(self.scroll.iter().map(|(modifiers, action)| {
            let trigger = if modifiers.is_empty() {
                "Scroll".to_owned()
            } else {
                format!("{} + scroll", keyboard_modifiers_label(*modifiers))
            };
            format!("{trigger}: {}", scroll_action_label(*action))
        }));
        bindings.extend(self.click.iter().map(|(button, action)| {
            format!(
                "{}-click: {}",
                mouse_button_label(*button),
                click_action_label(*action)
            )
        }));
        bindings.extend(self.double_click.iter().map(|(button, action)| {
            format!(
                "{} double-click: {}",
                mouse_button_label(*button),
                click_action_label(*action)
            )
        }));
        bindings.extend(self.key.iter().map(|(key, action)| {
            format!("{}: {}", keyboard_key_label(key), key_action_label(*action))
        }));

        bindings
    }
}

fn mouse_button_label(button: mouse::Button) -> String {
    match button {
        mouse::Button::Left => "Left".to_owned(),
        mouse::Button::Right => "Right".to_owned(),
        mouse::Button::Middle => "Middle".to_owned(),
        mouse::Button::Back => "Back".to_owned(),
        mouse::Button::Forward => "Forward".to_owned(),
        mouse::Button::Other(number) => format!("Mouse {number}"),
    }
}

fn keyboard_key_label(key: &keyboard::Key) -> String {
    match key {
        keyboard::Key::Named(named) => format!("{named:?}"),
        keyboard::Key::Character(character) => character.to_string(),
        keyboard::Key::Unidentified => "Unidentified".to_owned(),
    }
}

fn drag_action_label(action: DragAction) -> &'static str {
    match action {
        DragAction::Pan => "pan",
        DragAction::BoxZoom => "box zoom",
    }
}

fn scroll_action_label(action: ScrollAction) -> &'static str {
    match action {
        ScrollAction::Pan => "pan",
        ScrollAction::Zoom => "zoom at cursor",
    }
}

fn click_action_label(action: ClickAction) -> &'static str {
    match action {
        ClickAction::Pick => "pick point",
        ClickAction::Autoscale => "reset / autoscale",
        ClickAction::ClearPick => "clear picked points",
    }
}

fn key_action_label(action: KeyAction) -> String {
    match action {
        KeyAction::Autoscale => "reset / autoscale".to_owned(),
        KeyAction::ClearPick => "clear picked points".to_owned(),
        KeyAction::PanBy {
            direction,
            fraction,
        } => format!("pan {direction:?} by {:.0}%", fraction * 100.0),
    }
}

fn keyboard_modifiers_label(modifiers: keyboard::Modifiers) -> String {
    let mut labels = Vec::new();
    if modifiers.contains(keyboard::Modifiers::CTRL) {
        labels.push("Ctrl");
    }
    if modifiers.contains(keyboard::Modifiers::SHIFT) {
        labels.push("Shift");
    }
    if modifiers.contains(keyboard::Modifiers::ALT) {
        labels.push("Alt");
    }
    if modifiers.contains(keyboard::Modifiers::LOGO) {
        labels.push("Logo");
    }
    if labels.is_empty() {
        "No modifier".to_owned()
    } else {
        labels.join(" + ")
    }
}
