//! Input actions: named controls such as `"jump"` or `"left"`, each bound to
//! keys, mouse buttons, and gamepad buttons, sticks and triggers. Games ask
//! whether an action is pressed instead of checking every key and button
//! that stands for it, and players can bind actions to other ones.

use super::{
    GAMEPAD_AXES, GAMEPAD_BUTTONS, GamepadAxis, GamepadButton, InputHandler, parse_gamepad_axis,
    parse_gamepad_button, parse_key,
};
use egui::{Key, PointerButton};
use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// How far (from 0 to 1, past its dead zone) a stick or trigger is pushed
/// to hold its action down, unless the action says otherwise (see
/// [`InputActions::set_threshold`]). Keys and buttons hold it at 1.
pub const ACTION_PRESS_THRESHOLD: f32 = 0.5;

/// Longest action name, in characters.
pub const MAX_ACTION_NAME_CHARS: usize = 64;

/// Which way an axis binding reads its stick or trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisDirection {
    /// Left or up, on a stick
    Negative,
    /// Right or down on a stick (+y is down), pressed on a trigger
    Positive,
}

/// What an action can be bound to. Bindings are written as text in project
/// files and scripts:
///
/// - `key:Space`, `key:A`, `key:ArrowLeft`: a key, by its egui name
///   (ignoring case);
/// - `mouse:left`, `mouse:right`, `mouse:middle`, `mouse:extra1`,
///   `mouse:extra2`;
/// - `pad:a`, `pad:start`, `pad:up`: a gamepad button on any gamepad, by the
///   names scripts use (see [`GAMEPAD_BUTTONS`]); `pad1:a` only on the
///   first gamepad, `pad2:a` on the second, and so on;
/// - `pad:left_x-`, `pad:left_x+`, `pad:left_y-`, `pad:left_y+` (and
///   `right_…`): a stick pushed left, right, up or down (+y is down), by
///   how far; `pad:lt+`, `pad:rt+`: a trigger, by how far it's pressed
///   (`pad:lt` without a sign is the trigger as a button).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputBinding {
    Key(Key),
    Mouse(PointerButton),
    /// A gamepad button, on the `pad`-th gamepad (from 0, in the order they
    /// connected) or with `None` on any
    GamepadButton {
        button: GamepadButton,
        pad: Option<usize>,
    },
    /// A stick or trigger pushed one way: an analog binding, as strong as
    /// it's pushed
    GamepadAxis {
        axis: GamepadAxis,
        direction: AxisDirection,
        pad: Option<usize>,
    },
}

impl InputBinding {
    /// Read a binding written as text (see [`InputBinding`]).
    pub fn parse(text: &str) -> Result<Self, String> {
        let invalid = |why: String| format!("invalid binding \"{}\": {}", text, why);
        let devices = || "expected key:…, mouse:… or pad:…".to_string();
        let (device, name) = text
            .trim()
            .split_once(':')
            .ok_or_else(|| invalid(devices()))?;
        let (device, name) = (device.trim().to_ascii_lowercase(), name.trim());
        match device.as_str() {
            "key" => parse_key(name)
                .map(Self::Key)
                .ok_or_else(|| invalid(format!("no key is named \"{}\"", name))),
            "mouse" => mouse_button(name).map(Self::Mouse).ok_or_else(|| {
                invalid(
                    "expected mouse:left, mouse:right, mouse:middle, mouse:extra1 or mouse:extra2"
                        .to_string(),
                )
            }),
            _ => {
                let number = device
                    .strip_prefix("pad")
                    .ok_or_else(|| invalid(devices()))?;
                let pad = match number {
                    "" => None,
                    number => match number.parse::<usize>() {
                        Ok(number) if number >= 1 => Some(number - 1),
                        _ => return Err(invalid("gamepads are numbered from 1".to_string())),
                    },
                };
                let signed = match (name.strip_suffix('+'), name.strip_suffix('-')) {
                    (Some(axis), _) => Some((axis, AxisDirection::Positive)),
                    (_, Some(axis)) => Some((axis, AxisDirection::Negative)),
                    _ => None,
                };
                match signed {
                    Some((axis, direction)) => {
                        let axis = parse_gamepad_axis(axis).ok_or_else(|| {
                            let names: Vec<&str> =
                                GAMEPAD_AXES.iter().map(|(_, names)| names[0]).collect();
                            invalid(format!("expected a stick or trigger: {}", names.join(", ")))
                        })?;
                        if is_trigger(axis) && direction == AxisDirection::Negative {
                            return Err(invalid(
                                "triggers only go one way: lt+ or rt+".to_string(),
                            ));
                        }
                        Ok(Self::GamepadAxis {
                            axis,
                            direction,
                            pad,
                        })
                    }
                    None => parse_gamepad_button(name)
                        .map(|button| Self::GamepadButton { button, pad })
                        .ok_or_else(|| {
                            let names: Vec<&str> =
                                GAMEPAD_BUTTONS.iter().map(|(_, names)| names[0]).collect();
                            invalid(format!("expected a gamepad button: {}", names.join(", ")))
                        }),
                }
            }
        }
    }

    /// What the binding is called in menus: `"Space"`, `"Left mouse
    /// button"`, `"Gamepad A"`, `"Gamepad 2 left stick up"`…
    pub fn label(&self) -> String {
        let pad = |pad: Option<usize>| match pad {
            None => "Gamepad".to_string(),
            Some(index) => format!("Gamepad {}", index + 1),
        };
        match *self {
            Self::Key(key) => match key {
                Key::ArrowLeft => "Left arrow".to_string(),
                Key::ArrowRight => "Right arrow".to_string(),
                Key::ArrowUp => "Up arrow".to_string(),
                Key::ArrowDown => "Down arrow".to_string(),
                key => key.name().to_string(),
            },
            Self::Mouse(button) => match button {
                PointerButton::Primary => "Left mouse button".to_string(),
                PointerButton::Secondary => "Right mouse button".to_string(),
                PointerButton::Middle => "Middle mouse button".to_string(),
                PointerButton::Extra1 => "Mouse button 4".to_string(),
                PointerButton::Extra2 => "Mouse button 5".to_string(),
            },
            Self::GamepadButton {
                button,
                pad: number,
            } => {
                format!("{} {}", pad(number), button_label(button))
            }
            Self::GamepadAxis {
                axis,
                direction,
                pad: number,
            } => format!("{} {}", pad(number), axis_label(axis, direction)),
        }
    }

    /// How far the binding is held this frame: 0 or 1 for keys and buttons,
    /// 0 to 1 for sticks and triggers.
    fn strength(&self, input: &InputHandler) -> f32 {
        let held = |held: bool| if held { 1.0 } else { 0.0 };
        match *self {
            Self::Key(key) => held(input.is_key_pressed(key)),
            Self::Mouse(button) => held(input.is_mouse_button_pressed(button)),
            Self::GamepadButton { button, pad } => held(input.is_gamepad_pressed(button, pad)),
            Self::GamepadAxis {
                axis,
                direction,
                pad,
            } => input
                .pads(pad)
                .map(|gamepad| directed(gamepad.axis(axis), direction))
                .fold(0.0, f32::max),
        }
    }

    /// Whether a key or button of the binding went down this frame, also
    /// when it was released within the frame.
    fn went_down(&self, input: &InputHandler) -> bool {
        match *self {
            Self::Key(key) => input.is_key_just_pressed(key),
            Self::Mouse(button) => input.is_mouse_button_just_pressed(button),
            Self::GamepadButton { button, pad } => input.is_gamepad_just_pressed(button, pad),
            Self::GamepadAxis { .. } => false,
        }
    }
}

impl fmt::Display for InputBinding {
    /// The binding as text, the way [`parse`](Self::parse) reads it back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pad = |pad: Option<usize>| match pad {
            None => "pad".to_string(),
            Some(index) => format!("pad{}", index + 1),
        };
        match *self {
            Self::Key(key) => write!(f, "key:{}", key_name(key)),
            Self::Mouse(button) => write!(f, "mouse:{}", mouse_button_name(button)),
            Self::GamepadButton {
                button,
                pad: number,
            } => {
                write!(f, "{}:{}", pad(number), button_name(button))
            }
            Self::GamepadAxis {
                axis,
                direction,
                pad: number,
            } => {
                let sign = match direction {
                    AxisDirection::Negative => '-',
                    AxisDirection::Positive => '+',
                };
                write!(f, "{}:{}{}", pad(number), axis_name(axis), sign)
            }
        }
    }
}

/// An action's state in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionState {
    /// Held down: a key or button bound to it is, or a stick or trigger
    /// bound to it is pushed at least as far as the action's threshold
    pub pressed: bool,
    /// It went down this frame (also when it was pressed and released
    /// within the frame)
    pub just_pressed: bool,
    /// It went up this frame
    pub just_released: bool,
    /// How far it's held, from 0 to 1: as far as its strongest binding
    pub strength: f32,
}

impl ActionState {
    /// The state following `previous`, with the action's bindings now
    /// `strength` strong (pressed from `threshold` on), and whether a key or
    /// button of them went down within the frame.
    fn next(previous: ActionState, strength: f32, threshold: f32, went_down: bool) -> Self {
        let pressed = strength >= threshold;
        let was = previous.pressed;
        Self {
            pressed,
            just_pressed: !was && (pressed || went_down),
            // Released, or tapped: pressed and released within the frame
            just_released: (was && !pressed) || (!was && went_down && !pressed),
            strength,
        }
    }
}

/// An action's bindings, and how far a stick or trigger bound to it is
/// pushed to press it.
#[derive(Clone, Debug, PartialEq)]
struct Action {
    bindings: Vec<InputBinding>,
    threshold: f32,
}

impl Action {
    fn new(bindings: Vec<InputBinding>) -> Self {
        Self {
            bindings,
            threshold: ACTION_PRESS_THRESHOLD,
        }
    }
}

/// A game's actions and their bindings, in the order they were defined.
/// Project files keep them as `"input_actions": {"jump": ["key:Space",
/// "pad:a"], …}`, or for an action with a threshold of its own
/// `"down": {"bindings": [...], "threshold": 0.7}`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InputActions {
    actions: IndexMap<String, Action>,
}

impl InputActions {
    pub fn new() -> Self {
        Self::default()
    }

    /// The actions new projects start with: `left`, `right`, `up` and
    /// `down` on WASD, the arrow keys, the d-pad and the left stick; `jump`
    /// on Space and gamepad A; `pause` on Escape and Start.
    pub fn defaults() -> Self {
        use AxisDirection::{Negative, Positive};
        let pad = |button| InputBinding::GamepadButton { button, pad: None };
        let stick = |axis, direction| InputBinding::GamepadAxis {
            axis,
            direction,
            pad: None,
        };
        let directions = [
            (
                "left",
                Key::A,
                Key::ArrowLeft,
                GamepadButton::DPadLeft,
                GamepadAxis::LeftStickX,
                Negative,
            ),
            (
                "right",
                Key::D,
                Key::ArrowRight,
                GamepadButton::DPadRight,
                GamepadAxis::LeftStickX,
                Positive,
            ),
            (
                "up",
                Key::W,
                Key::ArrowUp,
                GamepadButton::DPadUp,
                GamepadAxis::LeftStickY,
                Negative,
            ),
            (
                "down",
                Key::S,
                Key::ArrowDown,
                GamepadButton::DPadDown,
                GamepadAxis::LeftStickY,
                Positive,
            ),
        ];
        let mut actions = IndexMap::new();
        for (name, letter, arrow, dpad, axis, direction) in directions {
            let bindings = vec![
                InputBinding::Key(letter),
                InputBinding::Key(arrow),
                pad(dpad),
                stick(axis, direction),
            ];
            actions.insert(name.to_string(), Action::new(bindings));
        }
        let jump = vec![InputBinding::Key(Key::Space), pad(GamepadButton::South)];
        actions.insert("jump".to_string(), Action::new(jump));
        let pause = vec![InputBinding::Key(Key::Escape), pad(GamepadButton::Start)];
        actions.insert("pause".to_string(), Action::new(pause));
        Self { actions }
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// The actions and their bindings, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[InputBinding])> {
        self.actions
            .iter()
            .map(|(name, action)| (name.as_str(), action.bindings.as_slice()))
    }

    /// An action's bindings; `None` when there's no such action.
    pub fn bindings(&self, name: &str) -> Option<&[InputBinding]> {
        self.actions
            .get(name)
            .map(|action| action.bindings.as_slice())
    }

    /// How far a stick or trigger bound to an action is pushed to press it,
    /// from 0 to 1; `None` when there's no such action.
    pub fn threshold(&self, name: &str) -> Option<f32> {
        self.actions.get(name).map(|action| action.threshold)
    }

    /// Bind an action, adding it at the end if it's new. Names are 1 to
    /// [`MAX_ACTION_NAME_CHARS`] characters, without spaces around them.
    pub fn set(&mut self, name: &str, bindings: Vec<InputBinding>) -> Result<(), String> {
        validate_name(name)?;
        match self.actions.get_mut(name) {
            Some(existing) => existing.bindings = bindings,
            None => {
                self.actions.insert(name.to_string(), Action::new(bindings));
            }
        }
        Ok(())
    }

    /// Set how far a stick or trigger bound to the action is pushed to
    /// press it: above 0, up to 1 (the default is
    /// [`ACTION_PRESS_THRESHOLD`]). A higher one keeps a stick pushed
    /// diagonally from pressing it, say.
    pub fn set_threshold(&mut self, name: &str, threshold: f32) -> Result<(), String> {
        if !(threshold > 0.0 && threshold <= 1.0) {
            return Err(format!(
                "an action's threshold is above 0 and at most 1, not {}",
                threshold
            ));
        }
        let action = self
            .actions
            .get_mut(name)
            .ok_or_else(|| format!("there's no action named \"{}\"", name))?;
        action.threshold = threshold;
        Ok(())
    }

    /// Rename an action, keeping its place and bindings.
    pub fn rename(&mut self, name: &str, new_name: &str) -> Result<(), String> {
        validate_name(new_name)?;
        if name == new_name {
            return Ok(());
        }
        if self.actions.contains_key(new_name) {
            return Err(format!("there's already an action named \"{}\"", new_name));
        }
        let index = self
            .actions
            .get_index_of(name)
            .ok_or_else(|| format!("there's no action named \"{}\"", name))?;
        if let Some(action) = self.actions.shift_remove(name) {
            self.actions
                .shift_insert(index, new_name.to_string(), action);
        }
        Ok(())
    }

    /// Remove an action; returns whether there was one.
    pub fn remove(&mut self, name: &str) -> bool {
        self.actions.shift_remove(name).is_some()
    }
}

/// An action as project files keep it: its bindings, with its threshold
/// when it isn't the default.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum StoredAction {
    Bindings(Vec<String>),
    WithThreshold {
        bindings: Vec<String>,
        threshold: f32,
    },
}

impl Serialize for InputActions {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let stored: IndexMap<&str, StoredAction> = self
            .actions
            .iter()
            .map(|(name, action)| {
                let bindings = action.bindings.iter().map(ToString::to_string).collect();
                let stored = if action.threshold == ACTION_PRESS_THRESHOLD {
                    StoredAction::Bindings(bindings)
                } else {
                    StoredAction::WithThreshold {
                        bindings,
                        threshold: action.threshold,
                    }
                };
                (name.as_str(), stored)
            })
            .collect();
        stored.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InputActions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let stored = IndexMap::<String, StoredAction>::deserialize(deserializer)?;
        let mut actions = Self::new();
        for (name, action) in stored {
            let in_action =
                |e: String| D::Error::custom(format!("input action \"{}\": {}", name, e));
            let (texts, threshold) = match action {
                StoredAction::Bindings(texts) => (texts, None),
                StoredAction::WithThreshold {
                    bindings,
                    threshold,
                } => (bindings, Some(threshold)),
            };
            let bindings = texts
                .iter()
                .map(|text| InputBinding::parse(text))
                .collect::<Result<Vec<_>, _>>()
                .map_err(in_action)?;
            actions.set(&name, bindings).map_err(D::Error::custom)?;
            if let Some(threshold) = threshold {
                actions.set_threshold(&name, threshold).map_err(in_action)?;
            }
        }
        Ok(actions)
    }
}

impl InputHandler {
    /// The game's actions: what [`action`](Self::action) and friends
    /// read. Actions that stay keep their state.
    pub fn set_actions(&mut self, actions: InputActions) {
        self.action_states
            .retain(|name, _| actions.bindings(name).is_some());
        self.actions = actions;
    }

    pub fn actions(&self) -> &InputActions {
        &self.actions
    }

    /// Bind an action, adding it if it's new (a game's controls menu, say).
    pub fn set_action_bindings(
        &mut self,
        name: &str,
        bindings: Vec<InputBinding>,
    ) -> Result<(), String> {
        self.actions.set(name, bindings)
    }

    /// Work out every action's state from this frame's input. Call it once
    /// per frame, after taking in the keyboard, the mouse and the gamepads.
    pub fn update_actions(&mut self) {
        let mut states = std::mem::take(&mut self.action_states);
        for (name, action) in &self.actions.actions {
            let strength = action
                .bindings
                .iter()
                .map(|binding| binding.strength(self))
                .fold(0.0, f32::max);
            let went_down = action
                .bindings
                .iter()
                .any(|binding| binding.went_down(self));
            let threshold = action.threshold;
            match states.get_mut(name) {
                Some(state) => *state = ActionState::next(*state, strength, threshold, went_down),
                None => {
                    let state =
                        ActionState::next(ActionState::default(), strength, threshold, went_down);
                    states.insert(name.to_string(), state);
                }
            }
        }
        self.action_states = states;
    }

    /// An action's state this frame; `None` when there's no such action.
    pub fn action(&self, name: &str) -> Option<ActionState> {
        self.actions.bindings(name)?;
        Some(self.action_states.get(name).copied().unwrap_or_default())
    }

    /// How far `positive` is held minus how far `negative` is, from -1 to
    /// 1: e.g. `("left", "right")` to run. `None` when either action
    /// doesn't exist.
    pub fn action_axis(&self, negative: &str, positive: &str) -> Option<f32> {
        let (negative, positive) = (self.action(negative)?, self.action(positive)?);
        Some(positive.strength - negative.strength)
    }

    /// Something that went down this frame, as a binding: a key, a mouse
    /// button, a gamepad button, or a stick or trigger pushed past
    /// [`ACTION_PRESS_THRESHOLD`] (for any gamepad). For a game's "press a
    /// key" menus.
    pub fn just_pressed_binding(&self) -> Option<InputBinding> {
        if let Some(key) = self.keys_just_pressed.iter().min() {
            return Some(InputBinding::Key(*key));
        }
        if let Some(button) = self.mouse_just_pressed.iter().next() {
            return Some(InputBinding::Mouse(button));
        }
        for gamepad in &self.gamepads {
            for (button, _) in GAMEPAD_BUTTONS {
                if gamepad.just_pressed.contains(&button) {
                    return Some(InputBinding::GamepadButton { button, pad: None });
                }
            }
            for (axis, _) in GAMEPAD_AXES {
                for direction in [AxisDirection::Negative, AxisDirection::Positive] {
                    let was = directed(gamepad.previous_axis(axis), direction);
                    let now = directed(gamepad.axis(axis), direction);
                    if was < ACTION_PRESS_THRESHOLD && now >= ACTION_PRESS_THRESHOLD {
                        return Some(InputBinding::GamepadAxis {
                            axis,
                            direction,
                            pad: None,
                        });
                    }
                }
            }
        }
        None
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        Err("an action needs a name".to_string())
    } else if name.trim() != name {
        Err(format!(
            "action names can't start or end with spaces: \"{}\"",
            name
        ))
    } else if name.chars().count() > MAX_ACTION_NAME_CHARS {
        Err(format!(
            "action names are at most {} characters long",
            MAX_ACTION_NAME_CHARS
        ))
    } else {
        Ok(())
    }
}

/// An axis value read one way: how far it's pushed that way, from 0 to 1.
fn directed(value: f32, direction: AxisDirection) -> f32 {
    match direction {
        AxisDirection::Positive => value.max(0.0),
        AxisDirection::Negative => (-value).max(0.0),
    }
}

fn is_trigger(axis: GamepadAxis) -> bool {
    matches!(axis, GamepadAxis::LeftTrigger | GamepadAxis::RightTrigger)
}

/// A key's name in bindings: egui's, except the arrow keys, which are
/// `ArrowLeft` and so on, as scripts name them, rather than `Left`.
fn key_name(key: Key) -> &'static str {
    match key {
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::ArrowUp => "ArrowUp",
        Key::ArrowDown => "ArrowDown",
        key => key.name(),
    }
}

fn mouse_button(name: &str) -> Option<PointerButton> {
    match name.to_ascii_lowercase().as_str() {
        "left" => Some(PointerButton::Primary),
        "right" => Some(PointerButton::Secondary),
        "middle" => Some(PointerButton::Middle),
        "extra1" => Some(PointerButton::Extra1),
        "extra2" => Some(PointerButton::Extra2),
        _ => None,
    }
}

fn mouse_button_name(button: PointerButton) -> &'static str {
    match button {
        PointerButton::Primary => "left",
        PointerButton::Secondary => "right",
        PointerButton::Middle => "middle",
        PointerButton::Extra1 => "extra1",
        PointerButton::Extra2 => "extra2",
    }
}

/// A button's main script name (`"a"`, `"start"`…).
fn button_name(button: GamepadButton) -> &'static str {
    GAMEPAD_BUTTONS
        .iter()
        .find(|(candidate, _)| *candidate == button)
        .map_or("a", |(_, names)| names[0])
}

/// An axis' main script name (`"left_x"`, `"rt"`…).
fn axis_name(axis: GamepadAxis) -> &'static str {
    GAMEPAD_AXES
        .iter()
        .find(|(candidate, _)| *candidate == axis)
        .map_or("left_x", |(_, names)| names[0])
}

fn button_label(button: GamepadButton) -> &'static str {
    match button {
        GamepadButton::South => "A",
        GamepadButton::East => "B",
        GamepadButton::West => "X",
        GamepadButton::North => "Y",
        GamepadButton::LeftBumper => "LB",
        GamepadButton::RightBumper => "RB",
        GamepadButton::LeftTrigger => "LT",
        GamepadButton::RightTrigger => "RT",
        GamepadButton::Select => "Back",
        GamepadButton::Start => "Start",
        GamepadButton::LeftStick => "left stick press",
        GamepadButton::RightStick => "right stick press",
        GamepadButton::DPadUp => "d-pad up",
        GamepadButton::DPadDown => "d-pad down",
        GamepadButton::DPadLeft => "d-pad left",
        GamepadButton::DPadRight => "d-pad right",
    }
}

fn axis_label(axis: GamepadAxis, direction: AxisDirection) -> &'static str {
    use AxisDirection::{Negative, Positive};
    match (axis, direction) {
        (GamepadAxis::LeftStickX, Negative) => "left stick left",
        (GamepadAxis::LeftStickX, Positive) => "left stick right",
        (GamepadAxis::LeftStickY, Negative) => "left stick up",
        (GamepadAxis::LeftStickY, Positive) => "left stick down",
        (GamepadAxis::RightStickX, Negative) => "right stick left",
        (GamepadAxis::RightStickX, Positive) => "right stick right",
        (GamepadAxis::RightStickY, Negative) => "right stick up",
        (GamepadAxis::RightStickY, Positive) => "right stick down",
        (GamepadAxis::LeftTrigger, _) => "left trigger",
        (GamepadAxis::RightTrigger, _) => "right trigger",
    }
}
