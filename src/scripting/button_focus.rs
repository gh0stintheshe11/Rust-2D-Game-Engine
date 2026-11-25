//! Keyboard and gamepad focus for the buttons scripts draw with `button`.
//!
//! Buttons are immediate mode: a frame's input can't wait for the frame's
//! buttons to be drawn. So the focus moves among the buttons drawn in the
//! previous frame, before scripts run, and the buttons drawn this frame ask
//! whether they're focused, and whether Enter or gamepad A pressed them.

use crate::input::{GamepadAxis, GamepadButton, InputHandler};
use egui::Key;

/// The left stick moves the focus once it's pushed this far (and again
/// only after it came back).
const STICK_THRESHOLD: f32 = 0.5;

/// A button as drawn in a frame.
#[derive(Clone, Debug, PartialEq)]
struct Target {
    /// Its `id` option, or its label
    name: String,
    /// The name, numbered when several buttons of a frame share it
    key: String,
    position: (f32, f32),
    size: (f32, f32),
    focusable: bool,
    default: bool,
}

impl Target {
    fn contains(&self, point: (f32, f32)) -> bool {
        point.0 >= self.position.0
            && point.0 <= self.position.0 + self.size.0
            && point.1 >= self.position.1
            && point.1 <= self.position.1 + self.size.1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Previous,
    Next,
}

/// Which button has the focus, and the buttons it moves among.
#[derive(Debug, Default)]
pub(crate) struct ButtonFocus {
    // The buttons drawn so far this frame
    drawn: Vec<Target>,
    focused: Option<String>,
    // The button Enter or gamepad A pressed this frame
    pressed: Option<String>,
    // Where the left stick pointed last frame: -1 back, 1 forward, 0 neither
    stick: i8,
}

impl ButtonFocus {
    /// Start a frame, before scripts run: move the focus among the
    /// focusable buttons drawn last frame. It stays on its button while
    /// that one is drawn, or goes to the default button. The mouse moving
    /// over a button focuses it; the arrow keys, Tab and Shift+Tab, the
    /// d-pad and the left stick move it to the previous or next button,
    /// wrapping around; Enter and gamepad A press the focused button.
    pub(crate) fn begin_frame(&mut self, input: &InputHandler) {
        let previous = std::mem::take(&mut self.drawn);
        self.pressed = None;
        let stick = stick_direction(input);
        let stick_step = match stick {
            _ if stick == self.stick => None,
            -1 => Some(Step::Previous),
            1 => Some(Step::Next),
            _ => None,
        };
        self.stick = stick;

        let targets: Vec<&Target> = previous.iter().filter(|target| target.focusable).collect();
        if targets.is_empty() {
            self.focused = None;
            return;
        }
        let mut index = self
            .focused
            .as_ref()
            .and_then(|key| targets.iter().position(|target| &target.key == key))
            .or_else(|| targets.iter().position(|target| target.default));
        let moved = input
            .get_mouse_delta()
            .is_some_and(|delta| delta != egui::Vec2::ZERO);
        if moved {
            let pointer = input.get_mouse_screen_pos();
            if let Some(hovered) = targets.iter().position(|target| target.contains(pointer)) {
                index = Some(hovered);
            }
        }
        if let Some(step) = keyboard_step(input)
            .or_else(|| gamepad_step(input))
            .or(stick_step)
        {
            let count = targets.len();
            index = Some(match (index, step) {
                (None, Step::Next) => 0,
                (None, Step::Previous) => count - 1,
                (Some(index), Step::Next) => (index + 1) % count,
                (Some(index), Step::Previous) => (index + count - 1) % count,
            });
        }
        self.focused = index.map(|index| targets[index].key.clone());
        let press = input.is_key_just_pressed(Key::Enter)
            || input.is_gamepad_just_pressed(GamepadButton::South, None);
        if press {
            self.pressed = self.focused.clone();
        }
    }

    /// A button drawn this frame: whether it has the focus, and whether
    /// Enter or gamepad A pressed it. `name` is its `id` option or label.
    pub(crate) fn button(
        &mut self,
        name: &str,
        position: (f32, f32),
        size: (f32, f32),
        focusable: bool,
        default: bool,
    ) -> (bool, bool) {
        let same_name = self
            .drawn
            .iter()
            .filter(|target| target.name == name)
            .count();
        let key = match same_name {
            0 => name.to_string(),
            n => format!("{}\u{1f}{}", name, n),
        };
        let focused = focusable && self.focused.as_ref() == Some(&key);
        let pressed = focused && self.pressed.as_ref() == Some(&key);
        self.drawn.push(Target {
            name: name.to_string(),
            key,
            position,
            size,
            focusable,
            default,
        });
        (focused, pressed)
    }
}

fn keyboard_step(input: &InputHandler) -> Option<Step> {
    let pressed = |key| input.is_key_just_pressed(key);
    let tab = pressed(Key::Tab);
    let shift = input.modifiers().shift;
    if pressed(Key::ArrowUp) || pressed(Key::ArrowLeft) || (tab && shift) {
        Some(Step::Previous)
    } else if pressed(Key::ArrowDown) || pressed(Key::ArrowRight) || tab {
        Some(Step::Next)
    } else {
        None
    }
}

fn gamepad_step(input: &InputHandler) -> Option<Step> {
    let pressed = |button| input.is_gamepad_just_pressed(button, None);
    if pressed(GamepadButton::DPadUp) || pressed(GamepadButton::DPadLeft) {
        Some(Step::Previous)
    } else if pressed(GamepadButton::DPadDown) || pressed(GamepadButton::DPadRight) {
        Some(Step::Next)
    } else {
        None
    }
}

/// Where the left stick points: up or left is back (-1), down or right
/// forward (1), along whichever axis it's pushed further.
fn stick_direction(input: &InputHandler) -> i8 {
    let x = input.gamepad_axis(GamepadAxis::LeftStickX, None);
    let y = input.gamepad_axis(GamepadAxis::LeftStickY, None);
    let value = if y.abs() >= x.abs() { y } else { x };
    if value > STICK_THRESHOLD {
        1
    } else if value < -STICK_THRESHOLD {
        -1
    } else {
        0
    }
}
