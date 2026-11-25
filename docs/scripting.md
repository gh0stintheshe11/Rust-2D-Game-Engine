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
| `on_destroy(scene_id, entity_id)` | When a script removes the entity with `remove_entity`, **before** it goes, so the hook still reads it (where it was, for an explosion). It runs inside the `remove_entity` call; removing the entity again from it doesn't run it again, and an error in it is logged without stopping the removal. Not called when the scene changes or the game stops |

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

Input actions are the project's named controls (Project → Input Actions in
the editor, see [input.md](input.md#input-actions)): `"jump"` can stand for
Space, W and gamepad A, so scripts don't check each of them, and players can
rebind it.

| Function | Notes |
|---|---|
| `is_action_pressed(action)` | True while the action is held, through any of its bindings. An action the project doesn't have raises an error listing those it has |
| `is_action_just_pressed(action)` / `is_action_just_released(action)` | True only on the frame the action went down / up (a key tapped within one frame counts as both) |
| `get_action_strength(action)` | How far it's held, from 0 to 1: a stick or trigger bound to it holds it part way; it's pressed from its threshold on (0.5 unless the project sets another) |
| `get_action_axis(negative, positive)` | The strength of `positive` minus that of `negative`, from -1 to 1: `get_action_axis("left", "right")` to run, as fast as the stick is pushed |
| `get_action_bindings(action)` | The action's bindings, e.g. `{"key:Space", "pad:a"}` (written as in [input.md](input.md#input-actions)) |
| `set_action_bindings(action, bindings)` | Rebind an action, or add one, until the game stops (the next run starts with the project's bindings; keep a player's choice with `save_data`). Invalid bindings raise an error |
| `get_just_pressed_binding()` | Whatever went down this frame as a binding (`"key:Q"`, `"mouse:left"`, `"pad:y"`, `"pad:left_x-"`), or nil: for "press a key" menus |

```lua
function update(scene_id, entity_id)
    -- Run with the left / right actions (keys, d-pad or stick), jump with jump
    local _, vy = get_velocity(entity_id)
    if is_action_just_pressed("jump") then
        vy = -320
    end
    set_velocity(entity_id, 200 * get_action_axis("left", "right"), vy)
end
```

Keys, mouse buttons and gamepads, read directly:

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
| `play_sound(path [, options])` | Returns a play id, or nil when nothing can be played (missing file, no audio device; reported once per file). `options` is a table with `volume` (1.0 plays as recorded, 0.5 at half the amplitude; default 1.0), `loop` (start over at the end until stopped; default false), `group` (its volume group: `"sfx"` unless given) and `fade_in` (seconds to fade in from silence). Unknown options and negative volumes raise an error |
| `play_music(path [, options])` | Make a file the music, which loops in the `"music"` group; returns its play id, or nil. Options: `volume`, `crossfade` (seconds the music playing takes to fade out while this one fades in; 0, the default, switches at once), `loop` (default true). If that file is the music already, it plays on, fading to the new volume. Music carries over scene changes |
| `stop_music([seconds])` | Fade the music out over `seconds` (at once without) and stop it |
| `fade_sound(play_id, volume, seconds)` | Fade a playing sound's volume to `volume` over `seconds`, smoothly; accepts nil |
| `set_sound_volume(play_id, volume)` | Change a playing sound's volume; accepts nil |
| `set_group_volume(group, volume)` | Scale every sound of a volume group (`"sfx"`, `"music"`, or a name of your own), playing or started later: settings menus. Back to 1.0 when the game stops |
| `get_group_volume(group)` | A group's volume: 1.0 unless set |
| `set_master_volume(volume)` | Scales every sound, playing or started later; back to 1.0 when the game stops |
| `stop_sound(play_id [, seconds])` | Stop a sound, fading it out over `seconds` if given (it plays until it's silent); accepts nil |
| `is_sound_playing(play_id)` | False for nil; a looping sound plays until stopped |
| `stop_all_sounds()` | |

A sound plays at its own volume times its group's times the master volume.

```lua
function init(scene_id, entity_id)
    -- Each level asks for its music: a new track crossfades, the same one
    -- plays on
    play_music("assets/sounds/forest.ogg", {volume = 0.5, crossfade = 1})
    -- The player's setting, kept between runs
    set_group_volume("music", load_data("music_volume") or 1)
end
```

Tilemaps (see [scene.md](scene.md#tilemaps); cells are `column, row`
integers, and the entity must have a tilemap):

| Function | Notes |
|---|---|
| `get_tile(scene_id, entity_id, column, row)` | The tile number in the cell, or nil when it's empty |
| `set_tile(scene_id, entity_id, column, row, tile)` | Put a tile in the cell, or empty it with nil; returns whether the cell changed. The map grows as needed (up to 1024 columns and rows, with columns and rows from -1,048,576 to 1,048,576); a cell it can't add raises an error. A tilemap in the playing scene has its collider updated for that cell, which costs the same on a map of any size |
| `fill_tiles(scene_id, entity_id, column, row, columns, rows, tile)` | Put a tile (nil empties) in the `columns` × `rows` cells from (column, row): open a door, build a bridge. Returns how many cells changed; a rectangle the map can't take raises an error and changes nothing |
| `flood_fill_tiles(scene_id, entity_id, column, row, tile)` | A paint bucket: the cell and every cell connected to it through sides that holds what it holds, within the bounds of the map's tiles. Returns how many cells changed |
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
| `change_scene(scene [, options])` | Switch to another scene, given by name or id, once the current hooks finish, or with options through a transition; an unknown scene raises an error. See [Scenes and prefabs](#scenes-and-prefabs) |

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

## Timers and tweens

Timers call a function later, and tweens move numeric attributes over time.
Both run on game time (they wait while the game is paused), at the start of
each frame before the `update` hooks, so updates see where tweens are. They
belong to the entity whose script made them (`init`, `update` or another
hook, or a timer's or tween's callback): they stop when it's removed, when
the scene changes and when the game stops. Calling them from a script's top
level, outside its hooks, is an error.

| Function | Notes |
|---|---|
| `after(seconds, fn)` | Call `fn` once, `seconds` of game time from now (at least 0: `after(0, fn)` runs it next frame). Returns an id for `cancel` |
| `every(seconds, fn)` | Call `fn` every `seconds` (above 0) until it returns `false` or is cancelled, at most once a frame: a shorter interval runs it once a frame. An error in `fn` stops the timer. Returns an id |
| `tween(entity_id, values, seconds [, options])` | Move attributes of an entity of the playing scene from where they are when the tween starts to `values` (e.g. `{x = 300, opacity = 0}`) over `seconds`. They must be Float or Integer attributes (Integers are rounded). Moving `x` or `y` moves the entity's physics body too. Options: `ease` (`linear`, the default, `quad_in`, `quad_out`, `quad_in_out`, `cubic_…`, `sine_…`, `back_in`, `back_out`, `elastic_out`, `bounce_out`), `delay` (seconds before it starts), `loop` (`"restart"` or `"ping_pong"`: it never ends), `on_done` (a function called when it ends). Returns an id |
| `cancel(id)` | Stop a timer or tween; returns whether it was still there |

At most 10,000 timers and tweens wait at once (`MAX_TIMERS`); making more is
an error, which catches a script that makes them every frame.

```lua
function init(scene_id, entity_id)
    -- A coin that bobs forever, and fades out after 5 seconds (it has an
    -- opacity attribute: Add Feature > Sprite > Opacity in the inspector)
    local y = get_attribute(scene_id, entity_id, "y")
    tween(entity_id, {y = y - 8}, 0.6, {ease = "sine_in_out", loop = "ping_pong"})
    after(5, function()
        tween(entity_id, {opacity = 0}, 0.5, {on_done = function()
            remove_entity(scene_id, entity_id)
        end})
    end)
end
```

## Camera

The scene's camera (its default camera entity) can follow an entity and
shake. Both are done before the frame's `update` hooks, so an update that
places a background by the camera (`get_attribute(scene_id,
get_camera(scene_id), "x")`) puts it where the camera is painted.

| Function | Notes |
|---|---|
| `camera_follow(entity_id [, options])` | Keep the camera on an entity of the playing scene, from this frame on, until the scene changes, the entity is removed, or `camera_follow(nil)`. An entity with a physics body is followed by its middle, one without by its `x`/`y`. Options (world units): `offset_x`, `offset_y` (aim ahead of or above it), `smoothing` (how far behind the camera lags, in seconds: the time to get about two thirds of the way; 0, the default, locks it on), `deadzone_width`, `deadzone_height` (a box in the middle of the frame the entity moves in without the camera moving), `bounds = {x = ..., y = ..., width = ..., height = ...}` (an area the frame stays inside: the level), `snap = true` (jump there at once, whatever the smoothing) |
| `camera_shake(strength, seconds)` | Shake the camera by up to `strength` world units on each axis, fading out over `seconds` (above 0). Shakes at once don't add up: the strongest counts |
| `get_camera(scene_id)` | The scene's camera entity id, or nil |

```lua
function init(scene_id, entity_id)
    -- Follow the player a moment behind, without showing past the level
    camera_follow(entity_id, {
        smoothing = 0.12,
        bounds = {x = 0, y = 0, width = 3520, height = 540},
    })
end

function on_collision(scene_id, entity_id, other_id)
    if get_entity_name(scene_id, other_id) == "spikes" then
        camera_shake(6, 0.3)
    end
end
```

The shake moves the camera itself, so a background exactly as large as the
frame should move with the camera to keep its edges out of sight.

## Scenes and prefabs

`change_scene("level 2")` (a scene's name or id) switches the game to another
scene once the current pass of hooks finishes: the scripts' updates, or the
collision hooks of one physics step. The scene starts over as it was when
the game started (switching to the current scene restarts it), and its
entities run `init` again. `script_state`, the scripts' variables and the
sounds that are playing carry over. Stop returns to the scene that was active
when Play started.

With options, the screen goes to the other scene through a transition:

```lua
change_scene("level 2", {transition = "fade", seconds = 0.6, color = "#000000"})
```

A color covers the camera frame over the first half of `seconds` (0.5 when
not given), the scene changes then, and the new one shows over the second
half. `transition` is `fade` (the default), or a wipe: `wipe_left`,
`wipe_right`, `wipe_up` or `wipe_down`, a band of the color sweeping across
in that direction; `color` is `"#rrggbb"` or `"#rrggbbaa"` (black when not
given), and `seconds = 0` cuts. Both scenes play during it: the one being
left until it's covered. Changes asked for while the frame is being covered
are ignored (pressing Play twice doesn't start the level twice); after the
change, another transition covers the new scene again from where it is. Text
drawn by scripts is covered too.

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
    if is_action_just_pressed("jump") then
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
- Tweening `x` or `y` of a physics body moves it to each new place (like
  `set_attribute`), without a velocity: bodies resting on a tweened platform
  aren't carried along. Two tweens of one attribute both write it, and the
  later one wins.
- Timers fire at most once a frame, and on the frame's time: `after(0.01,
  ...)` waits for the next frame.
- Prefabs are single entities: there are no groups of entities or nested
  prefabs.
- `script_state` is shared by all scripts; key collisions are the script
  author's problem.
- Entity ids are strings, re-parsed on every binding call.
