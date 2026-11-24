use std::cell::RefCell;
use std::rc::Rc;

use super::{LuaScripting, parse_uuid};
use crate::physics::PhysicsEngine;
use crate::project::ProjectManager;
use crate::scene::{AttributeType, AttributeValue, SceneManager};

impl LuaScripting {
    pub(crate) fn register_scene_bindings(
        &mut self,
        scene_manager: &Rc<RefCell<SceneManager>>,
        physics_engine: &Rc<RefCell<PhysicsEngine>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        let manager = Rc::clone(scene_manager);
        let add_entity =
            self.lua
                .create_function(move |_, (scene_id, entity_name): (String, String)| {
                    let mut manager = manager.borrow_mut();
                    let scene_uuid = parse_uuid(&scene_id, "scene")?;
                    let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                        mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                    })?;
                    let entity_id = scene.create_entity(&entity_name).map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to create entity '{}': {}",
                            entity_name, e
                        ))
                    })?;
                    Ok(entity_id.to_string())
                })?;
        globals.set("add_entity", add_entity)?;

        // remove_entity also removes the entity's physics body, so it doesn't
        // linger as an invisible obstacle
        let manager = Rc::clone(scene_manager);
        let physics = Rc::clone(physics_engine);
        let remove_entity =
            self.lua
                .create_function(move |_, (scene_id, entity_id): (String, String)| {
                    let scene_uuid = parse_uuid(&scene_id, "scene")?;
                    let entity_uuid = parse_uuid(&entity_id, "entity")?;
                    let removed = {
                        let mut manager = manager.borrow_mut();
                        let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                            mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                        })?;
                        scene.delete_entity(entity_uuid).map_err(|e| {
                            mlua::Error::external(format!(
                                "Failed to delete entity '{}': {}",
                                entity_uuid, e
                            ))
                        })?
                    };
                    if removed {
                        physics.borrow_mut().remove_entity(entity_uuid);
                    }
                    Ok(removed)
                })?;
        globals.set("remove_entity", remove_entity)?;

        let manager = Rc::clone(scene_manager);
        let create_physical_entity = self.lua.create_function(
            move |_, (scene_id, name, x, y, z): (String, String, f32, f32, f32)| {
                let mut manager = manager.borrow_mut();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                    mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                })?;
                let entity_id = scene.create_entity(&name).map_err(|e| {
                    mlua::Error::external(format!(
                        "Failed to create physical entity '{}': {}",
                        name, e
                    ))
                })?;

                // Assign default attributes from the built-in Physics template
                if let Ok(entity) = scene.get_entity_mut(entity_id) {
                    entity.add_physics_attributes();

                    // Apply the requested spawn position
                    entity.set_position(x, y, z).map_err(|e| {
                        mlua::Error::external(format!("Failed to set spawn position: {}", e))
                    })?;
                    if let Ok(pos_attr) = entity.get_attribute_by_name("position") {
                        let pos_id = pos_attr.id;
                        let _ = entity.modify_attribute(
                            pos_id,
                            None,
                            None,
                            Some(AttributeValue::Vector2(x, y)),
                        );
                    }
                } else {
                    return Err(mlua::Error::external(format!(
                        "Failed to retrieve entity with ID '{}'",
                        entity_id
                    )));
                }

                Ok(entity_id.to_string())
            },
        )?;
        globals.set("create_physical_entity", create_physical_entity)?;

        // Position setters. Entities with a physics body are teleported too;
        // otherwise the next physics step would move them back.
        macro_rules! register_setter {
            ($lua_name:literal, $method:ident) => {{
                let manager = Rc::clone(scene_manager);
                let physics = Rc::clone(physics_engine);
                let setter = self.lua.create_function(
                    move |_, (scene_id, entity_id, value): (String, String, f32)| {
                        let mut manager = manager.borrow_mut();
                        let scene_uuid = parse_uuid(&scene_id, "scene")?;
                        let entity_uuid = parse_uuid(&entity_id, "entity")?;
                        let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                            mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                        })?;
                        let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                            mlua::Error::external(format!(
                                "Entity '{}' not found: {}",
                                entity_uuid, e
                            ))
                        })?;
                        entity.$method(value).map_err(|e| {
                            mlua::Error::external(format!(
                                "Failed to set {}: {}",
                                stringify!($method),
                                e
                            ))
                        })?;
                        physics.borrow_mut().set_position(
                            entity_uuid,
                            entity.get_x(),
                            entity.get_y(),
                        );
                        Ok(())
                    },
                )?;
                globals.set($lua_name, setter)?;
            }};
        }

        register_setter!("set_x", set_x);
        register_setter!("set_y", set_y);
        register_setter!("set_z", set_z);

        let manager = Rc::clone(scene_manager);
        let physics = Rc::clone(physics_engine);
        let set_position = self.lua.create_function(
            move |_, (scene_id, entity_id, x, y): (String, String, f32, f32)| {
                let mut manager = manager.borrow_mut();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
                let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                    mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                })?;
                let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                    mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                })?;
                // Only touch x/y; leave z as-is
                entity
                    .set_x(x)
                    .and_then(|_| entity.set_y(y))
                    .map_err(|e| mlua::Error::external(format!("Failed to set position: {}", e)))?;
                physics.borrow_mut().set_position(entity_uuid, x, y);
                Ok(())
            },
        )?;
        globals.set("set_position", set_position)?;

        let manager = Rc::clone(scene_manager);
        let add_image =
            self.lua
                .create_function(move |_, (entity_id, image_path): (String, String)| {
                    let mut manager = manager.borrow_mut();
                    let entity_uuid = parse_uuid(&entity_id, "entity")?;
                    let scene = manager
                        .get_active_scene_mut()
                        .ok_or_else(|| mlua::Error::external("No active scene found"))?;
                    let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                        mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                    })?;
                    let full_image_path = ProjectManager::resolve_asset_path(&image_path);
                    entity.add_image(full_image_path).map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to add image to entity '{}': {}",
                            entity_uuid, e
                        ))
                    })?;
                    Ok(())
                })?;
        globals.set("add_image", add_image)?;

        let manager = Rc::clone(scene_manager);
        let set_script =
            self.lua
                .create_function(move |_, (entity_id, script_path): (String, String)| {
                    let mut manager = manager.borrow_mut();
                    let entity_uuid = parse_uuid(&entity_id, "entity")?;
                    let scene = manager
                        .get_active_scene_mut()
                        .ok_or_else(|| mlua::Error::external("No active scene found"))?;
                    let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                        mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                    })?;
                    let full_script_path = ProjectManager::resolve_asset_path(&script_path);
                    // Replaces the entity's current script, if any
                    let _ = entity.remove_script();
                    entity.set_script(full_script_path).map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to set script for entity '{}': {}",
                            entity_uuid, e
                        ))
                    })?;
                    Ok(())
                })?;
        globals.set("set_script", set_script)?;

        // update_entity_attribute_bool(scene_id, entity_id, name, value):
        // set a Boolean attribute (older spelling of set_attribute)
        let manager = Rc::clone(scene_manager);
        let update_entity_attribute_bool = self.lua.create_function(
            move |_, (scene_id, entity_id, attr_name, value): (String, String, String, bool)| {
                let mut manager = manager.borrow_mut();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
                let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                    mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                })?;
                let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                    mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                })?;
                let attr = entity.get_attribute_by_name(&attr_name).map_err(|e| {
                    mlua::Error::external(format!("Attribute '{}' not found: {}", attr_name, e))
                })?;
                if !matches!(attr.data_type, AttributeType::Boolean) {
                    return Err(mlua::Error::external(format!(
                        "Attribute '{}' is not a Boolean",
                        attr_name
                    )));
                }
                let attr_id = attr.id;
                entity
                    .modify_attribute(attr_id, None, None, Some(AttributeValue::Boolean(value)))
                    .map_err(mlua::Error::external)?;
                Ok(())
            },
        )?;
        globals.set("update_entity_attribute_bool", update_entity_attribute_bool)?;

        // Attribute creators
        macro_rules! register_attribute_creator {
            ($lua_name:literal, $rust_ty:ty, $attr_type:expr, $to_value:expr) => {{
                let manager = Rc::clone(scene_manager);
                let creator = self.lua.create_function(
                    move |_,
                          (scene_id, entity_id, attr_name, value): (
                        String,
                        String,
                        String,
                        $rust_ty,
                    )| {
                        let mut manager = manager.borrow_mut();
                        let scene_uuid = parse_uuid(&scene_id, "scene")?;
                        let entity_uuid = parse_uuid(&entity_id, "entity")?;
                        let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                            mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                        })?;
                        let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                            mlua::Error::external(format!(
                                "Entity '{}' not found: {}",
                                entity_uuid, e
                            ))
                        })?;
                        entity
                            .create_attribute(&attr_name, $attr_type, $to_value(value))
                            .map_err(|e| {
                                mlua::Error::external(format!(
                                    "Failed to create attribute '{}': {}",
                                    attr_name, e
                                ))
                            })?;
                        Ok(())
                    },
                )?;
                globals.set($lua_name, creator)?;
            }};
        }

        register_attribute_creator!(
            "create_attribute_float",
            f32,
            AttributeType::Float,
            AttributeValue::Float
        );
        register_attribute_creator!(
            "create_attribute_bool",
            bool,
            AttributeType::Boolean,
            AttributeValue::Boolean
        );

        let manager = Rc::clone(scene_manager);
        let create_attribute_vector2 = self.lua.create_function(
            move |_, (scene_id, entity_id, attr_name, x, y): (String, String, String, f32, f32)| {
                let mut manager = manager.borrow_mut();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
                let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                    mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                })?;
                let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                    mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                })?;
                entity
                    .create_attribute(
                        &attr_name,
                        AttributeType::Vector2,
                        AttributeValue::Vector2(x, y),
                    )
                    .map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to create vector2 attribute '{}': {}",
                            attr_name, e
                        ))
                    })?;
                Ok(())
            },
        )?;
        globals.set("create_attribute_vector2", create_attribute_vector2)?;

        let manager = Rc::clone(scene_manager);
        let list_entities_name_x_y = self.lua.create_function(move |lua, scene_id: String| {
            let manager = manager.borrow();
            let scene_uuid = parse_uuid(&scene_id, "scene")?;
            let scene = manager.get_scene(scene_uuid).ok_or_else(|| {
                mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
            })?;

            let lua_table = lua.create_table()?;
            for (index, (entity_id, entity)) in scene.entities.iter().enumerate() {
                let entity_data = lua.create_table()?;
                entity_data.set("id", entity_id.to_string())?;
                entity_data.set("name", entity.name.clone())?;
                // x/y are kept in sync by the physics engine each step
                entity_data.set("x", entity.get_x())?;
                entity_data.set("y", entity.get_y())?;
                lua_table.set(index + 1, entity_data)?;
            }
            Ok(lua_table)
        })?;
        globals.set("list_entities_name_x_y", list_entities_name_x_y)?;

        // === Generic attribute access ===
        // These let scripts use inspector-defined attributes as game
        // variables: get_attribute / set_attribute / has_attribute.

        // get_attribute(scene_id, entity_id, name)
        //   -> number | boolean | string | {x, y} table, or nil if missing
        let manager = Rc::clone(scene_manager);
        let get_attribute = self.lua.create_function(
            move |lua, (scene_id, entity_id, name): (String, String, String)| {
                let manager = manager.borrow();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;

                let attr_value = manager
                    .get_scene(scene_uuid)
                    .and_then(|scene| scene.get_entity(entity_uuid).ok())
                    .and_then(|entity| {
                        entity
                            .get_attribute_by_name(&name)
                            .ok()
                            .map(|attr| attr.value.clone())
                    });

                let Some(value) = attr_value else {
                    return Ok(mlua::Value::Nil);
                };

                Ok(match value {
                    AttributeValue::Integer(i) => mlua::Value::Integer(i as i64),
                    AttributeValue::Float(f) => mlua::Value::Number(f as f64),
                    AttributeValue::Boolean(b) => mlua::Value::Boolean(b),
                    AttributeValue::String(s) => mlua::Value::String(lua.create_string(&s)?),
                    AttributeValue::Vector2(x, y) => {
                        let table = lua.create_table()?;
                        table.set("x", x)?;
                        table.set("y", y)?;
                        mlua::Value::Table(table)
                    }
                })
            },
        )?;
        globals.set("get_attribute", get_attribute)?;

        // set_attribute(scene_id, entity_id, name, value)
        //   The Lua value is coerced to the attribute's declared type; a
        //   mismatch or a missing attribute raises an error. Setting x, y or
        //   position also moves the entity's physics body.
        let manager = Rc::clone(scene_manager);
        let physics = Rc::clone(physics_engine);
        let set_attribute = self.lua.create_function(
            move |_, (scene_id, entity_id, name, value): (String, String, String, mlua::Value)| {
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
                // Read the Lua value completely before borrowing the scene:
                // nothing may call back into Lua while the borrow is held
                let value = ScriptValue::from_lua(&value)?;

                let mut manager = manager.borrow_mut();
                let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                    mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                })?;
                let entity = scene.get_entity_mut(entity_uuid).map_err(|e| {
                    mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                })?;
                let attr = entity.get_attribute_by_name(&name).map_err(|e| {
                    mlua::Error::external(format!("Attribute '{}' not found: {}", name, e))
                })?;
                let attr_id = attr.id;
                let new_value = value
                    .coerce(&attr.data_type)
                    .map_err(|e| mlua::Error::external(format!("Attribute '{}' {}", name, e)))?;
                if let AttributeValue::Vector2(x, y) = new_value
                    && name == "position"
                {
                    entity.set_x(x).map_err(mlua::Error::external)?;
                    entity.set_y(y).map_err(mlua::Error::external)?;
                }
                entity
                    .modify_attribute(attr_id, None, None, Some(new_value))
                    .map_err(mlua::Error::external)?;
                if matches!(name.as_str(), "x" | "y" | "position") {
                    physics
                        .borrow_mut()
                        .set_position(entity_uuid, entity.get_x(), entity.get_y());
                }
                Ok(())
            },
        )?;
        globals.set("set_attribute", set_attribute)?;

        // has_attribute(scene_id, entity_id, name) -> bool
        let manager = Rc::clone(scene_manager);
        let has_attribute = self.lua.create_function(
            move |_, (scene_id, entity_id, name): (String, String, String)| {
                let manager = manager.borrow();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
                Ok(manager
                    .get_scene(scene_uuid)
                    .and_then(|scene| scene.get_entity(entity_uuid).ok())
                    .map(|entity| entity.get_attribute_by_name(&name).is_ok())
                    .unwrap_or(false))
            },
        )?;
        globals.set("has_attribute", has_attribute)?;

        // get_entity_name(scene_id, entity_id) -> name string, or nil if the
        // entity no longer exists
        let manager = Rc::clone(scene_manager);
        let get_entity_name =
            self.lua
                .create_function(move |_, (scene_id, entity_id): (String, String)| {
                    let manager = manager.borrow();
                    let scene_uuid = parse_uuid(&scene_id, "scene")?;
                    let entity_uuid = parse_uuid(&entity_id, "entity")?;
                    let name = manager
                        .get_scene(scene_uuid)
                        .and_then(|scene| scene.entities.get(&entity_uuid))
                        .map(|entity| entity.name.clone());
                    Ok(name)
                })?;
        globals.set("get_entity_name", get_entity_name)?;

        Ok(())
    }
}

/// A Lua value converted to Rust, so engine state can be borrowed without
/// any Lua code (e.g. a table's `__index`) running meanwhile.
enum ScriptValue {
    Number(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    Vector2(f32, f32),
    Other(&'static str),
}

impl ScriptValue {
    fn from_lua(value: &mlua::Value) -> mlua::Result<Self> {
        Ok(match value {
            mlua::Value::Number(n) => ScriptValue::Number(*n),
            mlua::Value::Integer(i) => ScriptValue::Integer(*i),
            mlua::Value::Boolean(b) => ScriptValue::Boolean(*b),
            mlua::Value::String(s) => ScriptValue::String(s.to_str()?.to_string()),
            mlua::Value::Table(table) => {
                // {x = .., y = ..} or {.., ..}; raw reads skip metamethods
                let component = |name: &str, index: i64| -> mlua::Result<Option<f32>> {
                    let mut value: mlua::Value = table.raw_get(name)?;
                    if value.is_nil() {
                        value = table.raw_get(index)?;
                    }
                    Ok(match value {
                        mlua::Value::Number(n) => Some(n as f32),
                        mlua::Value::Integer(i) => Some(i as f32),
                        _ => None,
                    })
                };
                match (component("x", 1)?, component("y", 2)?) {
                    (Some(x), Some(y)) => ScriptValue::Vector2(x, y),
                    _ => ScriptValue::Other("table"),
                }
            }
            other => ScriptValue::Other(other.type_name()),
        })
    }

    fn type_name(&self) -> &'static str {
        match self {
            ScriptValue::Number(_) | ScriptValue::Integer(_) => "number",
            ScriptValue::Boolean(_) => "boolean",
            ScriptValue::String(_) => "string",
            ScriptValue::Vector2(_, _) => "{x, y} table",
            ScriptValue::Other(name) => name,
        }
    }

    /// Convert to an attribute value of `data_type`, or explain why not.
    fn coerce(self, data_type: &AttributeType) -> Result<AttributeValue, String> {
        let got = self.type_name();
        let mismatch = |expected: &str| format!("is {}, got Lua {}", expected, got);
        match (data_type, self) {
            (AttributeType::Float, ScriptValue::Number(n)) => Ok(AttributeValue::Float(n as f32)),
            (AttributeType::Float, ScriptValue::Integer(i)) => Ok(AttributeValue::Float(i as f32)),
            (AttributeType::Integer, ScriptValue::Integer(i)) => i32::try_from(i)
                .map(AttributeValue::Integer)
                .map_err(|_| format!("is an Integer; {} is out of range", i)),
            (AttributeType::Integer, ScriptValue::Number(n))
                if n.fract() == 0.0 && n >= i32::MIN as f64 && n <= i32::MAX as f64 =>
            {
                Ok(AttributeValue::Integer(n as i32))
            }
            (AttributeType::Integer, _) => Err(mismatch("an Integer")),
            (AttributeType::Boolean, ScriptValue::Boolean(b)) => Ok(AttributeValue::Boolean(b)),
            (AttributeType::String, ScriptValue::String(s)) => Ok(AttributeValue::String(s)),
            (AttributeType::Vector2, ScriptValue::Vector2(x, y)) => {
                Ok(AttributeValue::Vector2(x, y))
            }
            (AttributeType::Float, _) => Err(mismatch("a Float")),
            (AttributeType::Boolean, _) => Err(mismatch("a Boolean")),
            (AttributeType::String, _) => Err(mismatch("a String")),
            (AttributeType::Vector2, _) => Err(mismatch("a Vector2 ({x, y} table)")),
        }
    }
}
