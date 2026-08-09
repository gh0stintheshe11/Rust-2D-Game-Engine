use std::cell::RefCell;
use std::rc::Rc;

use egui::Key;

use super::LuaScripting;
use crate::input_handler::InputHandler;

impl LuaScripting {
    pub(crate) fn register_input_bindings(
        &mut self,
        input_handler: &Rc<RefCell<InputHandler>>,
    ) -> Result<(), mlua::Error> {
        let input = Rc::clone(input_handler);
        let is_key_just_pressed = self.lua.create_function(move |_, key: String| {
            let parsed_key = Key::from_name(&key)
                .ok_or_else(|| mlua::Error::external(format!("Invalid key '{}'", key)))?;
            Ok(input.borrow().is_key_just_pressed(parsed_key))
        })?;
        self.lua
            .globals()
            .set("is_key_just_pressed", is_key_just_pressed)?;

        Ok(())
    }
}
