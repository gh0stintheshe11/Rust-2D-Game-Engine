use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::{parse_uuid, LuaScripting};
use crate::ecs::{AttributeType, AttributeValue, SceneManager};
use crate::gui::scene_hierarchy::predefined_entities::PREDEFINED_ENTITIES;
use crate::project_manager::ProjectManager;

impl LuaScripting {
    pub(crate) fn register_ecs_bindings(
        &mut self,
        scene_manager: &Rc<RefCell<SceneManager>>,
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

        let manager = Rc::clone(scene_manager);
        let remove_entity =
            self.lua
                .create_function(move |_, (scene_id, entity_id): (String, String)| {
                    let mut manager = manager.borrow_mut();
                    let scene_uuid = parse_uuid(&scene_id, "scene")?;
                    let entity_uuid = parse_uuid(&entity_id, "entity")?;
                    let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                        mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                    })?;
                    let success = scene.delete_entity(entity_uuid).map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to delete entity '{}': {}",
                            entity_uuid, e
                        ))
                    })?;
                    Ok(success)
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

                // Assign default attributes based on the predefined Physics entity
                if let Ok(entity) = scene.get_entity_mut(entity_id) {
                    if let Some(predefined) =
                        PREDEFINED_ENTITIES.iter().find(|e| e.name == "Physics")
                    {
                        for (attr_name, attr_type, attr_value) in predefined.attributes.iter() {
                            let _ = entity.create_attribute(
                                attr_name,
                                attr_type.clone(),
                                attr_value.clone(),
                            );
                        }
                    }

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

        // Position setters
        macro_rules! register_setter {
            ($lua_name:literal, $method:ident) => {{
                let manager = Rc::clone(scene_manager);
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
                    let project_path = ProjectManager::get_project_path().ok_or_else(|| {
                        mlua::Error::external("No project is currently open".to_string())
                    })?;
                    let full_image_path = PathBuf::from(project_path).join(&image_path);
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
                    let project_path = ProjectManager::get_project_path().ok_or_else(|| {
                        mlua::Error::external("No project is currently open".to_string())
                    })?;
                    let full_script_path = PathBuf::from(project_path).join(&script_path);
                    entity.set_script(full_script_path).map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to set script for entity '{}': {}",
                            entity_uuid, e
                        ))
                    })?;
                    Ok(())
                })?;
        globals.set("set_script", set_script)?;

        let manager = Rc::clone(scene_manager);
        let update_entity_attribute_bool = self.lua.create_function(
            move |_, (scene_id, entity_id, attr_name, value): (String, String, String, bool)| {
                let mut manager = manager.borrow_mut();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
                let scene = manager.get_scene_mut(scene_uuid).ok_or_else(|| {
                    mlua::Error::external(format!("Scene '{}' not found", scene_uuid))
                })?;
                let attr_id = scene
                    .get_entity(entity_uuid)
                    .map_err(|e| {
                        mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e))
                    })?
                    .get_attribute_by_name(&attr_name)
                    .map_err(|e| {
                        mlua::Error::external(format!("Attribute '{}' not found: {}", attr_name, e))
                    })?
                    .id;
                scene
                    .update_entity_attribute(entity_uuid, attr_id, AttributeValue::Boolean(value))
                    .map_err(|e| {
                        mlua::Error::external(format!(
                            "Failed to update attribute '{}': {}",
                            attr_name, e
                        ))
                    })?;
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
        //   The Lua value is coerced to the attribute's declared type;
        //   a mismatch or a missing attribute raises an error.
        let manager = Rc::clone(scene_manager);
        let set_attribute = self.lua.create_function(
            move |_, (scene_id, entity_id, name, value): (String, String, String, mlua::Value)| {
                let mut manager = manager.borrow_mut();
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;
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
                let attr_type = attr.data_type.clone();

                let type_error = |expected: &str, got: &mlua::Value| {
                    mlua::Error::external(format!(
                        "Attribute '{}' is {}, got Lua {}",
                        name,
                        expected,
                        got.type_name()
                    ))
                };

                let new_value = match attr_type {
                    AttributeType::Float => match &value {
                        mlua::Value::Number(n) => AttributeValue::Float(*n as f32),
                        mlua::Value::Integer(i) => AttributeValue::Float(*i as f32),
                        other => return Err(type_error("a Float", other)),
                    },
                    AttributeType::Integer => match &value {
                        mlua::Value::Integer(i) => AttributeValue::Integer(*i as i32),
                        mlua::Value::Number(n) if n.fract() == 0.0 => {
                            AttributeValue::Integer(*n as i32)
                        }
                        other => return Err(type_error("an Integer", other)),
                    },
                    AttributeType::Boolean => match &value {
                        mlua::Value::Boolean(b) => AttributeValue::Boolean(*b),
                        other => return Err(type_error("a Boolean", other)),
                    },
                    AttributeType::String => match &value {
                        mlua::Value::String(s) => AttributeValue::String(s.to_str()?.to_string()),
                        other => return Err(type_error("a String", other)),
                    },
                    AttributeType::Vector2 => match &value {
                        mlua::Value::Table(t) => {
                            let x: f32 = t.get("x").or_else(|_| t.get(1))?;
                            let y: f32 = t.get("y").or_else(|_| t.get(2))?;
                            AttributeValue::Vector2(x, y)
                        }
                        other => return Err(type_error("a Vector2 ({x, y} table)", other)),
                    },
                };

                entity
                    .modify_attribute(attr_id, None, None, Some(new_value))
                    .map_err(mlua::Error::external)?;
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
