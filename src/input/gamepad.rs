//! Gamepads: buttons and axes as games see them, fed by gamepad events
//! from the system (see [`GamepadBackend`](super::GamepadBackend)) or from
//! tests.

use std::collections::HashSet;

/// A gamepad button, named after its place on an Xbox-style pad: on a
/// PlayStation pad, `South` is cross, `East` circle, `West` square and
/// `North` triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    /// The left trigger, pressed past halfway (its analog value is the
    /// `LeftTrigger` axis).
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    /// A click of the left stick.
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// Every gamepad button, with the names scripts use for it (the first is
/// the main one).
pub const GAMEPAD_BUTTONS: [(GamepadButton, &[&str]); 16] = [
    (GamepadButton::South, &["a", "south", "cross"]),
    (GamepadButton::East, &["b", "east", "circle"]),
    (GamepadButton::West, &["x", "west", "square"]),
    (GamepadButton::North, &["y", "north", "triangle"]),
    (GamepadButton::LeftBumper, &["lb", "left_bumper", "l1"]),
    (GamepadButton::RightBumper, &["rb", "right_bumper", "r1"]),
    (GamepadButton::LeftTrigger, &["lt", "left_trigger", "l2"]),
    (GamepadButton::RightTrigger, &["rt", "right_trigger", "r2"]),
    (GamepadButton::Select, &["back", "select", "share"]),
    (GamepadButton::Start, &["start", "options", "menu"]),
    (GamepadButton::LeftStick, &["ls", "left_stick", "l3"]),
    (GamepadButton::RightStick, &["rs", "right_stick", "r3"]),
    (GamepadButton::DPadUp, &["up", "dpad_up"]),
    (GamepadButton::DPadDown, &["down", "dpad_down"]),
    (GamepadButton::DPadLeft, &["left", "dpad_left"]),
    (GamepadButton::DPadRight, &["right", "dpad_right"]),
];

/// A gamepad axis. Sticks go from -1 to 1, with +x right and +y **down**,
/// like world coordinates; triggers from 0 (released) to 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

/// Every gamepad axis, with the names scripts use for it.
pub const GAMEPAD_AXES: [(GamepadAxis, &[&str]); 6] = [
    (GamepadAxis::LeftStickX, &["left_x"]),
    (GamepadAxis::LeftStickY, &["left_y"]),
    (GamepadAxis::RightStickX, &["right_x"]),
    (GamepadAxis::RightStickY, &["right_y"]),
    (GamepadAxis::LeftTrigger, &["lt", "left_trigger", "l2"]),
    (GamepadAxis::RightTrigger, &["rt", "right_trigger", "r2"]),
];

/// The button named `name` (case-insensitive), as listed in
/// [`GAMEPAD_BUTTONS`].
pub fn parse_gamepad_button(name: &str) -> Option<GamepadButton> {
    let name = name.to_ascii_lowercase();
    GAMEPAD_BUTTONS
        .iter()
        .find(|(_, names)| names.contains(&name.as_str()))
        .map(|(button, _)| *button)
}

/// The axis named `name` (case-insensitive), as listed in [`GAMEPAD_AXES`].
pub fn parse_gamepad_axis(name: &str) -> Option<GamepadAxis> {
    let name = name.to_ascii_lowercase();
    GAMEPAD_AXES
        .iter()
        .find(|(_, names)| names.contains(&name.as_str()))
        .map(|(axis, _)| *axis)
}

/// Sticks closer to the middle than this read as centered: worn sticks
/// rarely rest exactly at 0.
pub const STICK_DEADZONE: f32 = 0.15;

/// Triggers pressed less than this read as released.
pub const TRIGGER_DEADZONE: f32 = 0.05;

/// Something that happened to a gamepad. `gamepad` identifies the pad for
/// as long as it stays connected.
#[derive(Clone, Debug, PartialEq)]
pub enum GamepadEvent {
    Connected {
        gamepad: usize,
    },
    Disconnected {
        gamepad: usize,
    },
    ButtonPressed {
        gamepad: usize,
        button: GamepadButton,
    },
    ButtonReleased {
        gamepad: usize,
        button: GamepadButton,
    },
    /// A raw axis value: sticks -1 to 1 with +y down, triggers 0 to 1.
    AxisMoved {
        gamepad: usize,
        axis: GamepadAxis,
        value: f32,
    },
}

/// One connected gamepad's state.
#[derive(Clone, Debug, Default)]
pub(super) struct Gamepad {
    pub(super) id: usize,
    pub(super) pressed: HashSet<GamepadButton>,
    pub(super) just_pressed: HashSet<GamepadButton>,
    pub(super) just_released: HashSet<GamepadButton>,
    /// Raw values, indexed like [`GAMEPAD_AXES`]
    raw_axes: [f32; 6],
}

impl Gamepad {
    pub(super) fn new(id: usize) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub(super) fn press(&mut self, button: GamepadButton) {
        if self.pressed.insert(button) {
            self.just_pressed.insert(button);
        }
    }

    pub(super) fn release(&mut self, button: GamepadButton) {
        if self.pressed.remove(&button) {
            self.just_released.insert(button);
        }
    }

    pub(super) fn set_axis(&mut self, axis: GamepadAxis, value: f32) {
        let value = if value.is_finite() { value } else { 0.0 };
        self.raw_axes[axis_index(axis)] = match axis {
            GamepadAxis::LeftTrigger | GamepadAxis::RightTrigger => value.clamp(0.0, 1.0),
            _ => value.clamp(-1.0, 1.0),
        };
    }

    /// An axis, with its dead zone: a stick's is round, so diagonals read
    /// like the other directions, and values beyond it start again from 0
    /// (no jump at its edge).
    pub(super) fn axis(&self, axis: GamepadAxis) -> f32 {
        let raw = |axis| self.raw_axes[axis_index(axis)];
        let (x, y, along_y) = match axis {
            GamepadAxis::LeftTrigger | GamepadAxis::RightTrigger => {
                let value = raw(axis);
                return if value < TRIGGER_DEADZONE {
                    0.0
                } else {
                    (value - TRIGGER_DEADZONE) / (1.0 - TRIGGER_DEADZONE)
                };
            }
            GamepadAxis::LeftStickX => (raw(axis), raw(GamepadAxis::LeftStickY), false),
            GamepadAxis::LeftStickY => (raw(GamepadAxis::LeftStickX), raw(axis), true),
            GamepadAxis::RightStickX => (raw(axis), raw(GamepadAxis::RightStickY), false),
            GamepadAxis::RightStickY => (raw(GamepadAxis::RightStickX), raw(axis), true),
        };
        let length = x.hypot(y);
        if length <= STICK_DEADZONE {
            return 0.0;
        }
        let scale = ((length - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)).min(1.0) / length;
        if along_y { y * scale } else { x * scale }
    }

    /// Start a new frame: nothing was just pressed or released yet.
    pub(super) fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

fn axis_index(axis: GamepadAxis) -> usize {
    GAMEPAD_AXES
        .iter()
        .position(|(candidate, _)| *candidate == axis)
        .unwrap_or(0)
}
