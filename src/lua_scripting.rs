use mlua::{Lua, Value as LuaValue, Result as LuaResult, Function as LuaFunction, Table as LuaTable};
use crate::ecs::SceneManager;
use crate::ecs::AttributeType;
use crate::ecs::AttributeValue;
use serde_json::Value as JsonValue;
use uuid::Uuid;
use std::fs;
use mlua::{LuaSerdeExt, UserData};
use crate::physics_engine::PhysicsEngine;
use rapier2d::prelude::*;
use std::path::PathBuf;
use crate::gui::scene_hierarchy::predefined_entities::PREDEFINED_ENTITIES;
use crate::project_manager::ProjectManager;

pub struct LuaScripting {
    pub lua: Lua,
}

impl LuaScripting {
    pub fn new() -> Self {
        LuaScripting {
            lua: Lua::new(),
        }
    }
}


impl LuaScripting {

    // This is for binding physics engine functions to Lua
    pub fn initialize_bindings_physics_engine(&mut self, physics_engine: &mut PhysicsEngine, scene_manager: &mut SceneManager) -> Result<(), mlua::Error> {
        let physics_engine_ref = physics_engine as *mut PhysicsEngine;
        let scene_manager_ref = scene_manager as *const SceneManager;

        // Binding set_velocity
        let set_velocity = self.lua.create_function(move |_, (entity_id, x, y): (String, f32, f32)| {
            let physics_engine = unsafe { &mut *physics_engine_ref };

            let uuid = Uuid::parse_str(&entity_id).map_err(|e| {
                eprintln!("Invalid UUID '{}': {}", entity_id, e);
                mlua::Error::external(format!("Invalid UUID '{}': {}", entity_id, e))
            })?;

            if !physics_engine.has_rigid_body(&uuid) {
                eprintln!("Entity '{}' not found in physics engine.", uuid);
                return Err(mlua::Error::external(format!(
                    "Entity '{}' not found in physics engine",
                    uuid
                )));
            }

            // Set the velocity
            let velocity = vector![x, y];
            // eprintln!("Setting velocity for entity '{}': {:?}", uuid, velocity);
            physics_engine.set_velocity(&uuid, velocity);

            Ok(())
        })?;
        self.lua.globals().set("set_velocity", set_velocity)?;


        // Binding add_entity (Rust) to add_entity_to_physics_engine (Lua)
        let add_entity_to_physics_engine = self.lua.create_function(move |_, entity_id: String| {
            let physics_engine = unsafe { &mut *physics_engine_ref };
            let scene_manager = unsafe { &*scene_manager_ref };

            let uuid = Uuid::parse_str(&entity_id).map_err(|e| {
                mlua::Error::external(format!("Invalid UUID '{}': {}", entity_id, e))
            })?;

            if let Some(active_scene) = scene_manager.get_active_scene() {
                if let Some(entity) = active_scene.entities.get(&uuid) {
                    physics_engine.add_entity(entity);
                    // println!("Entity '{}' added to physics engine.", uuid);
                    return Ok(());
                }
            }

            Err(mlua::Error::external(format!(
                "Entity '{}' not found in active scene",
                uuid
            )))
        })?;
        self.lua.globals().set("add_entity_to_physics_engine", add_entity_to_physics_engine)?;

        // Binding remove_entity (Rust) to remove_entity_from_physics_engine (Lua)
        let remove_entity_from_physics_engine = self.lua.create_function(move |_, entity_id: String| {
            let physics_engine = unsafe { &mut *physics_engine_ref };

            let uuid = Uuid::parse_str(&entity_id).map_err(|e| {
                mlua::Error::external(format!("Invalid UUID '{}': {}", entity_id, e))
            })?;

            physics_engine.remove_entity(uuid);
            // println!("Entity '{}' removed from physics engine.", uuid);
            return Ok(());
        })?;
        self.lua.globals().set("remove_entity_from_physics_engine", remove_entity_from_physics_engine)?;

        println!("Lua physics engine bindings initialized successfully.");
        Ok(())
    }


    // This is for binding ECS functions to Lua
    pub fn initialize_bindings_ecs(&mut self, scene_manager: &mut SceneManager) -> Result<(), mlua::Error> {
        let scene_manager_ref = scene_manager as *mut SceneManager;

        // Binding add_entity
        let add_entity = self.lua.create_function(move |_, (scene_id, entity_name): (String, String)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };
            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity_id = scene.create_entity(&entity_name)
                .map_err(|e| mlua::Error::external(format!("Failed to create entity '{}': {}", entity_name, e)))?;

            match scene.get_entity(entity_id) {
                Ok(entity) => {
                    println!("Entity '{}' created in scene '{}':", entity_id, scene_uuid);
                    for (attr_name, attr) in &entity.attributes {
                        println!(
                            "  Attribute: {} -> {:?} (Type: {:?})",
                            attr_name, attr.value, attr.data_type
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "Entity '{}' created but failed to retrieve attributes for debug: {}",
                        entity_id, e
                    );
                }
            }

            // println!("Entity '{}' created in scene '{}'", entity_id, scene_uuid);
            Ok(entity_id.to_string())
        })?;
        self.lua.globals().set("add_entity", add_entity)?;

        // Binding delete_entity
        let remove_entity = self.lua.create_function(move |_, (scene_id, entity_id): (String, String)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };
            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let success = scene.delete_entity(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Failed to delete entity '{}': {}", entity_uuid, e)))?;

            if success {
                println!("Entity '{}' deleted from scene '{}'", entity_uuid, scene_uuid);
            } else {
                println!("Entity '{}' not found in scene '{}'", entity_uuid, scene_uuid);
            }

            Ok(success)
        })?;
        self.lua.globals().set("remove_entity", remove_entity)?;

        // This uses create_entity, and add attributes inside the Lua, due to somehow create_physical_entity not working
        let create_physical_entity = self.lua.create_function(move |_, (scene_id, name, x, y, z): (String, String, f32, f32, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };
            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity_id = scene.create_entity(&name)
                .map_err(|e| mlua::Error::external(format!("Failed to create physical entity '{}': {}", name, e)))?;


            // Assign default attributes based on predefined entities
            if let Ok(entity) = scene.get_entity_mut(entity_id) {
                if let Some(predefined) = PREDEFINED_ENTITIES.iter().find(|e| e.name == "Physics") {
                    for (attr_name, attr_type, attr_value) in predefined.attributes.iter() {
                        // Create attributes for the entity
                        let _ = entity.create_attribute(attr_name, attr_type.clone(), attr_value.clone());
                    }
                }
            } else {
                return Err(mlua::Error::external(format!(
                    "Failed to retrieve entity with ID '{}'",
                    entity_id
                )));
            }

            // println!("Physical entity '{}' created in scene '{}'", entity_id, scene_uuid);
            Ok(entity_id.to_string())
        })?;

        self.lua.globals().set("create_physical_entity", create_physical_entity)?;

        // Update Single Entity Attribute Binding
        let set_x = self.lua.create_function(move |_, (scene_id, entity_id, value): (String, String, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.set_x(value)
                .map_err(|e| mlua::Error::external(format!("Failed to set x: {}", e)))?;

            // println!(
            //     "Set x to '{}' for entity '{}' in scene '{}'",
            //     value, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("set_x", set_x)?;

        let set_y = self.lua.create_function(move |_, (scene_id, entity_id, value): (String, String, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.set_y(value)
                .map_err(|e| mlua::Error::external(format!("Failed to set y: {}", e)))?;

            // println!(
            //     "Set y to '{}' for entity '{}' in scene '{}'",
            //     value, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("set_y", set_y)?;

        let set_z = self.lua.create_function(move |_, (scene_id, entity_id, value): (String, String, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.set_z(value)
                .map_err(|e| mlua::Error::external(format!("Failed to set z: {}", e)))?;

            // println!(
            //     "Set z to '{}' for entity '{}' in scene '{}'",
            //     value, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("set_z", set_z)?;

        let set_position = self.lua.create_function(move |_, (scene_id, entity_id, x, y): (String, String, f32, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.set_position(x, y, 0.0)
                .map_err(|e| mlua::Error::external(format!("Failed to set position: {}", e)))?;

            // println!(
            //     "Set position to '({}, {}, 0.0)' for entity '{}' in scene '{}'",
            //     x, y, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("set_position", set_position)?;


        // Add Image Binding
        let add_image = self.lua.create_function(move |_, (entity_id, image_path): (String, String)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };
            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_active_scene_mut()
                .ok_or_else(|| mlua::Error::external("No active scene found"))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            let full_image_path = PathBuf::from(ProjectManager::get_project_path().unwrap().to_string()).join(&image_path);
            entity.add_image(full_image_path)
                .map_err(|e| mlua::Error::external(format!("Failed to add image to entity '{}': {}", entity_uuid, e)))?;

            // println!("Image added to entity '{}'", entity_uuid);
            Ok(())
        })?;
        self.lua.globals().set("add_image", add_image)?;

        // Set Script Binding
        let set_script = self.lua.create_function(move |_, (entity_id, script_path): (String, String)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };
            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_active_scene_mut()
                .ok_or_else(|| mlua::Error::external("No active scene found"))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            let full_script_path = PathBuf::from(ProjectManager::get_project_path().unwrap().to_string()).join(&script_path);
            entity.set_script(full_script_path)
                .map_err(|e| mlua::Error::external(format!("Failed to set script for entity '{}': {}", entity_uuid, e)))?;

            // println!("Script set for entity '{}'", entity_uuid);
            Ok(())
        })?;
        self.lua.globals().set("set_script", set_script)?;


        let update_entity_attribute_bool = self.lua.create_function(move |_, (scene_id, entity_id, attr_name, value): (String, String, String, bool)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            // Parse UUIDs
            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;
            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            // Find the scene
            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            // Find the entity
            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            // Find the attribute ID using the attribute name
            let attr_id = entity
                .get_attribute_by_name(&attr_name)
                .map_err(|e| mlua::Error::external(format!("Attribute '{}' not found: {}", attr_name, e)))?
                .id;

            // Update the attribute value
            scene
                .update_entity_attribute(entity_uuid, attr_id, AttributeValue::Boolean(value))
                .map_err(|e| mlua::Error::external(format!("Failed to update attribute '{}': {}", attr_name, e)))?;

            println!(
                "Boolean attribute '{}' updated to '{}' for entity '{}' in scene '{}'",
                attr_name, value, entity_uuid, scene_uuid
            );
            Ok(())
        })?;
        self.lua.globals().set("update_entity_attribute_bool", update_entity_attribute_bool)?;



        // Function to create a float attribute
        let create_attribute_float = self.lua.create_function(move |_, (scene_id, entity_id, attr_name, value): (String, String, String, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;
            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.create_attribute(&attr_name, AttributeType::Float, AttributeValue::Float(value))
                .map_err(|e| mlua::Error::external(format!("Failed to create float attribute '{}': {}", attr_name, e)))?;

            // println!(
            //     "Float attribute '{}' with value '{}' created for entity '{}' in scene '{}'",
            //     attr_name, value, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("create_attribute_float", create_attribute_float)?;

        // Function to create a bool attribute
        let create_attribute_bool = self.lua.create_function(move |_, (scene_id, entity_id, attr_name, value): (String, String, String, bool)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;
            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.create_attribute(&attr_name, AttributeType::Boolean, AttributeValue::Boolean(value))
                .map_err(|e| mlua::Error::external(format!("Failed to create boolean attribute '{}': {}", attr_name, e)))?;

            // println!(
            //     "Boolean attribute '{}' with value '{}' created for entity '{}' in scene '{}'",
            //     attr_name, value, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("create_attribute_bool", create_attribute_bool)?;

        // Function to create a vector2 attribute
        let create_attribute_vector2 = self.lua.create_function(move |_, (scene_id, entity_id, attr_name, x, y): (String, String, String, f32, f32)| {
            let scene_manager = unsafe { &mut *scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;
            let entity_uuid = Uuid::parse_str(&entity_id)
                .map_err(|e| mlua::Error::external(format!("Invalid entity UUID '{}': {}", entity_id, e)))?;

            let scene = scene_manager
                .get_scene_mut(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let entity = scene.get_entity_mut(entity_uuid)
                .map_err(|e| mlua::Error::external(format!("Entity '{}' not found: {}", entity_uuid, e)))?;

            entity.create_attribute(&attr_name, AttributeType::Vector2, AttributeValue::Vector2(x, y))
                .map_err(|e| mlua::Error::external(format!("Failed to create vector2 attribute '{}': {}", attr_name, e)))?;

            // println!(
            //     "Vector2 attribute '{}' with value '({}, {})' created for entity '{}' in scene '{}'",
            //     attr_name, x, y, entity_uuid, scene_uuid
            // );
            Ok(())
        })?;
        self.lua.globals().set("create_attribute_vector2", create_attribute_vector2)?;


        // TODO: list_entities_name_x_y is not working, need to be fixed
        let list_entities_name_x_y = self.lua.create_function(move |lua, scene_id: String| {
            let scene_manager = unsafe { &*scene_manager_ref };

            let scene_uuid = Uuid::parse_str(&scene_id)
                .map_err(|e| mlua::Error::external(format!("Invalid scene UUID '{}': {}", scene_id, e)))?;

            let scene = scene_manager
                .get_scene(scene_uuid)
                .ok_or_else(|| mlua::Error::external(format!("Scene '{}' not found", scene_uuid)))?;

            let lua_table = lua.create_table()?;

            for (index, (entity_id, entity)) in scene.entities.iter().enumerate() {
                let name = &entity.name;

                let (x, y) = if let Ok(position_attr) = entity.get_attribute_by_name("position") {
                    if let AttributeValue::Vector2(x, y) = position_attr.value {
                        (x, y)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                // Create a table for each entity
                let entity_data = lua.create_table()?;
                entity_data.set("id", entity_id.to_string())?;
                entity_data.set("name", name.to_string())?;
                entity_data.set("x", x)?;
                entity_data.set("y", y)?;

                // println!("Entity '{}' list for scene '{}'", entity_id, name);

                // Add the entity table to the main table
                lua_table.set(index + 1, entity_data)?;
            }

            Ok(lua_table)
        })?;

        self.lua.globals().set("list_entities_name_x_y", list_entities_name_x_y)?;





        println!("ECS bindings initialized successfully.");
        Ok(())
    }



    /// Load SceneManager into Lua global space
    pub fn load_scene_manager(&mut self, scene_manager: &SceneManager) -> Result<(), mlua::Error> {
        // Serialize SceneManager as Lua userdata
        self.lua = Lua::new();
        let globals = self.lua.globals();
        globals.set("scene_manager", self.lua.to_value(scene_manager)?)?;
        Ok(())
    }

    pub fn run_scripts_for_scene(
        &self,
        scene_manager: &mut SceneManager,
        active_scene_id: Uuid,
    ) -> Result<(), String> {
        let active_scene = scene_manager
            .get_scene_mut(active_scene_id)
            .ok_or_else(|| "Active scene not found.".to_string())?;

        for (entity_id, entity) in &mut active_scene.entities {
            if let Some(script_path) = &entity.script {
                // println!("Found script for entity {}: {:?}", entity_id, script_path);

                let script_content = std::fs::read_to_string(script_path)
                    .map_err(|e| format!("Error reading script file for entity {}: {}", entity_id, e))?;

                self.lua
                    .load(&script_content)
                    .exec()
                    .map_err(|e| format!("Error executing script for entity {}: {}", entity_id, e))?;

                let update_function: LuaFunction = self
                    .lua
                    .globals()
                    .get("update")
                    .map_err(|e| format!("Error: Script for entity {} does not define update(): {}", entity_id, e))?;

                update_function
                    .call::<()>((active_scene_id.to_string(), entity_id.to_string()))
                    .map_err(|e| format!("Error executing update() in script for entity {}: {}", entity_id, e))?;
            }
        }

        println!("Lua scripts executed successfully for scene {}", active_scene_id);
        Ok(())
    }


    /// Convert JSON Value to Lua Value
    pub fn lua_to_json(&self, lua_value: LuaValue) -> Result<JsonValue, mlua::Error> {
        match lua_value {
            LuaValue::Table(table) => {
                let mut is_array = true;
                let mut array = Vec::new();
                let mut map = serde_json::Map::new();

                for pair in table.pairs::<LuaValue, LuaValue>() {
                    let (key, value) = pair?;
                    match key {
                        LuaValue::Integer(i) if i > 0 => {
                            if is_array {
                                let index = (i - 1) as usize; // Convert 1-based Lua index to 0-based
                                if index == array.len() {
                                    array.push(self.lua_to_json(value)?);
                                } else {
                                    is_array = false;
                                }
                            }
                        }
                        LuaValue::String(key_str) => {
                            is_array = false;
                            map.insert(key_str.to_str()?.to_string(), self.lua_to_json(value)?);
                        }
                        _ => {
                            return Err(mlua::Error::FromLuaConversionError {
                                from: "Lua key",
                                to: "JSON key".to_string(),
                                message: Some("Lua table keys must be strings or positive integers.".to_string()),
                            });
                        }
                    }
                }

                if is_array {
                    Ok(JsonValue::Array(array))
                } else {
                    Ok(JsonValue::Object(map))
                }
            }
            LuaValue::String(s) => Ok(JsonValue::String(s.to_str()?.to_string())),
            LuaValue::Integer(i) => Ok(JsonValue::Number(i.into())),
            LuaValue::Number(n) => Ok(JsonValue::Number(
                serde_json::Number::from_f64(n).ok_or_else(|| mlua::Error::RuntimeError("Invalid float".into()))?,
            )),
            LuaValue::Boolean(b) => Ok(JsonValue::Bool(b)),
            LuaValue::Nil => Ok(JsonValue::Null),
            LuaValue::LightUserData(_) => {
                // Skip LightUserData
                eprintln!("Warning: Skipping LightUserData during JSON conversion.");
                Ok(JsonValue::Null)
            },
            other => {
                eprintln!("Error: Unsupported Lua value type. Value: {:?}", other);
                Err(mlua::Error::FromLuaConversionError {
                    from: "Unsupported Lua value",
                    to: "JSON".to_string(),
                    message: Some(format!("Unsupported Lua value type: {:?}", other)),
                })
            }
        }
    }

    pub fn json_to_lua(&self, value: &JsonValue) -> Result<LuaTable, mlua::Error> {
        match value {
            JsonValue::Object(map) => {
                let table = self.lua.create_table()?;
                for (key, val) in map {
                    table.set(key.clone(), self.value_to_lua(val)?)?;
                }
                Ok(table)
            }
            JsonValue::Array(array) => {
                let table = self.lua.create_table()?;
                for (i, val) in array.iter().enumerate() {
                    table.raw_set((i + 1) as i64, self.value_to_lua(val)?)?; // Ensure Lua arrays are 1-based
                }
                Ok(table)
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: "json",
                to: "LuaTable".to_string(),
                message: Some("Root value must be an object or an array.".to_string()),
            }),
        }
    }

    fn value_to_lua(&self, value: &JsonValue) -> Result<LuaValue, mlua::Error> {
        match value {
            JsonValue::Object(map) => Ok(LuaValue::Table(self.json_to_lua(value)?)),
            JsonValue::Array(array) => {
                let table = self.lua.create_table()?;
                for (i, val) in array.iter().enumerate() {
                    table.raw_set((i + 1) as i64, self.value_to_lua(val)?)?; // Properly handle arrays as Lua arrays
                }
                Ok(LuaValue::Table(table))
            }
            JsonValue::String(s) => Ok(LuaValue::String(self.lua.create_string(s)?)),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(LuaValue::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(LuaValue::Number(f))
                } else {
                    Err(mlua::Error::FromLuaConversionError {
                        from: "number",
                        to: "LuaValue".to_string(),
                        message: None,
                    })
                }
            }
            JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
            JsonValue::Null => Ok(LuaValue::Nil),
        }
    }


    pub fn run_entity_scripts(
        &mut self,
        scene_manager: &mut SceneManager,
        active_scene_id: Uuid,
    ) -> Result<(), String> {
        let mut scripts_to_run = Vec::new();

        if let Some(active_scene) = scene_manager.get_scene_mut(active_scene_id) {
            for (entity_id, entity) in &active_scene.entities {
                if let Some(script_path) = &entity.script {
                    scripts_to_run.push((entity_id.clone(), script_path.clone()));
                }
            }
        } else {
            return Err("Active scene not found.".to_string());
        }

        for (entity_id, script_path) in scripts_to_run {
            // println!("Found script for entity {}: {:?}", entity_id, script_path);

            let script_content = std::fs::read_to_string(&script_path)
                .map_err(|e| format!("Error reading script file for entity {}: {}", entity_id, e))?;

            // println!("Executing script for entity {}...", entity_id);

            // Load the Lua script
            let lua_function = self.lua
                .load(&script_content)
                .into_function()
                .map_err(|e| format!("Error loading script for entity {}: {}", entity_id, e))?;

            self.execute_entity_script(lua_function, scene_manager, active_scene_id, entity_id)?;
        }

        Ok(())
    }

    fn execute_entity_script(
        &self,
        lua_function: LuaFunction,
        scene_manager: &mut SceneManager,
        scene_id: Uuid,
        entity_id: Uuid,
    ) -> Result<(), String> {
        // Serialize the scene_manager to pass to Lua
        let json_scene_manager =
            serde_json::to_value(scene_manager).map_err(|e| format!("Serialization error: {}", e))?;
        let lua_scene_manager = self
            .json_to_lua(&json_scene_manager)
            .map_err(|e| format!("Error converting JSON to Lua: {}", e))?;

        // Call the Lua function, passing the scene_id and entity_id as arguments
        lua_function
            .call::<()>((lua_scene_manager, scene_id.to_string(), entity_id.to_string()))
            .map_err(|e| format!("Error executing Lua script: {}", e))
    }


}

