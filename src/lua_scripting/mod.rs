use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use mlua::{Function as LuaFunction, Lua};
use uuid::Uuid;

use crate::ecs::SceneManager;
use crate::input_handler::InputHandler;
use crate::logger::LOGGER;
use crate::physics_engine::PhysicsEngine;

mod ecs_bindings;
mod input_bindings;
mod physics_bindings;

/// A compiled script's `update` function, cached for the duration of a play
/// session. `modified` allows hot-reloading when the file changes on disk.
struct CachedScript {
    modified: Option<SystemTime>,
    update_fn: LuaFunction,
}

/// Lua scripting engine.
///
/// Lifecycle:
/// - `start_session` is called once when the game starts playing: it creates a
///   fresh VM, sets up globals (`accumulated_time`, `script_state`,
///   `keys_pressed`) and registers all engine bindings, which capture shared
///   `Rc<RefCell<...>>` handles to the runtime's subsystems.
/// - Every frame the runtime calls `update_global_time`, `bind_keys_pressed`
///   and `run_scripts_for_scene`. Scripts are compiled once (per session, or
///   when their file changes) and their `update(scene_id, entity_id)` is
///   called directly.
///
/// Script environment:
/// - Each script file runs in its own environment table whose `__index` falls
///   back to the VM globals. Scripts therefore see all engine bindings and
///   shared globals, but their own top-level definitions (like `update`)
///   don't collide with other scripts.
/// - `script_state` is a plain Lua table (`script_state.state`) shared by all
///   scripts and persistent for the whole session.
pub struct LuaScripting {
    pub lua: Lua,
    accumulated_time: f32,
    script_cache: HashMap<PathBuf, CachedScript>,
    scene_manager: Option<Rc<RefCell<SceneManager>>>,
    // Set by the end_game() binding; polled by the runtime each frame
    game_stop_requested: Rc<Cell<bool>>,
}

pub(crate) fn parse_uuid(value: &str, what: &str) -> Result<Uuid, mlua::Error> {
    Uuid::parse_str(value)
        .map_err(|e| mlua::Error::external(format!("Invalid {} UUID '{}': {}", what, value, e)))
}

impl Default for LuaScripting {
    fn default() -> Self {
        Self::new()
    }
}

impl LuaScripting {
    pub fn new() -> Self {
        LuaScripting {
            lua: Lua::new(),
            accumulated_time: 0.0,
            script_cache: HashMap::new(),
            scene_manager: None,
            game_stop_requested: Rc::new(Cell::new(false)),
        }
    }

    /// Start a fresh scripting session: new VM, fresh globals, all engine
    /// bindings registered. Called once when the game starts playing.
    pub fn start_session(
        &mut self,
        scene_manager: Rc<RefCell<SceneManager>>,
        physics_engine: Rc<RefCell<PhysicsEngine>>,
        input_handler: Rc<RefCell<InputHandler>>,
    ) -> Result<(), mlua::Error> {
        self.lua = Lua::new();
        self.script_cache.clear();
        self.accumulated_time = 0.0;
        self.scene_manager = Some(Rc::clone(&scene_manager));
        self.game_stop_requested.set(false);

        let globals = self.lua.globals();
        globals.set("accumulated_time", 0.0)?;

        // script_state.state is the persistent, shared store for script data
        let script_state = self.lua.create_table()?;
        script_state.set("state", self.lua.create_table()?)?;
        globals.set("script_state", script_state)?;

        globals.set("keys_pressed", self.lua.create_table()?)?;

        // end_game(): scripts call this to stop the running game (e.g. on
        // player death). The runtime polls the flag after scripts run.
        let stop_flag = Rc::clone(&self.game_stop_requested);
        let end_game = self.lua.create_function(move |_, ()| {
            stop_flag.set(true);
            Ok(())
        })?;
        globals.set("end_game", end_game)?;

        self.register_physics_bindings(&physics_engine, &scene_manager)?;
        self.register_input_bindings(&input_handler)?;
        self.register_ecs_bindings(&scene_manager)?;

        LOGGER.info("Lua scripting session started");
        Ok(())
    }

    /// Returns true (once) if a script requested the game to stop since the
    /// last call. Clears the flag.
    pub fn take_game_stop_request(&self) -> bool {
        self.game_stop_requested.replace(false)
    }

    /// Increment the shared game clock and expose it to scripts.
    pub fn update_global_time(&mut self, delta_time: f32) -> Result<(), String> {
        self.accumulated_time += delta_time;
        self.lua
            .globals()
            .set("accumulated_time", self.accumulated_time)
            .map_err(|e| e.to_string())
    }

    /// Refresh the `keys_pressed` array global from the input handler.
    pub fn bind_keys_pressed(&self, input_handler: &InputHandler) -> Result<(), mlua::Error> {
        let keys_pressed_table = self.lua.create_table()?;
        for (index, key) in input_handler.get_all_active_inputs().iter().enumerate() {
            keys_pressed_table.set(index + 1, key.to_string())?;
        }
        self.lua.globals().set("keys_pressed", keys_pressed_table)?;
        Ok(())
    }

    /// Run the `update` function of every scripted entity in the scene.
    ///
    /// The entity list is snapshotted first, so scripts can safely add or
    /// remove entities while running. A failing script is logged and skipped
    /// instead of aborting the others.
    pub fn run_scripts_for_scene(&mut self, active_scene_id: Uuid) -> Result<(), String> {
        let scene_manager = self
            .scene_manager
            .clone()
            .ok_or_else(|| "Lua session not started".to_string())?;

        // Snapshot (entity, script) pairs without holding a borrow while
        // scripts run - scripts may mutate the scene through bindings.
        let scripts: Vec<(Uuid, PathBuf)> = {
            let manager = scene_manager.borrow();
            let scene = manager
                .get_scene(active_scene_id)
                .ok_or_else(|| "Active scene not found.".to_string())?;
            scene
                .entities
                .iter()
                .filter_map(|(id, entity)| entity.script.clone().map(|path| (*id, path)))
                .collect()
        };

        for (entity_id, script_path) in scripts {
            // The entity may have been removed by a script earlier this frame
            {
                let manager = scene_manager.borrow();
                match manager.get_scene(active_scene_id) {
                    Some(scene) => {
                        if !scene.entities.contains_key(&entity_id) {
                            continue;
                        }
                    }
                    None => break,
                }
            }

            let update_fn = match self.get_or_load_script(&script_path) {
                Ok(f) => f,
                Err(e) => {
                    LOGGER.error(format!("Script load error for entity {}: {}", entity_id, e));
                    continue;
                }
            };

            if let Err(e) =
                update_fn.call::<()>((active_scene_id.to_string(), entity_id.to_string()))
            {
                LOGGER.error(format!(
                    "Script update() error for entity {} ({}): {}",
                    entity_id,
                    script_path.display(),
                    e
                ));
            }
        }

        Ok(())
    }

    /// Compile a script (once per session, re-compiled if the file changed)
    /// and return its cached `update` function.
    fn get_or_load_script(&mut self, path: &Path) -> Result<LuaFunction, String> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();

        if let Some(cached) = self.script_cache.get(path) {
            if cached.modified == modified {
                return Ok(cached.update_fn.clone());
            }
        }

        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Error reading script file {}: {}", path.display(), e))?;

        // Each script gets its own environment with global fallback, so
        // top-level definitions (like `update`) don't collide across scripts.
        let build_env = || -> Result<mlua::Table, mlua::Error> {
            let env = self.lua.create_table()?;
            let meta = self.lua.create_table()?;
            meta.set("__index", self.lua.globals())?;
            env.set_metatable(Some(meta))?;
            Ok(env)
        };
        let env = build_env().map_err(|e| format!("Error creating script env: {}", e))?;

        self.lua
            .load(&source)
            .set_name(path.display().to_string())
            .set_environment(env.clone())
            .exec()
            .map_err(|e| format!("Error executing script {}: {}", path.display(), e))?;

        let update_fn: LuaFunction = env.get("update").map_err(|_| {
            format!(
                "Script {} does not define an update(scene_id, entity_id) function",
                path.display()
            )
        })?;

        self.script_cache.insert(
            path.to_path_buf(),
            CachedScript {
                modified,
                update_fn: update_fn.clone(),
            },
        );
        Ok(update_fn)
    }
}
