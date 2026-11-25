use std::cell::RefCell;
use std::rc::Rc;

use super::{LuaScripting, parse_uuid};
use crate::physics::PhysicsEngine;
use crate::scene::SceneManager;

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
                    if !physics.has_rigid_body(uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.set_velocity(uuid, (x, y));
                    Ok(())
                })?;
        globals.set("set_velocity", set_velocity)?;

        // get_velocity(entity_id) -> vx, vy: the body's velocity, px/s (+y
        // is down), e.g. to change one direction and keep the other
        let physics = Rc::clone(physics_engine);
        let get_velocity = self.lua.create_function(move |_, entity_id: String| {
            let uuid = parse_uuid(&entity_id, "entity")?;
            physics.borrow().get_velocity(uuid).ok_or_else(|| {
                mlua::Error::external(format!("Entity '{}' not found in physics engine", uuid))
            })
        })?;
        globals.set("get_velocity", get_velocity)?;

        // drop_through_platforms(entity_id): the body falls through the
        // one-way platforms it stands on, until it's clear of them (press
        // down to drop); nothing happens on solid ground
        let physics = Rc::clone(physics_engine);
        let drop_through_platforms = self.lua.create_function(move |_, entity_id: String| {
            let uuid = parse_uuid(&entity_id, "entity")?;
            if physics.borrow_mut().drop_through_platforms(uuid) {
                Ok(())
            } else {
                Err(mlua::Error::external(format!(
                    "Entity '{}' not found in physics engine",
                    uuid
                )))
            }
        })?;
        globals.set("drop_through_platforms", drop_through_platforms)?;

        let physics = Rc::clone(physics_engine);
        let apply_force =
            self.lua
                .create_function(move |_, (entity_id, x, y): (String, f32, f32)| {
                    let mut physics = physics.borrow_mut();
                    let uuid = parse_uuid(&entity_id, "entity")?;
                    if !physics.has_rigid_body(uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.apply_force(uuid, (x, y));
                    Ok(())
                })?;
        globals.set("apply_force", apply_force)?;

        // set_angular_velocity(entity_id, radians_per_second); +y is down,
        // so positive values spin clockwise on screen
        let physics = Rc::clone(physics_engine);
        let set_angular_velocity =
            self.lua
                .create_function(move |_, (entity_id, velocity): (String, f32)| {
                    let mut physics = physics.borrow_mut();
                    let uuid = parse_uuid(&entity_id, "entity")?;
                    if !physics.has_rigid_body(uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.set_angular_velocity(uuid, velocity);
                    Ok(())
                })?;
        globals.set("set_angular_velocity", set_angular_velocity)?;

        let physics = Rc::clone(physics_engine);
        let apply_impulse =
            self.lua
                .create_function(move |_, (entity_id, x, y): (String, f32, f32)| {
                    let mut physics = physics.borrow_mut();
                    let uuid = parse_uuid(&entity_id, "entity")?;
                    if !physics.has_rigid_body(uuid) {
                        return Err(mlua::Error::external(format!(
                            "Entity '{}' not found in physics engine",
                            uuid
                        )));
                    }
                    physics.apply_impulse(uuid, (x, y));
                    Ok(())
                })?;
        globals.set("apply_impulse", apply_impulse)?;

        let physics = Rc::clone(physics_engine);
        let manager = Rc::clone(scene_manager);
        let add_entity_to_physics_engine =
            self.lua.create_function(move |_, entity_id: String| {
                let uuid = parse_uuid(&entity_id, "entity")?;
                let manager = manager.borrow();
                if let Some(active_scene) = manager.get_active_scene()
                    && let Some(entity) = active_scene.entities.get(&uuid)
                {
                    physics.borrow_mut().add_entity(entity);
                    return Ok(());
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
            let colliding = physics.borrow().get_colliding_entities(uuid);
            let table = lua.create_table()?;
            for (index, id) in colliding.iter().enumerate() {
                table.set(index + 1, id.to_string())?;
            }
            Ok(table)
        })?;
        globals.set("get_colliding_entities", get_colliding_entities)?;

        // set_gravity(x, y): change the global gravity (screen space: +y is down)
        let physics = Rc::clone(physics_engine);
        let set_gravity = self.lua.create_function(move |_, (x, y): (f32, f32)| {
            physics.borrow_mut().set_gravity(x, y);
            Ok(())
        })?;
        globals.set("set_gravity", set_gravity)?;

        // raycast(x1, y1, x2, y2 [, exclude_entity_id]) -> {id, x, y, distance}
        // of the first collider the segment hits (sensors ignored), or nil
        let physics = Rc::clone(physics_engine);
        let raycast = self.lua.create_function(
            move |lua, (x1, y1, x2, y2, exclude): (f32, f32, f32, f32, Option<String>)| {
                let exclude = exclude.map(|id| parse_uuid(&id, "entity")).transpose()?;
                let hit = physics.borrow().raycast((x1, y1), (x2, y2), exclude);
                let Some(hit) = hit else {
                    return Ok(None);
                };
                let table = lua.create_table()?;
                table.set("id", hit.entity_id.to_string())?;
                table.set("x", hit.point.0)?;
                table.set("y", hit.point.1)?;
                table.set("distance", hit.distance)?;
                Ok(Some(table))
            },
        )?;
        globals.set("raycast", raycast)?;

        Ok(())
    }
}
