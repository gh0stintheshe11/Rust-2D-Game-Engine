use std::cell::RefCell;
use std::rc::Rc;

use egui::{Key, PointerButton};

use super::LuaScripting;
use crate::input_handler::InputHandler;

fn parse_mouse_button(name: &str) -> Result<PointerButton, mlua::Error> {
    match name.to_lowercase().as_str() {
        "left" => Ok(PointerButton::Primary),
        "right" => Ok(PointerButton::Secondary),
        "middle" => Ok(PointerButton::Middle),
        other => Err(mlua::Error::external(format!(
            "Invalid mouse button '{}' (expected \"left\", \"right\" or \"middle\")",
            other
        ))),
    }
}

impl LuaScripting {
    pub(crate) fn register_input_bindings(
        &mut self,
        input_handler: &Rc<RefCell<InputHandler>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // is_key_just_pressed(key): true only on the frame the key went down
        let input = Rc::clone(input_handler);
        let is_key_just_pressed = self.lua.create_function(move |_, key: String| {
            let parsed_key = Key::from_name(&key)
                .ok_or_else(|| mlua::Error::external(format!("Invalid key '{}'", key)))?;
            Ok(input.borrow().is_key_just_pressed(parsed_key))
        })?;
        globals.set("is_key_just_pressed", is_key_just_pressed)?;

        // is_key_pressed(key): true while the key is held down
        let input = Rc::clone(input_handler);
        let is_key_pressed = self.lua.create_function(move |_, key: String| {
            let parsed_key = Key::from_name(&key)
                .ok_or_else(|| mlua::Error::external(format!("Invalid key '{}'", key)))?;
            Ok(input.borrow().is_key_pressed(parsed_key))
        })?;
        globals.set("is_key_pressed", is_key_pressed)?;

        // is_mouse_pressed("left" | "right" | "middle")
        let input = Rc::clone(input_handler);
        let is_mouse_pressed = self.lua.create_function(move |_, button: String| {
            let button = parse_mouse_button(&button)?;
            Ok(input.borrow().is_mouse_button_pressed(button))
        })?;
        globals.set("is_mouse_pressed", is_mouse_pressed)?;

        // get_mouse_position() -> {x, y} in window coordinates
        let input = Rc::clone(input_handler);
        let get_mouse_position = self.lua.create_function(move |lua, ()| {
            let pos = input.borrow().get_mouse_pos();
            let table = lua.create_table()?;
            table.set("x", pos.x)?;
            table.set("y", pos.y)?;
            Ok(table)
        })?;
        globals.set("get_mouse_position", get_mouse_position)?;

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

        Ok(())
    }
}
