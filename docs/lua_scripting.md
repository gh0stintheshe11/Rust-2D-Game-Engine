# Lua Scripting (`src/lua_scripting/`)

Game logic is written in Lua files attached to entities (`entity.script`). The engine runs each script's `update(scene_id, entity_id)` every frame while playing.

## Session model

A **session** spans one Play (from pressing ▶ until Stop/Reset):

- `start_session(scene_manager, physics_engine, input_handler)` — called by `GameRuntime::run()`. Creates a fresh `Lua` VM, sets up globals, and registers all engine bindings. Bindings capture `Rc<RefCell<...>>` handles to the runtime's subsystems (no `unsafe`, no raw pointers).
- Per frame, `GameRuntime::update()` calls:
  - `update_global_time(dt)` — advances the `accumulated_time` global
  - `bind_keys_pressed(&input)` — refreshes the `keys_pressed` array global
  - `run_scripts_for_scene(scene_id)` — runs every scripted entity's `update`

## Script compilation & environments

- Scripts are compiled **once per session** and cached. The cache key is the script path; the file's mtime is checked each frame, so **editing a script while the game is playing hot-reloads it**.
- Each script file executes in its **own environment table** whose `__index` falls back to the VM globals. Scripts see all engine bindings and shared globals, but their top-level definitions (like `update`) don't collide with other scripts.
- Before running scripts each frame, the entity list is **snapshotted**, so scripts can safely `add_entity` / `remove_entity` mid-frame.
- A failing script is logged to the editor console and skipped; other scripts still run.

## Globals available to scripts

| Global | Access | Meaning |
|---|---|---|
| `accumulated_time` | read | Seconds since the session started (increments by `1/target_fps` per frame) |
| `script_state` | read/write | Persistent shared table; convention: `script_state.state.<your_key>`. Survives across frames within a session; reset on each new Play |
| `keys_pressed` | read | Array of active input names this frame |

## Engine bindings (callable functions)

Physics:

| Function | Notes |
|---|---|
| `set_velocity(entity_id, x, y)` | Entity must be registered in the physics engine |
| `apply_force(entity_id, x, y)` | Forces are reset after every physics step |
| `apply_impulse(entity_id, x, y)` | |
| `add_entity_to_physics_engine(entity_id)` | Reads the entity from the active scene; re-adding replaces the body |
| `remove_entity_from_physics_engine(entity_id)` | |

Input:

| Function | Notes |
|---|---|
| `is_key_just_pressed(key_name)` | `key_name` uses egui `Key::from_name` names (e.g. `"Space"`, `"A"`) |

ECS (all IDs are UUID strings):

| Function | Notes |
|---|---|
| `add_entity(scene_id, name) -> entity_id` | |
| `remove_entity(scene_id, entity_id) -> bool` | |
| `create_physical_entity(scene_id, name, x, y, z) -> entity_id` | Seeds attributes from the predefined "Physics" archetype; position args currently unused |
| `set_x` / `set_y` / `set_z(scene_id, entity_id, value)` | |
| `set_position(scene_id, entity_id, x, y)` | Sets x and y; leaves z untouched |
| `add_image(entity_id, relative_path)` | Path is joined onto the open project's root |
| `set_script(entity_id, relative_path)` | Ditto |
| `update_entity_attribute_bool(scene_id, entity_id, attr_name, value)` | |
| `create_attribute_float` / `_bool(scene_id, entity_id, name, value)` | |
| `create_attribute_vector2(scene_id, entity_id, name, x, y)` | |
| `list_entities_name_x_y(scene_id) -> array of {id, name, x, y}` | x/y reflect the physics-synced position |

## Example

```lua
function update(scene_id, entity_id)
    if script_state.state.bird == nil then
        script_state.state.bird = { jumps = 0 }
    end

    if is_key_just_pressed("Space") then
        script_state.state.bird.jumps = script_state.state.bird.jumps + 1
        set_velocity(entity_id, 0.0, -100.0) -- +Y is down; negative y jumps up
    end
end
```

## Known limitations / TODO

- Only an `update` hook exists — no `init`, no collision callbacks, no `on_destroy`.
- No audio bindings (scripts can't play sounds yet) and only one input binding.
- Delta time is a fixed `1/target_fps` — not measured wall-clock time.
- `script_state` is shared by all scripts; key collisions are the script author's problem.
- `create_physical_entity` ignores its x/y/z arguments (the demo works around this by setting attributes from Lua).
