//! Keyboard, mouse and gamepad state, refreshed once per frame.

use egui::{Key, PointerButton};
use std::collections::{HashMap, HashSet};

mod actions;
mod gamepad;
mod gamepad_backend;

pub use actions::{
    ACTION_PRESS_THRESHOLD, ActionState, AxisDirection, InputActions, InputBinding,
    MAX_ACTION_NAME_CHARS,
};
use gamepad::Gamepad;
pub use gamepad::{
    GAMEPAD_AXES, GAMEPAD_BUTTONS, GamepadAxis, GamepadButton, GamepadEvent, STICK_DEADZONE,
    TRIGGER_DEADZONE, parse_gamepad_axis, parse_gamepad_button,
};
pub use gamepad_backend::GamepadBackend;

/// Which parts of a frame's input belong to the game (see
/// [`InputHandler::handle_input_with`]). An embedder such as the editor
/// withholds the keyboard while one of its own widgets has keyboard focus,
/// and the pointer while it is over its own widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputAccess {
    /// Keys and modifiers.
    pub keyboard: bool,
    /// Mouse buttons and the scroll wheel. The pointer's position is
    /// tracked either way.
    pub pointer: bool,
}

impl InputAccess {
    /// All input belongs to the game.
    pub const ALL: Self = Self {
        keyboard: true,
        pointer: true,
    };
}

const MOUSE_BUTTONS: [PointerButton; 5] = [
    PointerButton::Primary,
    PointerButton::Secondary,
    PointerButton::Middle,
    PointerButton::Extra1,
    PointerButton::Extra2,
];

/// A set of mouse buttons (`PointerButton` isn't hashable).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ButtonSet(u8);

impl ButtonSet {
    fn bit(button: PointerButton) -> u8 {
        match button {
            PointerButton::Primary => 1,
            PointerButton::Secondary => 2,
            PointerButton::Middle => 4,
            PointerButton::Extra1 => 8,
            PointerButton::Extra2 => 16,
        }
    }

    fn insert(&mut self, button: PointerButton) {
        self.0 |= Self::bit(button);
    }

    fn contains(self, button: PointerButton) -> bool {
        self.0 & Self::bit(button) != 0
    }

    /// The buttons in `self` that aren't in `other`.
    fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn iter(self) -> impl Iterator<Item = PointerButton> {
        MOUSE_BUTTONS
            .into_iter()
            .filter(move |button| self.contains(*button))
    }
}

/// Keyboard and mouse state, refreshed once per frame from egui's input,
/// and gamepad state, from gamepad events.
#[derive(Clone)]
pub struct InputHandler {
    keys_pressed: HashSet<Key>,
    keys_just_pressed: HashSet<Key>,
    keys_just_released: HashSet<Key>,
    mouse_buttons: ButtonSet,
    mouse_just_pressed: ButtonSet,
    mouse_just_released: ButtonSet,
    mouse_pos: egui::Pos2,
    prev_mouse_pos: egui::Pos2,
    // The pointer in the game, as the runtime maps it (see set_game_pointer)
    mouse_world_pos: (f32, f32),
    mouse_screen_pos: (f32, f32),
    // Where the left button last went down, in the camera frame's pixels
    press_screen_pos: Option<(f32, f32)>,
    scroll_delta: egui::Vec2,
    modifiers: egui::Modifiers,
    // Connected gamepads, in the order they connected
    gamepads: Vec<Gamepad>,
    // The game's actions (see set_actions), and their state this frame
    actions: InputActions,
    action_states: HashMap<String, ActionState>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_buttons: ButtonSet::default(),
            mouse_just_pressed: ButtonSet::default(),
            mouse_just_released: ButtonSet::default(),
            mouse_pos: egui::pos2(0.0, 0.0),
            prev_mouse_pos: egui::pos2(0.0, 0.0),
            mouse_world_pos: (0.0, 0.0),
            mouse_screen_pos: (0.0, 0.0),
            press_screen_pos: None,
            scroll_delta: egui::vec2(0.0, 0.0),
            modifiers: egui::Modifiers::default(),
            gamepads: Vec::new(),
            actions: InputActions::new(),
            action_states: HashMap::new(),
        }
    }

    /// Take in a frame's input, all of it.
    pub fn handle_input(&mut self, input: &egui::InputState) {
        self.handle_input_with(input, InputAccess::ALL);
    }

    /// Take in the parts of a frame's input that `access` allows; the rest
    /// counts as released.
    ///
    /// Presses and releases come from the frame's events as well as from
    /// comparing what is held with the previous frame, so a key or button
    /// pressed and released within one frame is still seen as just pressed
    /// (and just released).
    pub fn handle_input_with(&mut self, input: &egui::InputState, access: InputAccess) {
        self.modifiers = if access.keyboard {
            input.modifiers
        } else {
            egui::Modifiers::NONE
        };

        let old_keys = std::mem::take(&mut self.keys_pressed);
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        if access.keyboard {
            self.keys_pressed.extend(input.keys_down.iter().copied());
            for event in &input.events {
                if let egui::Event::Key {
                    key,
                    pressed,
                    repeat,
                    ..
                } = event
                {
                    if !pressed {
                        self.keys_just_released.insert(*key);
                    } else if !repeat {
                        self.keys_just_pressed.insert(*key);
                    }
                }
            }
        }
        // Changes without an event, e.g. keys withheld or the window losing
        // focus while they are held
        self.keys_just_pressed
            .extend(self.keys_pressed.difference(&old_keys));
        self.keys_just_released
            .extend(old_keys.difference(&self.keys_pressed));

        let old_buttons = std::mem::take(&mut self.mouse_buttons);
        self.mouse_just_pressed = ButtonSet::default();
        self.mouse_just_released = ButtonSet::default();
        if access.pointer {
            for button in MOUSE_BUTTONS {
                if input.pointer.button_down(button) {
                    self.mouse_buttons.insert(button);
                }
            }
            for event in &input.events {
                if let egui::Event::PointerButton {
                    button, pressed, ..
                } = event
                {
                    if *pressed {
                        self.mouse_just_pressed.insert(*button);
                    } else {
                        self.mouse_just_released.insert(*button);
                    }
                }
            }
        }
        self.mouse_just_pressed = self
            .mouse_just_pressed
            .union(self.mouse_buttons.difference(old_buttons));
        self.mouse_just_released = self
            .mouse_just_released
            .union(old_buttons.difference(self.mouse_buttons));

        // The position is tracked even when the pointer is withheld
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos = input.pointer.hover_pos().unwrap_or(self.mouse_pos);

        self.scroll_delta = if access.pointer {
            input.smooth_scroll_delta
        } else {
            egui::Vec2::ZERO
        };
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// True on the frame the key went down.
    pub fn is_key_just_pressed(&self, key: Key) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    /// True on the frame the key went up.
    pub fn is_key_just_released(&self, key: Key) -> bool {
        self.keys_just_released.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: PointerButton) -> bool {
        self.mouse_buttons.contains(button)
    }

    /// True on the frame the button went down.
    pub fn is_mouse_button_just_pressed(&self, button: PointerButton) -> bool {
        self.mouse_just_pressed.contains(button)
    }

    /// True on the frame the button went up.
    pub fn is_mouse_button_just_released(&self, button: PointerButton) -> bool {
        self.mouse_just_released.contains(button)
    }

    /// The modifier keys held (all released while the keyboard is
    /// withheld).
    pub fn modifiers(&self) -> egui::Modifiers {
        self.modifiers
    }

    /// The pointer's position in the window (the last known one when it
    /// isn't over the window).
    pub fn get_mouse_pos(&self) -> egui::Pos2 {
        self.mouse_pos
    }

    /// Record where the pointer is in the game: in world coordinates, and
    /// in screen coordinates (the camera frame's pixels, which text is laid
    /// out in). The runtime calls this every frame, after taking in the
    /// frame's input: when the left button just went down, this is also
    /// where it was pressed.
    pub fn set_game_pointer(&mut self, world: (f32, f32), screen: (f32, f32)) {
        self.mouse_world_pos = world;
        self.mouse_screen_pos = screen;
        if self.mouse_just_pressed.contains(PointerButton::Primary) {
            self.press_screen_pos = Some(screen);
        }
    }

    /// Where the left button last went down, in the camera frame's pixels:
    /// a button is clicked when the press and the release are both on it.
    pub fn get_mouse_press_screen_pos(&self) -> Option<(f32, f32)> {
        self.press_screen_pos
    }

    /// The pointer in world coordinates, as last recorded with
    /// [`set_game_pointer`](Self::set_game_pointer).
    pub fn get_mouse_world_pos(&self) -> (f32, f32) {
        self.mouse_world_pos
    }

    /// The pointer in the camera frame's pixels, as last recorded with
    /// [`set_game_pointer`](Self::set_game_pointer).
    pub fn get_mouse_screen_pos(&self) -> (f32, f32) {
        self.mouse_screen_pos
    }

    /// Take in a frame's gamepad events: call it once per frame, also
    /// without events, since it starts the frame (nothing was just pressed
    /// or released yet). A button pressed and released within the frame is
    /// just pressed and just released. A pad that sends events before its
    /// `Connected` counts as connected.
    pub fn handle_gamepad_events(&mut self, events: impl IntoIterator<Item = GamepadEvent>) {
        for gamepad in &mut self.gamepads {
            gamepad.begin_frame();
        }
        for event in events {
            match event {
                GamepadEvent::Connected { gamepad } => {
                    self.gamepad_mut(gamepad);
                }
                GamepadEvent::Disconnected { gamepad } => {
                    self.gamepads.retain(|pad| pad.id != gamepad);
                }
                GamepadEvent::ButtonPressed { gamepad, button } => {
                    self.gamepad_mut(gamepad).press(button);
                }
                GamepadEvent::ButtonReleased { gamepad, button } => {
                    self.gamepad_mut(gamepad).release(button);
                }
                GamepadEvent::AxisMoved {
                    gamepad,
                    axis,
                    value,
                } => self.gamepad_mut(gamepad).set_axis(axis, value),
            }
        }
    }

    fn gamepad_mut(&mut self, id: usize) -> &mut Gamepad {
        let index = match self.gamepads.iter().position(|pad| pad.id == id) {
            Some(index) => index,
            None => {
                self.gamepads.push(Gamepad::new(id));
                self.gamepads.len() - 1
            }
        };
        &mut self.gamepads[index]
    }

    /// The gamepads `pad` designates: the `pad`-th connected one (from 0),
    /// or all of them.
    fn pads(&self, pad: Option<usize>) -> impl Iterator<Item = &Gamepad> {
        let range = match pad {
            Some(index) => index..index.saturating_add(1),
            None => 0..self.gamepads.len(),
        };
        self.gamepads.get(range).into_iter().flatten()
    }

    /// How many gamepads are connected.
    pub fn gamepad_count(&self) -> usize {
        self.gamepads.len()
    }

    /// Whether a button is held, on the `pad`-th gamepad (from 0, in the
    /// order they connected) or, with `None`, on any.
    pub fn is_gamepad_pressed(&self, button: GamepadButton, pad: Option<usize>) -> bool {
        self.pads(pad)
            .any(|gamepad| gamepad.pressed.contains(&button))
    }

    /// Whether a button went down this frame (see
    /// [`is_gamepad_pressed`](Self::is_gamepad_pressed) for `pad`).
    pub fn is_gamepad_just_pressed(&self, button: GamepadButton, pad: Option<usize>) -> bool {
        self.pads(pad)
            .any(|gamepad| gamepad.just_pressed.contains(&button))
    }

    /// Whether a button went up this frame.
    pub fn is_gamepad_just_released(&self, button: GamepadButton, pad: Option<usize>) -> bool {
        self.pads(pad)
            .any(|gamepad| gamepad.just_released.contains(&button))
    }

    /// An axis of the `pad`-th gamepad, past its dead zone (see
    /// [`GamepadAxis`] for the ranges), or with `None` that of the first
    /// gamepad where it isn't at rest; 0 without such a gamepad.
    pub fn gamepad_axis(&self, axis: GamepadAxis, pad: Option<usize>) -> f32 {
        self.pads(pad)
            .map(|gamepad| gamepad.axis(axis))
            .find(|value| *value != 0.0)
            .unwrap_or(0.0)
    }

    pub fn get_mouse_delta(&self) -> Option<egui::Vec2> {
        Some(egui::vec2(
            self.mouse_pos.x - self.prev_mouse_pos.x,
            self.mouse_pos.y - self.prev_mouse_pos.y,
        ))
    }

    pub fn get_scroll_delta(&self) -> Option<egui::Vec2> {
        if self.scroll_delta.x != 0.0 || self.scroll_delta.y != 0.0 {
            Some(self.scroll_delta)
        } else {
            None
        }
    }

    /// Names of everything held down this frame, sorted:
    ///
    /// - modifiers: `"Ctrl"`, `"Shift"`, `"Alt"`, and `"Cmd"` (the macOS
    ///   Command key);
    /// - keys, by their egui names (`"Space"`, `"A"`, `"ArrowLeft"`,
    ///   `"ShiftLeft"`, ...), as accepted by [`parse_key`];
    /// - mouse buttons: `"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"`.
    pub fn get_all_active_inputs(&self) -> Vec<String> {
        let modifiers = [
            (self.modifiers.ctrl, "Ctrl"),
            (self.modifiers.shift, "Shift"),
            (self.modifiers.alt, "Alt"),
            (self.modifiers.mac_cmd, "Cmd"),
        ];
        let mut inputs: Vec<String> = modifiers
            .iter()
            .filter(|(held, _)| *held)
            .map(|(_, name)| name.to_string())
            .chain(self.keys_pressed.iter().map(|key| key.name().to_string()))
            .chain(
                self.mouse_buttons
                    .iter()
                    .map(|button| mouse_button_name(button).to_string()),
            )
            .collect();
        inputs.sort();
        inputs.dedup();
        inputs
    }
}

/// Look up a key by its egui name (`"Space"`, `"A"`, `"ArrowUp"`, ...),
/// ignoring case.
pub fn parse_key(name: &str) -> Option<Key> {
    Key::from_name(name).or_else(|| {
        Key::ALL
            .iter()
            .find(|key| key.name().eq_ignore_ascii_case(name))
            .copied()
    })
}

/// Look up a mouse button: `"left"`, `"right"` or `"middle"`, with or
/// without a `"Mouse"` prefix, ignoring case.
pub fn parse_mouse_button(name: &str) -> Option<PointerButton> {
    let name = name.to_ascii_lowercase();
    match name.strip_prefix("mouse").unwrap_or(&name) {
        "left" => Some(PointerButton::Primary),
        "right" => Some(PointerButton::Secondary),
        "middle" => Some(PointerButton::Middle),
        _ => None,
    }
}

fn mouse_button_name(button: PointerButton) -> &'static str {
    match button {
        PointerButton::Primary => "MouseLeft",
        PointerButton::Secondary => "MouseRight",
        PointerButton::Middle => "MouseMiddle",
        PointerButton::Extra1 => "MouseExtra1",
        PointerButton::Extra2 => "MouseExtra2",
    }
}
