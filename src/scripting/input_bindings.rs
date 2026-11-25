use std::cell::RefCell;
use std::rc::Rc;

use super::LuaScripting;
use crate::input::{InputHandler, parse_key, parse_mouse_button};

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

        Ok(())
    }
}
