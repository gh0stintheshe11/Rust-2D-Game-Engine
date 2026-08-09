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

## Script lifecycle hooks

All optional except `update`:

| Hook | When |
|---|---|
| `init(scene_id, entity_id)` | Once per entity, before its first `update` (spawned entities get it on their first frame) |
| `update(scene_id, entity_id)` | Every rendered frame |
| `on_collision(scene_id, entity_id, other_id)` | When a contact with another physics entity **begins** (edge-triggered — once per new contact, dispatched after the physics step) |

## Globals available to scripts

| Global | Access | Meaning |
|---|---|---|
| `accumulated_time` | read | Real seconds since the session started (measured wall-clock delta per frame) |
| `script_state` | read/write | Persistent shared table; convention: `script_state.state.<your_key>`. Survives across frames within a session; reset on each new Play |
| `keys_pressed` | read | Array of active input names this frame |

## Engine bindings (callable functions)

Physics:

| Function | Notes |
|---|---|
| `set_velocity(entity_id, x, y)` | Entity must be registered in the physics engine |
| `apply_force(entity_id, x, y)` | Forces are reset after every physics step |
| `apply_impulse(entity_id, x, y)` | |
| `add_entity_to_physics_engine(entity_id)` | Reads the entity from the active scene; re-adding replaces the body (useful after changing physics attributes) |
| `remove_entity_from_physics_engine(entity_id)` | |
| `get_colliding_entities(entity_id) -> array of entity ids` | Entities currently in contact; empty table if the entity isn't in the physics engine |
| `set_gravity(x, y)` | Change the global gravity vector (screen space: +y is down; default `(0, 50)`) |

Game flow:

| Function | Notes |
|---|---|
| `end_game()` | Request a game-over: the runtime freezes on the current frame in an `Ended` state; only Reset exits it |

Input:

| Function | Notes |
|---|---|
| `is_key_just_pressed(key_name)` | True only on the frame the key went down. `key_name` uses egui `Key::from_name` names (e.g. `"Space"`, `"A"`) |
| `is_key_pressed(key_name)` | True while held |
| `is_mouse_pressed(button)` | `"left"`, `"right"` or `"middle"` |
| `get_mouse_position() -> {x, y}` | Window coordinates |
| `get_scroll_delta() -> {x, y}` | Zero when not scrolling |

Audio:

| Function | Notes |
|---|---|
| `play_sound(relative_path) -> play_id or nil` | Path resolves against the project root; returns nil (never errors) when the file is missing or the machine has no audio device |
| `stop_sound(play_id)` | |
| `is_sound_playing(play_id) -> bool` | |
| `stop_all_sounds()` | |

ECS (all IDs are UUID strings):

| Function | Notes |
|---|---|
| `add_entity(scene_id, name) -> entity_id` | |
| `remove_entity(scene_id, entity_id) -> bool` | |
| `create_physical_entity(scene_id, name, x, y, z) -> entity_id` | Seeds attributes from the predefined "Physics" archetype and spawns at the given position |
| `set_x` / `set_y` / `set_z(scene_id, entity_id, value)` | |
| `set_position(scene_id, entity_id, x, y)` | Sets x and y; leaves z untouched |
| `add_image(entity_id, relative_path)` | Path is joined onto the open project's root |
| `set_script(entity_id, relative_path)` | Ditto |
| `update_entity_attribute_bool(scene_id, entity_id, attr_name, value)` | |
| `create_attribute_float` / `_bool(scene_id, entity_id, name, value)` | |
| `create_attribute_vector2(scene_id, entity_id, name, x, y)` | |
| `get_attribute(scene_id, entity_id, name)` | Returns number / boolean / string / `{x, y}` table, or nil if missing. Works for built-ins (`x`, `y`, ...) and designer-defined attributes alike |
| `set_attribute(scene_id, entity_id, name, value)` | Coerces the Lua value to the attribute's declared type; errors on mismatch or missing attribute |
| `has_attribute(scene_id, entity_id, name) -> bool` | |
| `list_entities_name_x_y(scene_id) -> array of {id, name, x, y}` | x/y reflect the physics-synced position |
| `get_entity_name(scene_id, entity_id) -> string or nil` | nil when the entity no longer exists |

## Example

Designer-defined attributes are the intended way to expose tuning knobs to
scripts: add an attribute in the Inspector, read it with `get_attribute`.

```lua
function update(scene_id, entity_id)
    -- `jump_velocity` is a Float attribute on this entity, editable in the
    -- Inspector - no code changes needed to retune the game
    if is_key_just_pressed("Space") then
        local jump = get_attribute(scene_id, entity_id, "jump_velocity") or -260.0
        set_velocity(entity_id, 0.0, jump)
    end
end
```

## Known limitations / TODO

- No `on_destroy` hook yet; `on_collision` reports contact *begin* only (no end event).
- `get_mouse_position` is in window coordinates — no viewport/world mapping yet.
- Delta time is the real measured frame time (clamped to 0.25s); physics
  advances on a fixed timestep independently of the display refresh rate.
- `script_state` is shared by all scripts; key collisions are the script author's problem.
