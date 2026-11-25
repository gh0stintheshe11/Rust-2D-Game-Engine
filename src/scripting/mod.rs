//! Lua 5.4 scripting: per-entity scripts and the engine bindings they call.
//!
//! See `docs/scripting.md` for the script-facing API.

use std::cell::{Cell, Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use mlua::{Function as LuaFunction, HookTriggers, Lua, LuaOptions, StdLib, Table, VmState};
use uuid::Uuid;

use crate::audio::AudioEngine;
use crate::input::InputHandler;
use crate::logger::LOGGER;
use crate::physics::PhysicsEngine;
use crate::render::HudText;
use crate::save::SaveData;
use crate::scene::{Scene, SceneManager};

mod audio_bindings;
mod input_bindings;
mod physics_bindings;
mod render_bindings;
mod save_bindings;
mod scene_bindings;

/// Longest a single call into a script (a hook, or a file's top-level
/// code) may run before it is aborted with an error. Guards the editor
/// against infinite loops.
const SCRIPT_TIME_LIMIT: Duration = Duration::from_secs(1);

/// Upper bound on the memory a play session's scripts may allocate.
const SCRIPT_MEMORY_LIMIT: usize = 256 * 1024 * 1024;

/// How often (in VM instructions) the time limit is checked.
const TIME_CHECK_INTERVAL: u32 = 10_000;

/// Standard libraries available to scripts. Notably absent: `io`,
/// `package`/`require` and `debug`; `os` is reduced to its clock functions
/// by [`SANDBOX_PRELUDE`].
fn script_libs() -> StdLib {
    StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8 | StdLib::COROUTINE | StdLib::OS
}

/// Runs once in every new VM: restricts what scripts can reach.
const SANDBOX_PRELUDE: &str = r#"
local os_clock, os_time, os_date, os_difftime = os.clock, os.time, os.date, os.difftime
os = { clock = os_clock, time = os_time, date = os_date, difftime = os_difftime }
dofile = nil
loadfile = nil
local raw_load = load
load = function(chunk, name, mode, env)
    return raw_load(chunk, name, "t", env)
end
"#;

/// A compiled script file: its environment (holding the file's top-level
/// variables) and lifecycle hooks.
#[derive(Clone)]
struct CompiledScript {
    modified: Option<SystemTime>,
    env: Table,
    update_fn: Option<LuaFunction>,
    init_fn: Option<LuaFunction>,
    on_collision_fn: Option<LuaFunction>,
    on_collision_end_fn: Option<LuaFunction>,
}

/// What is known about one script file this session.
enum ScriptSlot {
    Ready(CompiledScript),
    /// The file at `modified` doesn't compile or run; the last version that
    /// did (if any) keeps running until the file is fixed.
    Broken {
        modified: Option<SystemTime>,
        last_good: Option<CompiledScript>,
    },
}

impl ScriptSlot {
    fn usable(&self) -> Option<&CompiledScript> {
        match self {
            ScriptSlot::Ready(script) => Some(script),
            ScriptSlot::Broken { last_good, .. } => last_good.as_ref(),
        }
    }

    fn modified(&self) -> Option<SystemTime> {
        match self {
            ScriptSlot::Ready(script) => script.modified,
            ScriptSlot::Broken { modified, .. } => *modified,
        }
    }
}

/// Whether an entity's `init` ran.
enum InitState {
    Done,
    /// `init` raised an error; the entity's hooks are skipped until its
    /// script file changes.
    Failed(PathBuf),
}

/// A scene's scripted entities, grouped by script file: `files` lists each
/// file once, and `entities` pairs every scripted entity with its file's
/// index, so per-file work happens once.
struct ScriptedEntities {
    files: Vec<PathBuf>,
    entities: Vec<(Uuid, usize)>,
}

impl ScriptedEntities {
    fn of(scene: &Scene) -> Self {
        let mut files: Vec<PathBuf> = Vec::new();
        let mut index: HashMap<&Path, usize> = HashMap::new();
        let mut entities = Vec::with_capacity(scene.entities.len());
        for (id, entity) in &scene.entities {
            let Some(path) = entity.script.as_deref() else {
                continue;
            };
            let file = *index.entry(path).or_insert_with(|| {
                files.push(path.to_path_buf());
                files.len() - 1
            });
            entities.push((*id, file));
        }
        Self { files, entities }
    }
}

/// Lua scripting engine.
///
/// - [`start_session`](Self::start_session) is called when the game starts
///   playing: it creates a fresh, sandboxed VM, sets up globals and registers
///   all engine bindings, which capture shared `Rc<RefCell<...>>` handles to
///   the runtime's subsystems.
/// - Every frame the runtime calls [`update_global_time`](Self::update_global_time),
///   [`bind_keys_pressed`](Self::bind_keys_pressed) and
///   [`run_scripts_for_scene`](Self::run_scripts_for_scene), and after each
///   physics step [`dispatch_collision_events`](Self::dispatch_collision_events).
///
/// Each script file runs in its own environment table whose `__index` falls
/// back to the VM globals, so scripts see all bindings while their own
/// top-level definitions don't collide. All entities using the same file
/// share its environment.
pub struct LuaScripting {
    lua: Lua,
    accumulated_time: f64,
    scripts: HashMap<PathBuf, ScriptSlot>,
    scene_manager: Option<Rc<RefCell<SceneManager>>>,
    physics_engine: Option<Rc<RefCell<PhysicsEngine>>>,
    init_states: HashMap<Uuid, InitState>,
    // Contact sets after the previous dispatch, for edge-triggered on_collision
    previous_contacts: HashMap<Uuid, HashSet<Uuid>>,
    // Set by the end_game() binding; polled by the runtime
    game_stop_requested: Rc<Cell<bool>>,
    // Set by the change_scene() binding; polled by the runtime
    scene_change_request: Rc<Cell<Option<Uuid>>>,
    // Text drawn by scripts this frame (draw_text), painted by the runtime
    hud: Rc<RefCell<Vec<HudText>>>,
    // What save_data / load_data read and write (see set_save_data)
    save_data: Rc<RefCell<SaveData>>,
    // Entity and scene ids as Lua strings, for hook arguments: creating
    // them for every call costs far more than the calls themselves
    id_strings: HashMap<Uuid, mlua::LuaString>,
    // When the running script call must be aborted (see SCRIPT_TIME_LIMIT)
    deadline: Rc<Cell<Option<Instant>>>,
}

pub(crate) fn parse_uuid(value: &str, what: &str) -> Result<Uuid, mlua::Error> {
    Uuid::parse_str(value)
        .map_err(|e| mlua::Error::external(format!("Invalid {} UUID '{}': {}", what, value, e)))
}

/// Parse a script without running it. Returns a one-line, human-readable
/// description of the first syntax error (`line N: ...`), or `None` if the
/// source parses.
pub fn check_syntax(source: &str) -> Option<String> {
    let lua = Lua::new();
    let result = lua.load(source).set_name("script").into_function();
    match result {
        Ok(_) => None,
        Err(e) => {
            let message = e.to_string();
            // mlua reports `[string "script"]:LINE: message`; make it friendlier
            let message = message.replace("[string \"script\"]:", "line ");
            // Keep only the first line of the (possibly multi-line) report
            Some(message.lines().next().unwrap_or(&message).to_string())
        }
    }
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
            scripts: HashMap::new(),
            scene_manager: None,
            physics_engine: None,
            init_states: HashMap::new(),
            previous_contacts: HashMap::new(),
            game_stop_requested: Rc::new(Cell::new(false)),
            scene_change_request: Rc::new(Cell::new(None)),
            hud: Rc::new(RefCell::new(Vec::new())),
            save_data: Rc::new(RefCell::new(SaveData::in_memory())),
            id_strings: HashMap::new(),
            deadline: Rc::new(Cell::new(None)),
        }
    }

    /// Text drawn with `draw_text` since the current frame's scripts
    /// started.
    pub fn hud_texts(&self) -> Ref<'_, Vec<HudText>> {
        self.hud.borrow()
    }

    /// Use `save_data` for the `save_data` / `load_data` bindings of the
    /// sessions started from now on (by default, save data in memory).
    pub fn set_save_data(&mut self, save_data: Rc<RefCell<SaveData>>) {
        self.save_data = save_data;
    }

    /// The session's Lua VM, e.g. to inspect globals in tests.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// Start a fresh scripting session: new sandboxed VM, fresh globals,
    /// all engine bindings registered, and a copy of the scenes as they are
    /// now, which `spawn` copies templates from. Called when the game starts
    /// playing.
    pub fn start_session(
        &mut self,
        scene_manager: Rc<RefCell<SceneManager>>,
        physics_engine: Rc<RefCell<PhysicsEngine>>,
        input_handler: Rc<RefCell<InputHandler>>,
        audio_engine: Rc<RefCell<AudioEngine>>,
    ) -> Result<(), mlua::Error> {
        self.lua = self.new_sandboxed_vm()?;
        self.id_strings.clear();
        self.scripts.clear();
        self.init_states.clear();
        self.previous_contacts.clear();
        self.hud.borrow_mut().clear();
        self.accumulated_time = 0.0;
        self.scene_manager = Some(Rc::clone(&scene_manager));
        self.physics_engine = Some(Rc::clone(&physics_engine));
        self.game_stop_requested.set(false);
        self.scene_change_request.set(None);

        let globals = self.lua.globals();
        globals.set("accumulated_time", 0.0)?;
        globals.set("delta_time", 0.0)?;

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
        self.register_scene_bindings(&scene_manager, &physics_engine)?;
        self.register_audio_bindings(&audio_engine)?;
        self.register_render_bindings(&scene_manager)?;
        let save_data = Rc::clone(&self.save_data);
        self.register_save_bindings(&save_data)?;

        LOGGER.info("Lua scripting session started");
        Ok(())
    }

    /// A VM with only the [`script_libs`], a memory limit, `print` routed to
    /// the editor console, and the time limit hook installed.
    fn new_sandboxed_vm(&self) -> Result<Lua, mlua::Error> {
        let lua = Lua::new_with(script_libs(), LuaOptions::default())?;
        lua.load(SANDBOX_PRELUDE).set_name("sandbox").exec()?;
        lua.set_memory_limit(SCRIPT_MEMORY_LIMIT)?;

        let print = lua.create_function(|_, args: mlua::Variadic<mlua::Value>| {
            let parts: Vec<String> = args
                .iter()
                .map(|value| {
                    value
                        .to_string()
                        .unwrap_or_else(|_| value.type_name().to_string())
                })
                .collect();
            LOGGER.info(parts.join("\t"));
            Ok(())
        })?;
        lua.globals().set("print", print)?;

        // A global hook (unlike a per-thread one) is inherited by coroutines
        // that scripts create, so they can't escape the time limit
        let deadline = Rc::clone(&self.deadline);
        lua.set_global_hook(
            HookTriggers::new().every_nth_instruction(TIME_CHECK_INTERVAL),
            move |_, _| match deadline.get() {
                Some(deadline) if Instant::now() > deadline => Err(mlua::Error::runtime(format!(
                    "script ran for more than {} s (infinite loop?)",
                    SCRIPT_TIME_LIMIT.as_secs()
                ))),
                _ => Ok(VmState::Continue),
            },
        )?;
        Ok(lua)
    }

    /// Returns true (once) if a script requested the game to stop since the
    /// last call. Clears the flag.
    pub fn take_game_stop_request(&self) -> bool {
        self.game_stop_requested.replace(false)
    }

    /// The scene a script asked to switch to with `change_scene` since the
    /// last call (the last request wins), if any. Clears the request.
    pub fn take_scene_change_request(&self) -> Option<Uuid> {
        self.scene_change_request.take()
    }

    /// Start over in a newly activated scene: forget the previous scene's
    /// per-entity state (which entities ran `init`, their contacts) and the
    /// text drawn this frame. The VM, its globals (`script_state`) and the
    /// loaded script files carry over. Called by the runtime when it
    /// switches scenes.
    pub fn begin_scene(&mut self) {
        self.init_states.clear();
        self.previous_contacts.clear();
        self.id_strings.clear();
        self.hud.borrow_mut().clear();
    }

    /// Advance the game clock: `accumulated_time` grows by `delta_time`.
    pub fn update_global_time(&mut self, delta_time: f32) -> Result<(), String> {
        self.accumulated_time += f64::from(delta_time);
        let globals = self.lua.globals();
        globals
            .set("accumulated_time", self.accumulated_time)
            .and_then(|_| globals.set("delta_time", f64::from(delta_time)))
            .map_err(|e| e.to_string())
    }

    /// Refresh the `keys_pressed` array global from the input handler.
    pub fn bind_keys_pressed(&self, input_handler: &InputHandler) -> Result<(), mlua::Error> {
        let keys_pressed_table = self
            .lua
            .create_sequence_from(input_handler.get_all_active_inputs())?;
        self.lua.globals().set("keys_pressed", keys_pressed_table)?;
        Ok(())
    }

    /// Run the `update` function of every scripted entity in the scene. This
    /// starts a frame: text drawn during the previous one is cleared first.
    ///
    /// The entity list is snapshotted first, so scripts can safely add or
    /// remove entities while running. A failing script is logged and skipped
    /// instead of aborting the others.
    pub fn run_scripts_for_scene(&mut self, active_scene_id: Uuid) -> Result<(), String> {
        // A new frame: scripts draw their text again (immediate mode)
        self.hud.borrow_mut().clear();

        let scene_manager = self
            .scene_manager
            .clone()
            .ok_or_else(|| "Lua session not started".to_string())?;

        // Snapshot the scripted entities without holding a borrow while
        // scripts run - scripts may mutate the scene through bindings.
        let ScriptedEntities { files, entities } = {
            let manager = scene_manager.borrow();
            let scene = manager
                .get_scene(active_scene_id)
                .ok_or_else(|| "Active scene not found.".to_string())?;
            // Forget per-entity state of entities that no longer exist
            let exists = |id: &Uuid| scene.entities.contains_key(id);
            self.init_states.retain(|id, _| exists(id));
            self.previous_contacts.retain(|id, _| exists(id));
            self.id_strings
                .retain(|id, _| *id == active_scene_id || exists(id));
            ScriptedEntities::of(scene)
        };

        // Each file is checked for changes once per frame
        let scripts: Vec<Option<CompiledScript>> = files
            .iter()
            .map(|path| {
                self.refresh_script(path);
                self.usable_script(path)
            })
            .collect();
        let scene_arg = self.lua_id(active_scene_id).map_err(|e| e.to_string())?;

        for (entity_id, file) in entities {
            let Some(script) = &scripts[file] else {
                continue;
            };
            // The entity may have been removed by a script earlier this frame
            if !entity_exists(&scene_manager, active_scene_id, entity_id) {
                continue;
            }
            if !self.ensure_initialized(entity_id, &files[file], script, active_scene_id) {
                continue;
            }
            if let Some(update_fn) = &script.update_fn {
                let entity_arg = self.lua_id(entity_id).map_err(|e| e.to_string())?;
                let result =
                    self.with_time_limit(|| update_fn.call::<()>((&scene_arg, &entity_arg)));
                if let Err(e) = result {
                    LOGGER.error(format!(
                        "Script update() error for entity {} ({}): {}",
                        entity_id,
                        files[file].display(),
                        e
                    ));
                }
            }
        }

        Ok(())
    }

    /// An entity or scene id as a Lua string, cached for the scene.
    fn lua_id(&mut self, id: Uuid) -> mlua::Result<mlua::LuaString> {
        if let Some(string) = self.id_strings.get(&id) {
            return Ok(string.clone());
        }
        let string = self.lua.create_string(id.to_string())?;
        self.id_strings.insert(id, string.clone());
        Ok(string)
    }

    /// Fire `on_collision(scene_id, entity_id, other_id)` for every contact a
    /// scripted entity gained since the last call, and
    /// `on_collision_end(scene_id, entity_id, other_id)` for every contact it
    /// lost. Called by the runtime after each physics step.
    pub fn dispatch_collision_events(&mut self, active_scene_id: Uuid) -> Result<(), String> {
        let scene_manager = self
            .scene_manager
            .clone()
            .ok_or_else(|| "Lua session not started".to_string())?;
        let physics = self
            .physics_engine
            .clone()
            .ok_or_else(|| "Lua session not started".to_string())?;

        // Scripts are (re)loaded by the update pass; use what's loaded. Only
        // entities whose script has a collision hook track their contacts.
        let ScriptedEntities { files, entities } = {
            let manager = scene_manager.borrow();
            let Some(scene) = manager.get_scene(active_scene_id) else {
                return Ok(());
            };
            ScriptedEntities::of(scene)
        };
        let scripts: Vec<Option<CompiledScript>> = files
            .iter()
            .map(|path| {
                self.usable_script(path).filter(|script| {
                    script.on_collision_fn.is_some() || script.on_collision_end_fn.is_some()
                })
            })
            .collect();
        // Their contacts, read up front
        let contacts: Vec<(Uuid, usize, HashSet<Uuid>)> = {
            let physics = physics.borrow();
            entities
                .into_iter()
                .filter(|(_, file)| scripts[*file].is_some())
                .map(|(id, file)| {
                    let current = physics.get_colliding_entities(id).into_iter().collect();
                    (id, file, current)
                })
                .collect()
        };

        for (entity_id, file, current) in contacts {
            let previous = self
                .previous_contacts
                .remove(&entity_id)
                .unwrap_or_default();
            let started: Vec<Uuid> = current.difference(&previous).copied().collect();
            let ended: Vec<Uuid> = previous.difference(&current).copied().collect();
            self.previous_contacts.insert(entity_id, current);
            if started.is_empty() && ended.is_empty() {
                continue;
            }
            let (Some(script), script_path) = (&scripts[file], &files[file]) else {
                continue;
            };
            // Contacts that ended first, so leaving one zone comes before
            // entering the next
            let hooks = [
                (
                    ended,
                    script.on_collision_end_fn.clone(),
                    "on_collision_end",
                ),
                (started, script.on_collision_fn.clone(), "on_collision"),
            ];
            for (others, hook, hook_name) in hooks {
                let Some(hook) = hook else {
                    continue;
                };
                for other_id in others {
                    // An earlier handler may have removed an entity. A
                    // contact that started needs both; one that ended is
                    // reported even when the other entity is gone.
                    let other_needed = hook_name == "on_collision";
                    if !entity_exists(&scene_manager, active_scene_id, entity_id)
                        || (other_needed
                            && !entity_exists(&scene_manager, active_scene_id, other_id))
                    {
                        continue;
                    }
                    // Hooks always see an initialized entity
                    if !self.ensure_initialized(entity_id, script_path, script, active_scene_id) {
                        break;
                    }
                    let args = self.lua_id(active_scene_id).and_then(|scene| {
                        let entity = self.lua_id(entity_id)?;
                        Ok((scene, entity, self.lua_id(other_id)?))
                    });
                    let result = args.and_then(|(scene, entity, other)| {
                        self.with_time_limit(|| hook.call::<()>((&scene, &entity, &other)))
                    });
                    if let Err(e) = result {
                        LOGGER.error(format!(
                            "Script {}() error for entity {} ({}): {}",
                            hook_name,
                            entity_id,
                            script_path.display(),
                            e
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Run `init` for the entity unless it already ran. Returns whether the
    /// entity's other hooks may run.
    fn ensure_initialized(
        &mut self,
        entity_id: Uuid,
        script_path: &Path,
        script: &CompiledScript,
        scene_id: Uuid,
    ) -> bool {
        match self.init_states.get(&entity_id) {
            Some(InitState::Done) => return true,
            Some(InitState::Failed(_)) => return false,
            None => {}
        }
        let succeeded = match &script.init_fn {
            Some(init_fn) => {
                let args = self
                    .lua_id(scene_id)
                    .and_then(|scene| self.lua_id(entity_id).map(|entity| (scene, entity)));
                let result = args.and_then(|(scene, entity)| {
                    self.with_time_limit(|| init_fn.call::<()>((&scene, &entity)))
                });
                match result {
                    Ok(()) => true,
                    Err(e) => {
                        LOGGER.error(format!(
                            "Script init() error for entity {} ({}): {}. \
                             The entity's script is paused until the file changes.",
                            entity_id,
                            script_path.display(),
                            e
                        ));
                        false
                    }
                }
            }
            None => true,
        };
        let state = if succeeded {
            InitState::Done
        } else {
            InitState::Failed(script_path.to_path_buf())
        };
        self.init_states.insert(entity_id, state);
        succeeded
    }

    /// The version of a script that should run, if any.
    fn usable_script(&self, path: &Path) -> Option<CompiledScript> {
        self.scripts.get(path)?.usable().cloned()
    }

    /// Compile the script if it is new or its file changed since it was last
    /// compiled. Errors are logged once per change of the file.
    fn refresh_script(&mut self, path: &Path) {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let previous = self.scripts.remove(path);
        if let Some(slot) = &previous
            && slot.modified() == modified
        {
            self.scripts.insert(path.to_path_buf(), previous.unwrap());
            return;
        }

        let last_good = previous.as_ref().and_then(|slot| slot.usable().cloned());
        let slot = match self.compile_script(path, modified, last_good.as_ref()) {
            Ok(script) => {
                if last_good.is_some() {
                    LOGGER.info(format!("Reloaded script {}", path.display()));
                }
                // Entities whose init failed get another chance
                self.init_states
                    .retain(|_, state| !matches!(state, InitState::Failed(p) if p == path));
                ScriptSlot::Ready(script)
            }
            Err(e) => {
                let fallback = if last_good.is_some() {
                    " The previous version keeps running."
                } else {
                    ""
                };
                LOGGER.error(format!("{}{}", e, fallback));
                ScriptSlot::Broken {
                    modified,
                    last_good,
                }
            }
        };
        self.scripts.insert(path.to_path_buf(), slot);
    }

    /// Read and run a script file in a fresh environment. On a reload, the
    /// previous version's variables (but not its functions) are carried
    /// over first, so state set up in `init` survives; the file's top-level
    /// code may still overwrite it.
    fn compile_script(
        &self,
        path: &Path,
        modified: Option<SystemTime>,
        previous: Option<&CompiledScript>,
    ) -> Result<CompiledScript, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Can't read script {}: {}", path.display(), e))?;

        let env = self
            .new_environment(previous)
            .map_err(|e| format!("Error creating script env: {}", e))?;
        self.with_time_limit(|| {
            self.lua
                .load(&source)
                .set_name(path.display().to_string())
                .set_mode(mlua::chunk::ChunkMode::Text)
                .set_environment(env.clone())
                .exec()
        })
        .map_err(|e| format!("Error in script {}: {}", path.display(), e))?;

        Ok(CompiledScript {
            modified,
            update_fn: env.get("update").ok(),
            init_fn: env.get("init").ok(),
            on_collision_fn: env.get("on_collision").ok(),
            on_collision_end_fn: env.get("on_collision_end").ok(),
            env,
        })
    }

    fn new_environment(&self, previous: Option<&CompiledScript>) -> Result<Table, mlua::Error> {
        let env = self.lua.create_table()?;
        let meta = self.lua.create_table()?;
        meta.set("__index", self.lua.globals())?;
        env.set_metatable(Some(meta))?;
        if let Some(previous) = previous {
            for pair in previous.env.pairs::<mlua::Value, mlua::Value>() {
                let (key, value) = pair?;
                if !value.is_function() {
                    env.raw_set(key, value)?;
                }
            }
        }
        Ok(env)
    }

    /// Run a call into Lua under [`SCRIPT_TIME_LIMIT`].
    fn with_time_limit<T>(&self, call: impl FnOnce() -> mlua::Result<T>) -> mlua::Result<T> {
        self.deadline.set(Some(Instant::now() + SCRIPT_TIME_LIMIT));
        let result = call();
        self.deadline.set(None);
        result
    }
}

fn entity_exists(scene_manager: &RefCell<SceneManager>, scene_id: Uuid, entity_id: Uuid) -> bool {
    scene_manager
        .borrow()
        .get_scene(scene_id)
        .is_some_and(|scene| scene.entities.contains_key(&entity_id))
}
