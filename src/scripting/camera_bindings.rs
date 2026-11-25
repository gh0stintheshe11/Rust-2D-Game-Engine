//! The camera and the screen: `camera_follow`, `camera_shake`,
//! `get_camera`, and the transitions of `change_scene`.

use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Table, Value};
use uuid::Uuid;

use super::{LuaScripting, parse_uuid};
use crate::render::{CameraFollow, TRANSITIONS, Transition, TransitionKind};
use crate::scene::SceneManager;

/// Seconds a transition lasts when `change_scene`'s options don't say.
pub const DEFAULT_TRANSITION_SECONDS: f32 = 0.5;

fn error(function: &str, message: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::external(format!("{}: {}", function, message))
}

/// A finite number from an options table.
fn number(function: &str, key: &str, value: Value) -> mlua::Result<f32> {
    let number = match value {
        Value::Number(number) => number as f32,
        Value::Integer(number) => number as f32,
        other => {
            return Err(error(
                function,
                format!("{} must be a number, got a {}", key, other.type_name()),
            ));
        }
    };
    if number.is_finite() {
        Ok(number)
    } else {
        Err(error(function, format!("{} must be finite", key)))
    }
}

fn at_least_zero(function: &str, key: &str, value: Value) -> mlua::Result<f32> {
    let number = number(function, key, value)?;
    if number >= 0.0 {
        Ok(number)
    } else {
        Err(error(
            function,
            format!("{} must be at least 0, not {}", key, number),
        ))
    }
}

/// The pairs of an options table, with string keys.
fn options(function: &str, table: Table) -> mlua::Result<Vec<(String, Value)>> {
    let mut pairs = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::String(key) = key else {
            return Err(error(
                function,
                format!("option names must be strings, got a {}", key.type_name()),
            ));
        };
        pairs.push((key.to_str()?.to_string(), value));
    }
    Ok(pairs)
}

/// `camera_follow`'s options, and whether to snap. Read before anything is
/// borrowed: reading a table can run Lua code.
fn follow_options(table: Option<Table>) -> mlua::Result<(CameraFollow, bool)> {
    const F: &str = "camera_follow";
    let mut follow = CameraFollow::default();
    let mut snap = false;
    let Some(table) = table else {
        return Ok((follow, snap));
    };
    for (key, value) in options(F, table)? {
        match key.as_str() {
            "offset_x" => follow.offset.0 = number(F, &key, value)?,
            "offset_y" => follow.offset.1 = number(F, &key, value)?,
            "smoothing" => follow.smoothing = at_least_zero(F, &key, value)?,
            "deadzone_width" => follow.deadzone.0 = at_least_zero(F, &key, value)?,
            "deadzone_height" => follow.deadzone.1 = at_least_zero(F, &key, value)?,
            "snap" => match value {
                Value::Boolean(value) => snap = value,
                other => {
                    return Err(error(
                        F,
                        format!("snap must be true or false, got a {}", other.type_name()),
                    ));
                }
            },
            "bounds" => {
                let Value::Table(bounds) = value else {
                    return Err(error(
                        F,
                        "bounds must be a table: {x = ..., y = ..., width = ..., height = ...}",
                    ));
                };
                let mut rect = [None; 4];
                for (key, value) in options(F, bounds)? {
                    let index = ["x", "y", "width", "height"]
                        .iter()
                        .position(|name| *name == key)
                        .ok_or_else(|| {
                            error(
                                F,
                                format!(
                                    "unknown bounds key '{}' (expected x, y, width, height)",
                                    key
                                ),
                            )
                        })?;
                    let name = format!("bounds.{}", key);
                    rect[index] = Some(if index < 2 {
                        number(F, &name, value)?
                    } else {
                        at_least_zero(F, &name, value)?
                    });
                }
                let [Some(x), Some(y), Some(width), Some(height)] = rect else {
                    return Err(error(F, "bounds need x, y, width and height"));
                };
                follow.bounds = Some((x, y, width, height));
            }
            unknown => {
                return Err(error(
                    F,
                    format!(
                        "unknown option '{}' (expected offset_x, offset_y, smoothing, \
                         deadzone_width, deadzone_height, bounds, snap)",
                        unknown
                    ),
                ));
            }
        }
    }
    Ok((follow, snap))
}

/// `change_scene`'s options: how the screen goes to the new scene. `None`
/// without options, or with `seconds = 0`: a cut.
pub(super) fn transition_options(table: Option<Table>) -> mlua::Result<Option<Transition>> {
    const F: &str = "change_scene";
    let Some(table) = table else {
        return Ok(None);
    };
    let mut transition = Transition::fade(DEFAULT_TRANSITION_SECONDS);
    for (key, value) in options(F, table)? {
        match (key.as_str(), value) {
            ("transition", Value::String(name)) => {
                let name = name.to_str()?.to_string();
                transition.kind = TransitionKind::parse(&name).ok_or_else(|| {
                    let names: Vec<&str> = TRANSITIONS.iter().map(|(_, name)| *name).collect();
                    error(
                        F,
                        format!(
                            "unknown transition '{}' (expected {})",
                            name,
                            names.join(", ")
                        ),
                    )
                })?;
            }
            ("seconds", value) => transition.seconds = at_least_zero(F, "seconds", value)?,
            ("color", Value::String(color)) => {
                transition.color =
                    crate::render::parse_color(&color.to_str()?).map_err(|e| error(F, e))?;
            }
            (key, _) => {
                return Err(error(
                    F,
                    format!(
                        "unknown or invalid option '{}' (expected transition, seconds, color)",
                        key
                    ),
                ));
            }
        }
    }
    Ok((transition.seconds > 0.0).then_some(transition))
}

impl LuaScripting {
    /// Move the scene's camera as `camera_follow` and `camera_shake` say,
    /// for this frame. Does nothing while neither is in use, so the camera
    /// is the scripts' to move.
    pub(super) fn move_camera(&mut self, scene_id: Uuid) {
        let mut rig = self.camera_rig.borrow_mut();
        if !rig.is_active() {
            return;
        }
        let (Some(manager), Some(physics)) = (&self.scene_manager, &self.physics_engine) else {
            return;
        };
        let mut manager = manager.borrow_mut();
        let Some(scene) = manager.get_scene_mut(scene_id) else {
            return;
        };
        let (Some(camera_id), Some(frame)) =
            (scene.default_camera, crate::render::camera_frame(scene))
        else {
            return;
        };
        // An entity with a body is followed by its middle; one without, by
        // its x / y
        let target = rig.following().and_then(|target| {
            let entity = scene.get_entity(target).ok()?;
            Some(
                physics
                    .borrow()
                    .body_center(target)
                    .unwrap_or((entity.get_x(), entity.get_y())),
            )
        });
        let Ok(camera) = scene.get_entity_mut(camera_id) else {
            return;
        };
        let now = (camera.get_x(), camera.get_y());
        let (x, y) = rig.step(self.frame_dt, now, target, frame.world_size);
        if (x, y) != now && camera.set_x(x).is_ok() && camera.set_y(y).is_ok() {
            physics.borrow_mut().set_position(camera_id, x, y);
        }
    }

    pub(crate) fn register_camera_bindings(
        &mut self,
        scene_manager: &Rc<RefCell<SceneManager>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // camera_follow(entity_id [, options]): keep the playing scene's
        // camera on an entity, from this frame on, until the scene changes,
        // the entity is removed or camera_follow(nil). Options: offset_x,
        // offset_y, smoothing (seconds behind), deadzone_width,
        // deadzone_height, bounds = {x, y, width, height}, snap
        let rig = Rc::clone(&self.camera_rig);
        let manager = Rc::clone(scene_manager);
        let camera_follow = self.lua.create_function(
            move |_, (entity, table): (Option<String>, Option<Table>)| {
                let Some(entity) = entity else {
                    rig.borrow_mut().stop_following();
                    return Ok(());
                };
                let target = parse_uuid(&entity, "entity")?;
                let (follow, snap) = follow_options(table)?;
                let in_scene = manager
                    .borrow()
                    .get_active_scene()
                    .is_some_and(|scene| scene.get_entity(target).is_ok());
                if !in_scene {
                    return Err(error(
                        "camera_follow",
                        format!("entity {} isn't in the playing scene", target),
                    ));
                }
                rig.borrow_mut().follow(target, follow, snap);
                Ok(())
            },
        )?;
        globals.set("camera_follow", camera_follow)?;

        // camera_shake(strength, seconds): shake the camera by up to
        // `strength` world units, fading out over `seconds`
        let rig = Rc::clone(&self.camera_rig);
        let camera_shake =
            self.lua
                .create_function(move |_, (strength, seconds): (Value, Value)| {
                    let strength = at_least_zero("camera_shake", "strength", strength)?;
                    let seconds = at_least_zero("camera_shake", "seconds", seconds)?;
                    if seconds == 0.0 {
                        return Err(error("camera_shake", "seconds must be above 0"));
                    }
                    rig.borrow_mut().shake(strength, seconds);
                    Ok(())
                })?;
        globals.set("camera_shake", camera_shake)?;

        // get_camera(scene_id) -> the scene's camera entity id, or nil
        let manager = Rc::clone(scene_manager);
        let get_camera = self.lua.create_function(move |_, scene_id: String| {
            let scene_id = parse_uuid(&scene_id, "scene")?;
            Ok(manager
                .borrow()
                .get_scene(scene_id)
                .and_then(|scene| scene.default_camera)
                .map(|id| id.to_string()))
        })?;
        globals.set("get_camera", get_camera)?;
        Ok(())
    }
}
