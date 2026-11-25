//! The system's gamepads, read through gilrs and turned into
//! [`GamepadEvent`]s.

use super::{GamepadAxis, GamepadButton, GamepadEvent};
use crate::logger::LOGGER;

/// Reads the gamepads connected to the computer.
pub struct GamepadBackend {
    gilrs: gilrs::Gilrs,
    // Whether the pads connected before the first poll were announced
    announced: bool,
}

impl GamepadBackend {
    /// Start reading the system's gamepads; `None`, with a message in the
    /// console, where that isn't possible (no gamepad support on the
    /// system, or no access to its devices).
    pub fn open() -> Option<Self> {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Some(Self {
                gilrs,
                announced: false,
            }),
            Err(gilrs::Error::NotImplemented(_)) => {
                LOGGER.info("Gamepads aren't supported on this system");
                None
            }
            Err(e) => {
                LOGGER.warning(format!("Can't read gamepads: {}", e));
                None
            }
        }
    }

    /// The gamepad events since the last call; the first call also
    /// announces the pads connected before it.
    pub fn poll(&mut self) -> Vec<GamepadEvent> {
        let mut events = Vec::new();
        if !self.announced {
            self.announced = true;
            events.extend(
                self.gilrs
                    .gamepads()
                    .map(|(id, _)| GamepadEvent::Connected { gamepad: id.into() }),
            );
        }
        while let Some(event) = self.gilrs.next_event() {
            if let Some(raw) = Raw::of(event.event) {
                convert(usize::from(event.id), raw, &mut events);
            }
        }
        events
    }
}

/// The part of a gilrs event the engine uses.
#[derive(Clone, Copy, Debug)]
enum Raw {
    Connected,
    Disconnected,
    Pressed(gilrs::Button),
    Released(gilrs::Button),
    /// An analog button's value, 0 to 1
    Changed(gilrs::Button, f32),
    /// An axis' value, -1 to 1 (+y up)
    Axis(gilrs::Axis, f32),
}

impl Raw {
    fn of(event: gilrs::EventType) -> Option<Self> {
        use gilrs::EventType;
        Some(match event {
            EventType::Connected => Raw::Connected,
            EventType::Disconnected => Raw::Disconnected,
            EventType::ButtonPressed(button, _) => Raw::Pressed(button),
            EventType::ButtonReleased(button, _) => Raw::Released(button),
            EventType::ButtonChanged(button, value, _) => Raw::Changed(button, value),
            EventType::AxisChanged(axis, value, _) => Raw::Axis(axis, value),
            _ => return None,
        })
    }
}

/// The engine's events for a gilrs event.
fn convert(gamepad: usize, event: Raw, events: &mut Vec<GamepadEvent>) {
    use gilrs::{Axis, Button};
    let axis = |axis, value| GamepadEvent::AxisMoved {
        gamepad,
        axis,
        value,
    };
    match event {
        Raw::Connected => events.push(GamepadEvent::Connected { gamepad }),
        Raw::Disconnected => events.push(GamepadEvent::Disconnected { gamepad }),
        Raw::Pressed(button) => {
            if let Some(button) = button_of(button) {
                events.push(GamepadEvent::ButtonPressed { gamepad, button });
            }
        }
        Raw::Released(button) => {
            if let Some(button) = button_of(button) {
                events.push(GamepadEvent::ButtonReleased { gamepad, button });
            }
        }
        // Analog triggers
        Raw::Changed(Button::LeftTrigger2, value) => {
            events.push(axis(GamepadAxis::LeftTrigger, value));
        }
        Raw::Changed(Button::RightTrigger2, value) => {
            events.push(axis(GamepadAxis::RightTrigger, value));
        }
        Raw::Changed(..) => {}
        // gilrs has +y up; the engine, like the world, +y down
        Raw::Axis(Axis::LeftStickX, value) => events.push(axis(GamepadAxis::LeftStickX, value)),
        Raw::Axis(Axis::LeftStickY, value) => events.push(axis(GamepadAxis::LeftStickY, -value)),
        Raw::Axis(Axis::RightStickX, value) => events.push(axis(GamepadAxis::RightStickX, value)),
        Raw::Axis(Axis::RightStickY, value) => {
            events.push(axis(GamepadAxis::RightStickY, -value));
        }
        // A d-pad reported as axes, by pads without a known mapping
        Raw::Axis(Axis::DPadX, value) => {
            dpad(
                gamepad,
                value,
                GamepadButton::DPadLeft,
                GamepadButton::DPadRight,
                events,
            );
        }
        Raw::Axis(Axis::DPadY, value) => {
            dpad(
                gamepad,
                value,
                GamepadButton::DPadDown,
                GamepadButton::DPadUp,
                events,
            );
        }
        Raw::Axis(..) => {}
    }
}

/// D-pad presses for a d-pad axis: `negative` is held below -0.5,
/// `positive` above 0.5, neither in between.
fn dpad(
    gamepad: usize,
    value: f32,
    negative: GamepadButton,
    positive: GamepadButton,
    events: &mut Vec<GamepadEvent>,
) {
    for (button, held) in [(negative, value < -0.5), (positive, value > 0.5)] {
        events.push(if held {
            GamepadEvent::ButtonPressed { gamepad, button }
        } else {
            GamepadEvent::ButtonReleased { gamepad, button }
        });
    }
}

fn button_of(button: gilrs::Button) -> Option<GamepadButton> {
    use gilrs::Button;
    Some(match button {
        Button::South => GamepadButton::South,
        Button::East => GamepadButton::East,
        Button::West => GamepadButton::West,
        Button::North => GamepadButton::North,
        Button::LeftTrigger => GamepadButton::LeftBumper,
        Button::RightTrigger => GamepadButton::RightBumper,
        Button::LeftTrigger2 => GamepadButton::LeftTrigger,
        Button::RightTrigger2 => GamepadButton::RightTrigger,
        Button::Select => GamepadButton::Select,
        Button::Start => GamepadButton::Start,
        Button::LeftThumb => GamepadButton::LeftStick,
        Button::RightThumb => GamepadButton::RightStick,
        Button::DPadUp => GamepadButton::DPadUp,
        Button::DPadDown => GamepadButton::DPadDown,
        Button::DPadLeft => GamepadButton::DPadLeft,
        Button::DPadRight => GamepadButton::DPadRight,
        Button::C | Button::Z | Button::Mode | Button::Unknown => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gilrs_events_become_engine_events() {
        let mut events = Vec::new();
        for raw in [
            Raw::Pressed(gilrs::Button::LeftTrigger),
            Raw::Axis(gilrs::Axis::LeftStickY, 0.5),
            Raw::Changed(gilrs::Button::RightTrigger2, 0.7),
            Raw::Axis(gilrs::Axis::DPadX, -1.0),
            Raw::Pressed(gilrs::Button::Mode),
        ] {
            convert(3, raw, &mut events);
        }
        assert_eq!(
            events,
            vec![
                GamepadEvent::ButtonPressed {
                    gamepad: 3,
                    button: GamepadButton::LeftBumper
                },
                GamepadEvent::AxisMoved {
                    gamepad: 3,
                    axis: GamepadAxis::LeftStickY,
                    value: -0.5
                },
                GamepadEvent::AxisMoved {
                    gamepad: 3,
                    axis: GamepadAxis::RightTrigger,
                    value: 0.7
                },
                GamepadEvent::ButtonPressed {
                    gamepad: 3,
                    button: GamepadButton::DPadLeft
                },
                GamepadEvent::ButtonReleased {
                    gamepad: 3,
                    button: GamepadButton::DPadRight
                },
            ]
        );
    }
}
