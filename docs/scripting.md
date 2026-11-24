# Lua Scripting (`src/scripting/`)

Game logic is written in Lua 5.4 files attached to entities (`entity.script`).
While the game plays, the engine calls each script's hooks for its entity.

## Sessions

A **session** spans one Play (from ▶ until Stop/Reset/restart).
`GameRuntime::run` calls `start_session(scene_manager, physics, input, audio)`,
which creates a fresh sandboxed VM, resets the globals, and registers all
engine bindings. Bindings hold `Rc<RefCell<…>>` handles to the runtime's
subsystems (no `unsafe`, no raw pointers), and never call back into Lua while
holding a borrow.

Every rendered frame the runtime calls `update_global_time(dt)`,
`bind_keys_pressed(&input)` and `run_scripts_for_scene(scene_id)`; after
every physics step it calls `dispatch_collision_events(scene_id)`.

## Hooks

| Hook | When |
|---|---|
| `init(scene_id, entity_id)` | Once per entity, before any other hook of that entity (spawned entities get it on their first frame) |
| `update(scene_id, entity_id)` | Every rendered frame |
| `on_collision(scene_id, entity_id, other_id)` | When a contact with another physics entity **begins** (checked after every physics step; skipped if either entity was removed meanwhile) |

All hooks are optional. If `init` raises an error, the entity's other hooks are
skipped until its script file changes.

## Script files and hot reload

- Each script file runs in its **own environment table** whose `__index`
  falls back to the VM globals: scripts see every binding, while their
  top-level definitions (like `update`) don't collide with other scripts.
  **All entities using the same file share its environment**, so per-entity
  state belongs in attributes or in `script_state.state[entity_id]`.
- Files are compiled once per session. Each frame the engine checks whether a
  file changed on disk; if so it is **reloaded while the game keeps
  running**: the previous version's variables are carried over (functions are
  replaced), then the file's top-level code runs again, and `init` doesn't run
  again.
- A file that fails to compile or run is reported once per change. If an
  earlier version worked, it keeps running until the file is fixed.
- The entity list is snapshotted before scripts run, so scripts can add and
  remove entities mid-frame.

## Sandbox and limits

- Available libraries: `string`, `table`, `math`, `utf8`, `coroutine`, and
  `os.clock` / `os.time` / `os.date` / `os.difftime`. There is no `io`,
  `package`/`require`, `debug`, `dofile` or `loadfile`; `load` accepts text
  only.
- `print(...)` writes to the editor console.
- A single hook call (or a file's top-level code) is aborted after **1 second**
  (catches infinite loops, also inside coroutines), and a session's scripts may
  allocate at most **256 MiB**.

The sandbox keeps mistakes from damaging the editor or the machine; it is not
a guarantee against deliberately malicious code. Only open projects you trust.

## Globals

| Global | Meaning |
|---|---|
| `accumulated_time` | Game seconds since the session started (paused time and the Editor tab excluded) |
| `delta_time` | Seconds since the previous frame |
| `script_state` | Persistent table shared by all scripts; convention: `script_state.state.<key>`. Reset on each Play |
| `keys_pressed` | Sorted array of everything held this frame: `"Ctrl"`, `"Shift"`, `"Alt"`, `"Cmd"` (macOS), key names (`"Space"`, `"A"`, `"ArrowLeft"`, `"ShiftLeft"`, …), `"MouseLeft"` / `"MouseRight"` / `"MouseMiddle"` |

## Bindings

All ids are UUID strings. Paths are relative to the project root.

Physics:

| Function | Notes |
|---|---|
| `set_velocity(entity_id, x, y)` | Entity must have a physics body |
| `apply_force(entity_id, x, y)` | Pushes for the current frame; call it every frame for a continuous force (independent of the frame rate) |
| `apply_impulse(entity_id, x, y)` | Instant change of momentum |
| `set_angular_velocity(entity_id, radians_per_second)` | Positive spins clockwise on screen; the body needs `can_rotate` |
| `add_entity_to_physics_engine(entity_id)` | Builds the body from the entity's attributes (active scene); re-adding replaces it, e.g. after changing physics attributes |
| `remove_entity_from_physics_engine(entity_id)` | |
| `get_colliding_entities(entity_id)` | Array of entity ids currently touching; empty if the entity has no body |
| `set_gravity(x, y)` | Global gravity (+y is down; default `(0, 50)`, restored on every Play) |

Input:

| Function | Notes |
|---|---|
| `is_key_just_pressed(key)` | True only on the frame the key went down. Key names are egui's (`"Space"`, `"A"`, `"ArrowUp"`), case-insensitive; unknown names raise an error |
| `is_key_pressed(key)` | True while held |
| `is_mouse_pressed(button)` | `"left"`, `"right"` or `"middle"` (also `"MouseLeft"` etc.) |
| `get_mouse_position()` | `{x, y}` in window coordinates |
| `get_scroll_delta()` | `{x, y}`; zero when not scrolling |

Audio:

| Function | Notes |
|---|---|
| `play_sound(path)` | Returns a play id, or nil when nothing can be played (missing file, no audio device; reported once per file) |
| `stop_sound(play_id)` | Accepts nil |
| `is_sound_playing(play_id)` | False for nil |
| `stop_all_sounds()` | |

Scenes and entities:

| Function | Notes |
|---|---|
| `add_entity(scene_id, name)` | Returns the new entity's id |
| `remove_entity(scene_id, entity_id)` | Also removes the entity's physics body; returns whether it existed |
| `create_physical_entity(scene_id, name, x, y, z)` | Entity with the Physics template attributes at the given position (no body until `add_entity_to_physics_engine`) |
| `set_x` / `set_y` / `set_z(scene_id, entity_id, value)` | Entities with a physics body are teleported (velocity is kept) |
| `set_position(scene_id, entity_id, x, y)` | Sets x and y, teleporting the body if any; z untouched |
| `add_image(entity_id, path)` | Active scene |
| `set_script(entity_id, path)` | Active scene; replaces the current script |
| `list_entities_name_x_y(scene_id)` | Array of `{id, name, x, y}` |
| `get_entity_name(scene_id, entity_id)` | Name, or nil if the entity no longer exists |

Attributes:

| Function | Notes |
|---|---|
| `get_attribute(scene_id, entity_id, name)` | Number, boolean, string or `{x, y}` table; nil if missing |
| `set_attribute(scene_id, entity_id, name, value)` | Coerces the value to the attribute's type; errors on a mismatch, a missing attribute, or an Integer out of range. Setting `x`, `y` or `position` moves the physics body |
| `has_attribute(scene_id, entity_id, name)` | |
| `create_attribute_float` / `_bool` / `_string` / `_int(scene_id, entity_id, name, value)` | |
| `create_attribute_vector2(scene_id, entity_id, name, x, y)` | |
| `update_entity_attribute_bool(scene_id, entity_id, name, value)` | Sets a Boolean attribute |

Game flow: `end_game()` ends the game; the runtime freezes on the current frame
in the `Ended` state.

## Example

Designer-defined attributes are the intended way to expose tuning knobs: add
an attribute in the Inspector and read it from the script.

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

`check_syntax(source)` parses a script without running it; the editor uses it
for live error reporting.

## Known limitations

- No `on_destroy` hook, and `on_collision` reports contacts beginning only.
- `get_mouse_position` is in window coordinates, not world coordinates.
- `script_state` is shared by all scripts; key collisions are the script
  author's problem.
- Entity ids are strings, re-parsed on every binding call.
