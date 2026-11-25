use std::cell::RefCell;
use std::rc::Rc;

use super::LuaScripting;
use crate::input::{
    ActionState, GAMEPAD_AXES, GAMEPAD_BUTTONS, GamepadAxis, GamepadButton, InputBinding,
    InputHandler, parse_gamepad_axis, parse_gamepad_button, parse_key, parse_mouse_button,
};

fn key(name: &str) -> Result<egui::Key, mlua::Error> {
    parse_key(name).ok_or_else(|| mlua::Error::external(format!("Invalid key '{}'", name)))
}

fn mouse_button(name: &str) -> Result<egui::PointerButton, mlua::Error> {
    parse_mouse_button(name).ok_or_else(|| {
        mlua::Error::external(format!(
            "Invalid mouse button '{}' (expected \"left\", \"right\" or \"middle\")",
            name
        ))
    })
}

fn gamepad_button(function: &str, name: &str) -> Result<GamepadButton, mlua::Error> {
    parse_gamepad_button(name).ok_or_else(|| {
        let names: Vec<&str> = GAMEPAD_BUTTONS.iter().map(|(_, names)| names[0]).collect();
        mlua::Error::external(format!(
            "{}: unknown gamepad button '{}' (expected {})",
            function,
            name,
            names.join(", ")
        ))
    })
}

fn gamepad_axis(name: &str) -> Result<GamepadAxis, mlua::Error> {
    parse_gamepad_axis(name).ok_or_else(|| {
        let names: Vec<&str> = GAMEPAD_AXES.iter().map(|(_, names)| names[0]).collect();
        mlua::Error::external(format!(
            "get_gamepad_axis: unknown axis '{}' (expected {})",
            name,
            names.join(", ")
        ))
    })
}

/// A gamepad number from scripts (from 1) as an index (from 0); `None`
/// stands for any gamepad.
fn gamepad_index(function: &str, pad: Option<i64>) -> Result<Option<usize>, mlua::Error> {
    match pad {
        None => Ok(None),
        Some(pad) if pad >= 1 => Ok(Some((pad - 1) as usize)),
        Some(pad) => Err(mlua::Error::external(format!(
            "{}: gamepads are numbered from 1, got {}",
            function, pad
        ))),
    }
}

/// An input action's state, or an error naming the actions there are.
fn action(input: &InputHandler, function: &str, name: &str) -> Result<ActionState, mlua::Error> {
    input.action(name).ok_or_else(|| {
        let names: Vec<&str> = input.actions().iter().map(|(name, _)| name).collect();
        let known = if names.is_empty() {
            "the project has no input actions".to_string()
        } else {
            format!("expected {}", names.join(", "))
        };
        mlua::Error::external(format!(
            "{}: unknown input action '{}' ({})",
            function, name, known
        ))
    })
}

impl LuaScripting {
    pub(crate) fn register_input_bindings(
        &mut self,
        input_handler: &Rc<RefCell<InputHandler>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // is_key_just_pressed(key): true only on the frame the key went down.
        // Key names are egui's ("Space", "A", "ArrowUp"), case-insensitive.
        let input = Rc::clone(input_handler);
        let is_key_just_pressed = self.lua.create_function(move |_, name: String| {
            Ok(input.borrow().is_key_just_pressed(key(&name)?))
        })?;
        globals.set("is_key_just_pressed", is_key_just_pressed)?;

        // is_key_just_released(key): true only on the frame the key went up
        let input = Rc::clone(input_handler);
        let is_key_just_released = self.lua.create_function(move |_, name: String| {
            Ok(input.borrow().is_key_just_released(key(&name)?))
        })?;
        globals.set("is_key_just_released", is_key_just_released)?;

        // is_key_pressed(key): true while the key is held down
        let input = Rc::clone(input_handler);
        let is_key_pressed = self.lua.create_function(move |_, name: String| {
            Ok(input.borrow().is_key_pressed(key(&name)?))
        })?;
        globals.set("is_key_pressed", is_key_pressed)?;

        // is_mouse_pressed("left" | "right" | "middle"): true while held
        let input = Rc::clone(input_handler);
        let is_mouse_pressed = self.lua.create_function(move |_, name: String| {
            Ok(input.borrow().is_mouse_button_pressed(mouse_button(&name)?))
        })?;
        globals.set("is_mouse_pressed", is_mouse_pressed)?;

        // is_mouse_just_pressed(button) / is_mouse_just_released(button):
        // true only on the frame the button went down / up
        let input = Rc::clone(input_handler);
        let is_mouse_just_pressed = self.lua.create_function(move |_, name: String| {
            Ok(input
                .borrow()
                .is_mouse_button_just_pressed(mouse_button(&name)?))
        })?;
        globals.set("is_mouse_just_pressed", is_mouse_just_pressed)?;
        let input = Rc::clone(input_handler);
        let is_mouse_just_released = self.lua.create_function(move |_, name: String| {
            Ok(input
                .borrow()
                .is_mouse_button_just_released(mouse_button(&name)?))
        })?;
        globals.set("is_mouse_just_released", is_mouse_just_released)?;

        // get_mouse_position() -> {x, y} in world coordinates: where the
        // pointer is in the game, through the camera
        let input = Rc::clone(input_handler);
        let get_mouse_position = self.lua.create_function(move |lua, ()| {
            let (x, y) = input.borrow().get_mouse_world_pos();
            let table = lua.create_table()?;
            table.set("x", x)?;
            table.set("y", y)?;
            Ok(table)
        })?;
        globals.set("get_mouse_position", get_mouse_position)?;

        // get_mouse_screen_position() -> {x, y} in the camera frame's pixels,
        // the coordinates draw_text uses
        let input = Rc::clone(input_handler);
        let get_mouse_screen_position = self.lua.create_function(move |lua, ()| {
            let (x, y) = input.borrow().get_mouse_screen_pos();
            let table = lua.create_table()?;
            table.set("x", x)?;
            table.set("y", y)?;
            Ok(table)
        })?;
        globals.set("get_mouse_screen_position", get_mouse_screen_position)?;

        // get_scroll_delta() -> {x, y} (zero when not scrolling)
        let input = Rc::clone(input_handler);
        let get_scroll_delta = self.lua.create_function(move |lua, ()| {
            let delta = input
                .borrow()
                .get_scroll_delta()
                .unwrap_or(egui::Vec2::ZERO);
            let table = lua.create_table()?;
            table.set("x", delta.x)?;
            table.set("y", delta.y)?;
            Ok(table)
        })?;
        globals.set("get_scroll_delta", get_scroll_delta)?;

        // is_gamepad_pressed(button [, pad]), is_gamepad_just_pressed,
        // is_gamepad_just_released: button names after an Xbox pad ("a",
        // "b", "x", "y", "lb", "rb", "lt", "rt", "back", "start", "ls",
        // "rs", "up", "down", "left", "right"); `pad` counts from 1 in the
        // order pads connected, and without it any pad counts
        type ButtonQuery = fn(&InputHandler, GamepadButton, Option<usize>) -> bool;
        let queries: [(&str, ButtonQuery); 3] = [
            ("is_gamepad_pressed", InputHandler::is_gamepad_pressed),
            (
                "is_gamepad_just_pressed",
                InputHandler::is_gamepad_just_pressed,
            ),
            (
                "is_gamepad_just_released",
                InputHandler::is_gamepad_just_released,
            ),
        ];
        for (name, query) in queries {
            let input = Rc::clone(input_handler);
            let function =
                self.lua
                    .create_function(move |_, (button, pad): (String, Option<i64>)| {
                        let button = gamepad_button(name, &button)?;
                        let pad = gamepad_index(name, pad)?;
                        Ok(query(&input.borrow(), button, pad))
                    })?;
            globals.set(name, function)?;
        }

        // get_gamepad_axis(axis [, pad]): "left_x", "left_y", "right_x",
        // "right_y" from -1 to 1 (+y is down), "lt", "rt" from 0 to 1;
        // without `pad`, the first pad where the axis isn't at rest
        let input = Rc::clone(input_handler);
        let get_gamepad_axis =
            self.lua
                .create_function(move |_, (axis, pad): (String, Option<i64>)| {
                    let axis = gamepad_axis(&axis)?;
                    let pad = gamepad_index("get_gamepad_axis", pad)?;
                    Ok(input.borrow().gamepad_axis(axis, pad))
                })?;
        globals.set("get_gamepad_axis", get_gamepad_axis)?;

        // get_gamepad_count(): how many gamepads are connected
        let input = Rc::clone(input_handler);
        let get_gamepad_count = self
            .lua
            .create_function(move |_, ()| Ok(input.borrow().gamepad_count()))?;
        globals.set("get_gamepad_count", get_gamepad_count)?;

        self.register_action_bindings(input_handler)
    }

    /// Input actions: the project's named controls ("jump", "left"), each
    /// bound to keys, mouse buttons and gamepad controls (see
    /// [`crate::input::InputActions`]).
    fn register_action_bindings(
        &mut self,
        input_handler: &Rc<RefCell<InputHandler>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // is_action_pressed(action): true while it's held, through any of
        // its bindings; is_action_just_pressed(action) /
        // is_action_just_released(action): true only on the frame it went
        // down / up. An action the project doesn't have is an error.
        type StateQuery = fn(&ActionState) -> bool;
        let queries: [(&str, StateQuery); 3] = [
            ("is_action_pressed", |state| state.pressed),
            ("is_action_just_pressed", |state| state.just_pressed),
            ("is_action_just_released", |state| state.just_released),
        ];
        for (name, query) in queries {
            let input = Rc::clone(input_handler);
            let function = self.lua.create_function(move |_, action_name: String| {
                Ok(query(&action(&input.borrow(), name, &action_name)?))
            })?;
            globals.set(name, function)?;
        }

        // get_action_strength(action): how far it's held, from 0 to 1 (a
        // stick or trigger bound to it can hold it part way)
        let input = Rc::clone(input_handler);
        let get_action_strength = self.lua.create_function(move |_, name: String| {
            Ok(action(&input.borrow(), "get_action_strength", &name)?.strength)
        })?;
        globals.set("get_action_strength", get_action_strength)?;

        // get_action_axis(negative, positive): the strength of `positive`
        // minus that of `negative`, from -1 to 1, e.g. ("left", "right")
        let input = Rc::clone(input_handler);
        let get_action_axis =
            self.lua
                .create_function(move |_, (negative, positive): (String, String)| {
                    let input = input.borrow();
                    let negative = action(&input, "get_action_axis", &negative)?;
                    let positive = action(&input, "get_action_axis", &positive)?;
                    Ok(positive.strength - negative.strength)
                })?;
        globals.set("get_action_axis", get_action_axis)?;

        // get_action_bindings(action) -> {"key:Space", "pad:a", ...}
        let input = Rc::clone(input_handler);
        let get_action_bindings = self.lua.create_function(move |_, name: String| {
            let input = input.borrow();
            action(&input, "get_action_bindings", &name)?;
            let bindings = input.actions().bindings(&name).unwrap_or_default();
            Ok(bindings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>())
        })?;
        globals.set("get_action_bindings", get_action_bindings)?;

        // set_action_bindings(action, {"key:Space", "pad:a", ...}): bind an
        // action, adding it if it's new, for the rest of the run (a
        // controls menu); invalid bindings are an error
        let input = Rc::clone(input_handler);
        let set_action_bindings =
            self.lua
                .create_function(move |_, (name, texts): (String, Vec<String>)| {
                    let bindings = texts
                        .iter()
                        .map(|text| InputBinding::parse(text))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| {
                            mlua::Error::external(format!("set_action_bindings: {}", e))
                        })?;
                    input
                        .borrow_mut()
                        .set_action_bindings(&name, bindings)
                        .map_err(|e| mlua::Error::external(format!("set_action_bindings: {}", e)))
                })?;
        globals.set("set_action_bindings", set_action_bindings)?;

        // get_just_pressed_binding() -> what went down this frame as a
        // binding ("key:Q", "mouse:left", "pad:y", "pad:left_x-"), or nil:
        // for "press a key" menus
        let input = Rc::clone(input_handler);
        let get_just_pressed_binding = self.lua.create_function(move |_, ()| {
            Ok(input
                .borrow()
                .just_pressed_binding()
                .map(|binding| binding.to_string()))
        })?;
        globals.set("get_just_pressed_binding", get_just_pressed_binding)?;

        Ok(())
    }
}
