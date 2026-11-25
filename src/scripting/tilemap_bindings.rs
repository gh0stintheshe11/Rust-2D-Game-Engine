use std::cell::RefCell;
use std::rc::Rc;

use super::{LuaScripting, parse_uuid};
use crate::physics::PhysicsEngine;
use crate::scene::{Entity, SceneManager};

fn tile_error(function: &str, message: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::external(format!("{}: {}", function, message))
}

/// The entity, which must have a tilemap.
fn tilemap_entity<'a>(
    manager: &'a mut SceneManager,
    function: &str,
    scene_id: &str,
    entity_id: &str,
) -> mlua::Result<&'a mut Entity> {
    let scene_uuid = parse_uuid(scene_id, "scene")?;
    let entity_uuid = parse_uuid(entity_id, "entity")?;
    let entity = manager
        .get_scene_mut(scene_uuid)
        .ok_or_else(|| tile_error(function, format!("scene '{}' not found", scene_uuid)))?
        .get_entity_mut(entity_uuid)
        .map_err(|e| tile_error(function, e))?;
    if entity.tilemap.is_none() {
        return Err(tile_error(
            function,
            format!("entity '{}' has no tilemap", entity.name),
        ));
    }
    Ok(entity)
}

impl LuaScripting {
    pub(crate) fn register_tilemap_bindings(
        &mut self,
        scene_manager: &Rc<RefCell<SceneManager>>,
        physics_engine: &Rc<RefCell<PhysicsEngine>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // get_tile(scene_id, entity_id, column, row) -> the tile number in
        // the cell, or nil when it's empty
        let manager = Rc::clone(scene_manager);
        let get_tile = self.lua.create_function(
            move |_, (scene_id, entity_id, column, row): (String, String, i32, i32)| {
                let mut manager = manager.borrow_mut();
                let entity = tilemap_entity(&mut manager, "get_tile", &scene_id, &entity_id)?;
                Ok(entity.tilemap.as_ref().and_then(|map| map.get(column, row)))
            },
        )?;
        globals.set("get_tile", get_tile)?;

        // set_tile(scene_id, entity_id, column, row, tile) -> whether the
        // cell changed; tile nil empties it. A tilemap with a body gets its
        // collider rebuilt.
        let manager = Rc::clone(scene_manager);
        let physics = Rc::clone(physics_engine);
        let set_tile = self.lua.create_function(
            move |_,
                  (scene_id, entity_id, column, row, tile): (
                String,
                String,
                i32,
                i32,
                Option<u32>,
            )| {
                let mut manager = manager.borrow_mut();
                let entity = tilemap_entity(&mut manager, "set_tile", &scene_id, &entity_id)?;
                let changed = entity
                    .tilemap
                    .as_mut()
                    .expect("checked by tilemap_entity")
                    .set(column, row, tile)
                    .map_err(|e| tile_error("set_tile", e))?;
                if changed {
                    physics
                        .borrow_mut()
                        .update_tilemap_cells(entity, &[(column, row)]);
                }
                Ok(changed)
            },
        )?;
        globals.set("set_tile", set_tile)?;

        // fill_tiles(scene_id, entity_id, column, row, columns, rows, tile)
        // -> how many cells changed: put `tile` (nil empties) in a
        // `columns` x `rows` rectangle from (column, row)
        let manager = Rc::clone(scene_manager);
        let physics = Rc::clone(physics_engine);
        let fill_tiles = self.lua.create_function(
            move |_,
                  (scene_id, entity_id, column, row, columns, rows, tile): (
                String,
                String,
                i32,
                i32,
                i32,
                i32,
                Option<u32>,
            )| {
                if columns < 0 || rows < 0 {
                    return Err(tile_error(
                        "fill_tiles",
                        format!("columns and rows can't be negative: {} x {}", columns, rows),
                    ));
                }
                if columns == 0 || rows == 0 {
                    return Ok(0);
                }
                let last = |first: i32, count: i32| {
                    first.checked_add(count - 1).ok_or_else(|| {
                        tile_error(
                            "fill_tiles",
                            "the rectangle goes past the last column or row",
                        )
                    })
                };
                let (last_column, last_row) = (last(column, columns)?, last(row, rows)?);
                let mut manager = manager.borrow_mut();
                let entity = tilemap_entity(&mut manager, "fill_tiles", &scene_id, &entity_id)?;
                let changed = entity
                    .tilemap
                    .as_mut()
                    .expect("checked by tilemap_entity")
                    .fill_rect((column, last_column), (row, last_row), tile)
                    .map_err(|e| tile_error("fill_tiles", e))?;
                physics.borrow_mut().update_tilemap_cells(entity, &changed);
                Ok(changed.len())
            },
        )?;
        globals.set("fill_tiles", fill_tiles)?;

        // flood_fill_tiles(scene_id, entity_id, column, row, tile) -> how
        // many cells changed: like a paint bucket, from the cell through its
        // neighbours holding the same, within the map's tiles' bounds
        let manager = Rc::clone(scene_manager);
        let physics = Rc::clone(physics_engine);
        let flood_fill_tiles = self.lua.create_function(
            move |_,
                  (scene_id, entity_id, column, row, tile): (
                String,
                String,
                i32,
                i32,
                Option<u32>,
            )| {
                let mut manager = manager.borrow_mut();
                let entity =
                    tilemap_entity(&mut manager, "flood_fill_tiles", &scene_id, &entity_id)?;
                let changed = entity
                    .tilemap
                    .as_mut()
                    .expect("checked by tilemap_entity")
                    .flood_fill(column, row, tile)
                    .map_err(|e| tile_error("flood_fill_tiles", e))?;
                physics.borrow_mut().update_tilemap_cells(entity, &changed);
                Ok(changed.len())
            },
        )?;
        globals.set("flood_fill_tiles", flood_fill_tiles)?;

        // get_tile_cell(scene_id, entity_id, x, y) -> column, row of the cell
        // containing the world position
        let manager = Rc::clone(scene_manager);
        let get_tile_cell = self.lua.create_function(
            move |_, (scene_id, entity_id, x, y): (String, String, f32, f32)| {
                let mut manager = manager.borrow_mut();
                let entity = tilemap_entity(&mut manager, "get_tile_cell", &scene_id, &entity_id)?;
                let origin = (entity.get_x(), entity.get_y());
                let map = entity.tilemap.as_ref().expect("checked by tilemap_entity");
                Ok(map.cell_at(origin, (x, y)))
            },
        )?;
        globals.set("get_tile_cell", get_tile_cell)?;

        Ok(())
    }
}
