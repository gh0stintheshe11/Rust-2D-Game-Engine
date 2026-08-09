use std::cell::RefCell;
use std::rc::Rc;

use rapier2d::prelude::*;

use super::{parse_uuid, LuaScripting};
use crate::ecs::SceneManager;
use crate::physics_engine::PhysicsEngine;

impl LuaScripting {
    pub(crate) fn register_physics_bindings(
        &mut self,
        physics_engine: &Rc<RefCell<PhysicsEngine>>,
        scene_manager: &Rc<RefCell<SceneManager>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        let physics = Rc::clone(physics_engine);
        let set_velocity =
            self.lua
                .create_function(move |_, (entity_id, x, y): (String, f32, f32)| {
                    let mut physics = physics.borrow_mut();
                    let uuid = parse_uuid(&entity_id, "entity")?;
                    if !physics.has_rigid_body(&uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.set_velocity(&uuid, Vector::new(x, y));
                    Ok(())
                })?;
        globals.set("set_velocity", set_velocity)?;

        let physics = Rc::clone(physics_engine);
        let apply_force =
            self.lua
                .create_function(move |_, (entity_id, x, y): (String, f32, f32)| {
                    let mut physics = physics.borrow_mut();
                    let uuid = parse_uuid(&entity_id, "entity")?;
                    if !physics.has_rigid_body(&uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.apply_force(&uuid, Vector::new(x, y));
                    Ok(())
                })?;
        globals.set("apply_force", apply_force)?;

        let physics = Rc::clone(physics_engine);
        let apply_impulse =
            self.lua
                .create_function(move |_, (entity_id, x, y): (String, f32, f32)| {
                    let mut physics = physics.borrow_mut();
                    let uuid = parse_uuid(&entity_id, "entity")?;
                    if !physics.has_rigid_body(&uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.apply_impulse(&uuid, Vector::new(x, y));
                    Ok(())
                })?;
        globals.set("apply_impulse", apply_impulse)?;

        let physics = Rc::clone(physics_engine);
        let manager = Rc::clone(scene_manager);
        let add_entity_to_physics_engine =
            self.lua.create_function(move |_, entity_id: String| {
                let uuid = parse_uuid(&entity_id, "entity")?;
                let manager = manager.borrow();
                if let Some(active_scene) = manager.get_active_scene() {
                    if let Some(entity) = active_scene.entities.get(&uuid) {
                        physics.borrow_mut().add_entity(entity);
                        return Ok(());
                    }
                }
                Err(mlua::Error::external(format!(
                    "Entity '{}' not found in active scene",
                    uuid
                )))
            })?;
        globals.set("add_entity_to_physics_engine", add_entity_to_physics_engine)?;

        let physics = Rc::clone(physics_engine);
        let remove_entity_from_physics_engine =
            self.lua.create_function(move |_, entity_id: String| {
                let uuid = parse_uuid(&entity_id, "entity")?;
                physics.borrow_mut().remove_entity(uuid);
                Ok(())
            })?;
        globals.set(
            "remove_entity_from_physics_engine",
            remove_entity_from_physics_engine,
        )?;

        // get_colliding_entities(entity_id) -> array of entity id strings.
        // Returns an empty table for entities not in the physics engine.
        let physics = Rc::clone(physics_engine);
        let get_colliding_entities = self.lua.create_function(move |lua, entity_id: String| {
            let uuid = parse_uuid(&entity_id, "entity")?;
            let colliding = physics.borrow().get_colliding_entities(&uuid);
            let table = lua.create_table()?;
            for (index, id) in colliding.iter().enumerate() {
                table.set(index + 1, id.to_string())?;
            }
            Ok(table)
        })?;
        globals.set("get_colliding_entities", get_colliding_entities)?;

        Ok(())
    }
}
