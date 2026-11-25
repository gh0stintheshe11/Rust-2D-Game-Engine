use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Lua, Value};
use serde_json::{Map, Value as Json};

use super::LuaScripting;
use crate::logger::LOGGER;
use crate::save::{SaveData, SaveError};

/// Deepest nesting of tables `save_data` accepts (also stops cycles).
const MAX_DEPTH: usize = 32;

fn save_error(message: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::external(format!("save_data: {}", message))
}

/// A Lua value as JSON: booleans, finite numbers, strings, and tables that
/// are lists (keys 1..n) or have string keys, nested. Everything else (nil
/// inside tables can't occur), functions, NaN and infinities are errors.
fn to_json(value: &Value, depth: usize) -> mlua::Result<Json> {
    match value {
        Value::Boolean(b) => Ok(Json::Bool(*b)),
        Value::Integer(i) => Ok(Json::from(*i)),
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(Json::Number)
            .ok_or_else(|| save_error(format!("can't save the number {}", n))),
        Value::String(s) => Ok(Json::String(s.to_str()?.to_string())),
        Value::Table(table) => {
            if depth >= MAX_DEPTH {
                return Err(save_error(format!(
                    "tables nested more than {} deep (or containing themselves)",
                    MAX_DEPTH
                )));
            }
            let pairs: Vec<(Value, Value)> =
                table.pairs::<Value, Value>().collect::<mlua::Result<_>>()?;
            let is_list = pairs.iter().all(
                |(key, _)| matches!(key, Value::Integer(i) if *i >= 1 && *i <= pairs.len() as i64),
            );
            if is_list && !pairs.is_empty() {
                let mut items = vec![Json::Null; pairs.len()];
                for (key, value) in &pairs {
                    if let Value::Integer(i) = key {
                        items[*i as usize - 1] = to_json(value, depth + 1)?;
                    }
                }
                return Ok(Json::Array(items));
            }
            let mut object = Map::new();
            for (key, value) in &pairs {
                let Value::String(key) = key else {
                    return Err(save_error(
                        "tables must be lists or have string keys, not a mix",
                    ));
                };
                object.insert(key.to_str()?.to_string(), to_json(value, depth + 1)?);
            }
            Ok(Json::Object(object))
        }
        other => Err(save_error(format!("can't save a {}", other.type_name()))),
    }
}

/// JSON back into Lua: lists become sequences, objects tables with string
/// keys; integers stay integers.
fn to_lua(lua: &Lua, json: &Json) -> mlua::Result<Value> {
    Ok(match json {
        Json::Null => Value::Nil,
        Json::Bool(b) => Value::Boolean(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Integer(i),
            None => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
        },
        Json::String(s) => Value::String(lua.create_string(s)?),
        Json::Array(items) => {
            let table = lua.create_table()?;
            for (index, item) in items.iter().enumerate() {
                table.set(index + 1, to_lua(lua, item)?)?;
            }
            Value::Table(table)
        }
        Json::Object(object) => {
            let table = lua.create_table()?;
            for (key, value) in object {
                table.set(key.as_str(), to_lua(lua, value)?)?;
            }
            Value::Table(table)
        }
    })
}

impl LuaScripting {
    pub(crate) fn register_save_bindings(
        &mut self,
        save_data: &Rc<RefCell<SaveData>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // save_data(key, value) -> true when written to disk: keep a value
        // between runs of the game; nil removes it
        let save = Rc::clone(save_data);
        let save_data_fn = self
            .lua
            .create_function(move |_, (key, value): (String, Value)| {
                // Convert before borrowing the save data; nothing calls back
                // into Lua meanwhile
                let json = match value {
                    Value::Nil => None,
                    value => Some(to_json(&value, 0)?),
                };
                let result = save.borrow_mut().set(&key, json);
                match result {
                    Ok(()) => Ok(true),
                    Err(SaveError::TooLarge) => Err(save_error(SaveError::TooLarge)),
                    Err(SaveError::Io(e)) => {
                        LOGGER.warning(format!("save_data(\"{}\"): {}", key, e));
                        Ok(false)
                    }
                }
            })?;
        globals.set("save_data", save_data_fn)?;

        // load_data(key) -> the value saved under the key, or nil
        let save = Rc::clone(save_data);
        let load_data = self.lua.create_function(move |lua, key: String| {
            let json = save.borrow().get(&key).cloned();
            match json {
                Some(json) => to_lua(lua, &json),
                None => Ok(Value::Nil),
            }
        })?;
        globals.set("load_data", load_data)?;

        Ok(())
    }
}
