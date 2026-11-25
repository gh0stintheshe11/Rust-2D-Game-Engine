use std::cell::RefCell;
use std::rc::Rc;

use super::LuaScripting;
use crate::input::{
    GAMEPAD_AXES, GAMEPAD_BUTTONS, GamepadAxis, GamepadButton, InputHandler, parse_gamepad_axis,
    parse_gamepad_button, parse_key, parse_mouse_button,
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

        Ok(())
    }
}
