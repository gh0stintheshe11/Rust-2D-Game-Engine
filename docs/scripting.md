# Lua Scripting (`src/scripting/`)

Game logic is written in Lua 5.4 files attached to entities (`entity.script`).
While the game plays, the engine calls each script's hooks for its entity.

## Sessions

A **session** spans one Play (from ▶ until Stop/Reset/restart), including
scene changes (see [Scenes and prefabs](#scenes-and-prefabs)).
`GameRuntime::run` calls `start_session(scene_manager, physics, input, audio)`,
which creates a fresh sandboxed VM, resets the globals, registers all engine
bindings, and keeps a copy of the scenes for `spawn` to copy templates from.
Bindings hold `Rc<RefCell<…>>` handles to the runtime's subsystems (no
`unsafe`, no raw pointers), and never call back into Lua while holding a
borrow.

Every rendered frame the runtime calls `update_global_time(dt)`,
`bind_keys_pressed(&input)` and `run_scripts_for_scene(scene_id)`; after
every physics step it calls `dispatch_collision_events(scene_id)`.

## Hooks

| Hook | When |
|---|---|
| `init(scene_id, entity_id)` | Once per entity and visit of its scene, before any other hook of that entity (spawned entities get it on their first frame) |
| `update(scene_id, entity_id)` | Every rendered frame |
| `on_collision(scene_id, entity_id, other_id)` | When a contact with another physics entity **begins**, or an overlap with a sensor (checked after every physics step; skipped if either entity was removed meanwhile) |
| `on_collision_end(scene_id, entity_id, other_id)` | When such a contact **ends**: the entities separated, or the other one was removed (`get_entity_name` then returns nil). An entity's ended contacts are reported before its new ones |

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
| `get_velocity(entity_id)` | `vx, vy` of the entity's physics body, px/s (+y is down): e.g. to set the horizontal speed and keep the vertical one |
| `drop_through_platforms(entity_id)` | The body falls through the one-way tiles it stands on (see [physics.md](physics.md#tilemaps)), until it's clear of them; on solid ground nothing happens. A platformer calls it when down is pressed |
| `apply_force(entity_id, x, y)` | Pushes for the current frame; call it every frame for a continuous force (independent of the frame rate) |
| `apply_impulse(entity_id, x, y)` | Instant change of momentum |
| `set_angular_velocity(entity_id, radians_per_second)` | Positive spins clockwise on screen; the body needs `can_rotate` |
| `add_entity_to_physics_engine(entity_id)` | Builds the body from the entity's attributes (active scene); re-adding replaces it, e.g. after changing physics attributes |
| `remove_entity_from_physics_engine(entity_id)` | |
| `get_colliding_entities(entity_id)` | Array of entity ids currently touching, or overlapping when either is a sensor; empty if the entity has no body |
| `raycast(x1, y1, x2, y2 [, exclude_id])` | `{id, x, y, distance}` of the first collider the segment from (x1, y1) to (x2, y2) hits, or nil. Sensors are ignored, and so is the entity `exclude_id` (pass the caster when the ray starts inside it). Sees bodies as of the last physics step |
| `set_gravity(x, y)` | Global gravity (+y is down; default `(0, 50)`, restored on every Play) |

```lua
function update(scene_id, entity_id)
    -- Standing on something? Look 2 px below the feet (a 32 x 32 sprite)
    local x = get_attribute(scene_id, entity_id, "x")
    local y = get_attribute(scene_id, entity_id, "y")
    local ground = raycast(x + 16, y + 32, x + 16, y + 34, entity_id)
    if ground and is_key_just_pressed("Space") then
        set_velocity(entity_id, 0, -260)
    end
end
```

Input:

| Function | Notes |
|---|---|
| `is_key_just_pressed(key)` | True only on the frame the key went down, even if it was released before the frame ended. Key names are egui's (`"Space"`, `"A"`, `"ArrowUp"`), case-insensitive; unknown names raise an error |
| `is_key_just_released(key)` | True only on the frame the key went up |
| `is_key_pressed(key)` | True while held |
| `is_mouse_pressed(button)` | True while held: `"left"`, `"right"` or `"middle"` (also `"MouseLeft"` etc.) |
| `is_mouse_just_pressed(button)` / `is_mouse_just_released(button)` | True only on the frame the button went down / up |
| `get_mouse_position()` | `{x, y}` in world coordinates: where the pointer is in the game, through the camera (the same in the editor and in exported games) |
| `get_mouse_screen_position()` | `{x, y}` in the camera frame's pixels, the coordinates `draw_text` uses: (0, 0) is the frame's top-left corner |
| `get_scroll_delta()` | `{x, y}`; zero when not scrolling |
| `is_gamepad_pressed(button [, pad])` | True while a gamepad button is held: `"a"`, `"b"`, `"x"`, `"y"` (named after their place on an Xbox pad: on a PlayStation pad, cross, circle, square, triangle), `"lb"`, `"rb"`, `"lt"`, `"rt"`, `"back"`, `"start"`, `"ls"`, `"rs"` (stick clicks), `"up"`, `"down"`, `"left"`, `"right"` (the d-pad). `pad` counts from 1 in the order the pads connected; without it, any pad counts |
| `is_gamepad_just_pressed(button [, pad])` / `is_gamepad_just_released(button [, pad])` | True only on the frame the button went down / up |
| `get_gamepad_axis(axis [, pad])` | `"left_x"`, `"left_y"`, `"right_x"`, `"right_y"`: -1 to 1, +y **down** like the world; `"lt"`, `"rt"` (triggers): 0 to 1. Dead zones are applied (see [input.md](input.md#gamepads)). Without `pad`, the first pad where the axis isn't at rest; 0 without a pad |
| `get_gamepad_count()` | How many gamepads are connected |

```lua
function update(scene_id, entity_id)
    -- Shoot towards the pointer
    if is_mouse_just_pressed("left") then
        local target = get_mouse_position()
        local x = get_attribute(scene_id, entity_id, "x")
        local y = get_attribute(scene_id, entity_id, "y")
        local bullet = spawn(scene_id, "prefabs/bullet", x, y)
        local dx, dy = target.x - x, target.y - y
        local length = math.max(math.sqrt(dx * dx + dy * dy), 1)
        set_velocity(bullet, dx / length * 400, dy / length * 400)
    end
end
```

In the editor, the game gets the keyboard unless an editor text field has
focus, and mouse buttons and the scroll wheel only while the pointer is over
the game, not over panels or the play buttons.

Drawing and animation:

| Function | Notes |
|---|---|
| `play_animation(scene_id, entity_id, frames [, options])` | Play frames of the entity's sprite sheet (it needs a `frame_size`, see [render.md](render.md)): `frames` like `"0-3"` or `"0,1,2,1"`. Options: `fps` (default 10), `loop` (default true), `restart`. Playing the animation that is already playing keeps it running, so it can be called every frame; other frames, or `restart = true`, start over. Creates the animation attributes the entity lacks; invalid frames or options raise an error |
| `emit_particles(scene_id, entity_id, count [, x, y])` | A burst of `count` particles (at most 1,000) from a particle emitter (an entity with `particle_rate`, see [render.md](render.md#particles)), at its position or at `x`, `y`. The particles play out even if the emitter is removed right after. Errors for an entity that isn't an emitter |
| `draw_text(text, x, y [, options])` | Draw text over the game for the current frame; call it every frame, from `update`. Numbers are converted to text. `x`/`y` (the top of the text) and sizes are in the camera frame's pixels: (0, 0) is its top-left corner, the frame is the camera's `width` × `height`, and the text scales with it like the sprites. Options: `size` (font height, default 24), `color` (`"#rrggbb"` or `"#rrggbbaa"`, default white), `align` (`"left"`, `"center"` or `"right"` of `x`; default left). Text stays on screen while the game is paused or over |

```lua
function update(scene_id, entity_id)
    draw_text("Score: " .. script_state.state.score, 10, 10, {size = 28, color = "#ffd84a"})

    -- Frames 0-3 of the sheet while running, frame 4 while standing still
    if is_key_pressed("ArrowRight") then
        play_animation(scene_id, entity_id, "0-3", {fps = 12})
    else
        play_animation(scene_id, entity_id, "4")
    end
end
```

Game UI, drawn over the game for the current frame like `draw_text` (call
these every frame, from `update`; positions and sizes in the camera frame's
pixels; later drawings cover earlier ones):

| Function | Notes |
|---|---|
| `draw_rect(x, y, width, height [, options])` | A filled rectangle: panels, health bars. Options: `color` (default white), `radius` (corner radius, default 0), `outline` (a color), `outline_width` (default 2 with an outline) |
| `draw_image(path, x, y [, options])` | An image file (project-relative path): icons, lives. Without `width` and `height` it has its own size; with one of them, its proportions. Options: `width`, `height`, `color` (tint), `opacity` (0 to 1) |
| `button(label, x, y, width, height [, options])` | Draws a button and returns whether it was clicked this frame (pressed and released with the left mouse button on it, or pressed with Enter or gamepad A while it has the focus) and whether it has the keyboard and gamepad focus. It's lighter while hovered or focused, darker while pressed, and outlined while focused. Options: `size` (label height, default 22), `color` (label), `background`, `hover`, `pressed`, `radius` (default 8), `outline`, `outline_width`, `focus` (the focus outline's color), `default` (true: it takes the focus when the buttons appear, so Enter or A press it right away), `focusable` (false: keyboard and gamepad never focus it, e.g. a button over the game while playing), `id` (who the button is for the focus, when its label changes) |

Colors are `"#rrggbb"` or `"#rrggbbaa"`; unknown options, values of the
wrong type and negative sizes raise an error.

**Focus.** One button at a time has the keyboard and gamepad focus. The up
and left arrows, Shift+Tab, the d-pad's up and left and the left stick
pushed up or left move it to the previous button, in the order the buttons
are drawn; the down and right arrows, Tab and the other directions to the
next one, wrapping around at the ends. The stick moves it once per push.
The mouse moving over a button focuses it. Without a `default` button, no
button has the focus until one of those moves it. Buttons are known by their
label (or `id`), so the focus stays on a button that moves.

```lua
-- A title screen: Play has the focus first, so Enter or gamepad A start
function update(scene_id, entity_id)
    draw_rect(440, 180, 400, 330, {color = "#10142080", radius = 12})
    draw_text("My Game", 640, 210, {size = 48, align = "center"})
    if button("Play", 540, 300, 200, 60, {default = true}) then
        change_scene("level 1")
    end
    if button("Quit", 540, 390, 200, 60) then
        end_game()
    end
end
```

Audio:

| Function | Notes |
|---|---|
| `play_sound(path [, options])` | Returns a play id, or nil when nothing can be played (missing file, no audio device; reported once per file). `options` is a table with `volume` (1.0 plays as recorded, 0.5 at half the amplitude; default 1.0) and `loop` (start over at the end until stopped; default false). Unknown options and negative volumes raise an error |
| `set_sound_volume(play_id, volume)` | Change a playing sound's volume; accepts nil |
| `set_master_volume(volume)` | Scales every sound, playing or started later; back to 1.0 when the game stops |
| `stop_sound(play_id)` | Accepts nil |
| `is_sound_playing(play_id)` | False for nil; a looping sound plays until stopped |
| `stop_all_sounds()` | |

```lua
function init(scene_id, entity_id)
    -- Background music for the whole game
    script_state.state.music = play_sound("assets/sounds/music.ogg", {loop = true, volume = 0.4})
end
```

Tilemaps (see [scene.md](scene.md#tilemaps); cells are `column, row`
integers, and the entity must have a tilemap):

| Function | Notes |
|---|---|
| `get_tile(scene_id, entity_id, column, row)` | The tile number in the cell, or nil when it's empty |
| `set_tile(scene_id, entity_id, column, row, tile)` | Put a tile in the cell, or empty it with nil; returns whether the cell changed. The map grows as needed (up to 1024 columns and rows). A tilemap in the playing scene gets its collider rebuilt |
| `get_tile_cell(scene_id, entity_id, x, y)` | `column, row` of the cell containing the world position |

```lua
-- Break the block the player bumps from below
local column, row = get_tile_cell(scene_id, ground_id, x + 8, y - 1)
if get_tile(scene_id, ground_id, column, row) == BRICK then
    set_tile(scene_id, ground_id, column, row, nil)
end
```

Save data (kept between runs of the game; see [save.md](save.md)):

| Function | Notes |
|---|---|
| `save_data(key, value)` | Save a boolean, number, string, or a table of those (a list, or with string keys); `nil` removes the key. Written to disk right away; returns false (and logs why) when writing failed, the value then lasting until the game stops. Unsupported values raise an error |
| `load_data(key)` | The value saved under the key, or nil |

```lua
function init(scene_id, entity_id)
    script_state.state.best = load_data("best") or 0
end

-- ...when the round ends:
--     state.best = math.max(state.best, state.score)
--     save_data("best", state.best)
```

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
| `find_entity(scene_id, name)` | Id of the scene's first entity with that name, or nil |
| `spawn(scene_id, template, x, y)` | Copy a template entity to (x, y) and return the copy's id; see [Scenes and prefabs](#scenes-and-prefabs) |
| `change_scene(scene)` | Switch to another scene, given by name or id, once the current hooks finish; an unknown scene raises an error |

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

## Scenes and prefabs

`change_scene("level 2")` (a scene's name or id) switches the game to another
scene once the current pass of hooks finishes: the scripts' updates, or the
collision hooks of one physics step. The scene starts over as it was when
the game started (switching to the current scene restarts it), and its
entities run `init` again. `script_state`, the scripts' variables and the
sounds that are playing carry over. Stop returns to the scene that was active
when Play started.

`spawn(scene_id, template, x, y)` copies an entity, with its attributes,
images and script, puts the copy's top-left corner at (x, y) and returns its
id. Copies in the playing scene get their physics body right away and run
`init` on their first frame. `template` is `"Scene/Entity"`, or `"Entity"`,
looked up in the scene `scene_id` first and then in the others. Templates are
copied as they were when the game started, so changing or removing them while
playing doesn't affect the copies.

A scene that never plays makes a prefab library: design the bullets, enemies
and coins in a scene named "prefabs", and keep another scene active.

```lua
function update(scene_id, entity_id)
    if is_key_just_pressed("Space") then
        local x = get_attribute(scene_id, entity_id, "x")
        local y = get_attribute(scene_id, entity_id, "y")
        local bullet = spawn(scene_id, "prefabs/bullet", x + 12, y - 8)
        set_velocity(bullet, 0, -400)
    end
end

function on_collision(scene_id, entity_id, other_id)
    if get_entity_name(scene_id, other_id) == "exit" then
        change_scene("level 2")
    end
end
```

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

- The focus moves among buttons in drawing order, not by direction on
  screen, and the keys and gamepad buttons that move and press it are also
  seen by the game's scripts (`is_key_just_pressed` and friends), like a
  click on a button.
- No `on_destroy` hook.
- Prefabs are single entities: there are no groups of entities or nested
  prefabs.
- `script_state` is shared by all scripts; key collisions are the script
  author's problem.
- Entity ids are strings, re-parsed on every binding call.
