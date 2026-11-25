# Runtime (`src/runtime/`)

`GameRuntime` owns play mode: the state machine, the per-frame game loop, and
the plumbing between scenes, scripts, physics, audio and rendering. The editor
and the standalone player ([`src/player.rs`](../src/player.rs)) call
`update(ctx, ui, viewport_rect)` every frame, which is:

1. feed egui's input into the `InputHandler` (see below),
2. `tick(frame_dt)` — advance the simulation (headless, used directly by tests),
3. `paint(ui, viewport_rect)` — draw the scene (skipped when stopped).

## Input

`update` registers a click-only "game surface" widget over the viewport, and
gives the game:

- the keyboard, unless another egui widget has keyboard focus (in the
  editor: a text field);
- mouse buttons and the scroll wheel while the surface is the topmost widget
  under the pointer, or a press that started on it is held. Widgets added
  later in the frame, like the editor's play buttons, and windows above it
  keep their clicks.

It then maps the pointer's window position through the camera that paints
this frame (the free camera, or the game camera fitted into the viewport) and
records it in the `InputHandler`: in world coordinates, and in the camera
frame's pixels, the space `draw_text` lays text out in. `input()` returns the
handler, e.g. for tests.

## Ownership

The scene manager, physics engine, input handler and audio engine are
`Rc<RefCell<…>>` because Lua bindings hold handles to them (see
[scripting.md](scripting.md)). The `RenderEngine` is a plain field.

```mermaid
graph TD
    U["editor / player"] -->|"update() every frame"| GR[GameRuntime]
    GR --> SM["Rc&lt;RefCell&lt;SceneManager&gt;&gt;"]
    GR --> PE["Rc&lt;RefCell&lt;PhysicsEngine&gt;&gt;"]
    GR --> IH["Rc&lt;RefCell&lt;InputHandler&gt;&gt;"]
    GR --> AE["Rc&lt;RefCell&lt;AudioEngine&gt;&gt;"]
    GR --> RE[RenderEngine]
    GR --> LS[LuaScripting]
    LS -.->|bindings hold Rc handles| SM
    LS -.-> PE
    LS -.-> IH
    LS -.-> AE
```

## State machine

| Call | From → to | What happens |
|---|---|---|
| `run()` | Stopped → Playing | Activates the first scene if none is active, remembers the scenes' initial state, loads a fresh physics world (default gravity), starts a fresh Lua session. Running from another state resets first |
| `pause()` / `resume()` | Playing ⇄ Paused | Simulation and audio freeze; the scene keeps rendering |
| script calls `end_game()` | Playing → Ended | Sounds stop and the last frame stays visible |
| `reset()` | any → Stopped | Physics, render caches and audio cleared; scenes restored to their initial state |
| `restart()` | any → Playing | `reset()` then `run()` (the player's "play again") |

## One tick (while Playing)

1. The frame time is capped at 5 physics steps (1/12 s at 60 Hz). Slower
   frames run the whole game in slow motion, so scripts and physics always
   see the same time.
2. `physics.begin_frame(frame_dt)` — forces scripts apply this frame act for
   this frame's duration.
3. Lua: `accumulated_time += frame_dt`, `keys_pressed` refreshed, every
   scripted entity's `update` runs (once per rendered frame).
4. Physics runs as many fixed steps (`1/target_fps`) as the accumulated time
   allows. After **each step**, `on_collision_end` and `on_collision` fire for
   the contacts that step ended and started, so short contacts aren't missed
   between frames.
5. Sprite sheet animations advance by the frame time.
6. Finished sounds are reaped.

When a script calls `end_game()` or `change_scene()`, the request takes
effect as soon as the pass of hooks that made it finishes (step 3, or one
physics step's collision hooks), and the rest of the tick is skipped. Script
errors are logged to the editor console and never stop the loop.

## Switching scenes

A game can span several scenes: a title screen, levels, a game-over screen.
`switch_scene(scene_id)`, which the `change_scene` script binding triggers,
makes another scene the active one while the game runs:

- The scene is replaced by a copy of how it was when the game started, so
  every visit starts it over; switching to the current scene restarts it.
- A new physics world is loaded from it, keeping the current gravity.
- Its scripted entities run `init` again, and their contacts and drawn text
  start empty (`LuaScripting::begin_scene`).
- The Lua session carries over: `script_state`, the scripts' variables, and
  the sounds that are playing.

The scene that was left keeps its state until it is entered again. `reset()`
restores every scene, and which one is active, to how they were at `run()`.
`switch_scene` fails when the game is stopped or the scene doesn't exist.

## Save data

The runtime owns the game's `SaveData` (see [save.md](save.md)) and shares it
with every scripting session, so it outlives Stop and Play. A new runtime
keeps it in memory; `set_save_data` points it at a file (the player uses the
user's data folder, the editor the project's `saves/` folder), and
`save_data()` gives access, e.g. to clear it.

## Painting and cameras

`set_camera_mode` picks the view:

- `CameraMode::Game` (default, used by exported games): the scene's default
  camera entity, scaled to fit the viewport and letterboxed with black bars
  (see `render::game_view`).
- `CameraMode::Free { position, zoom }`: a camera controlled by the caller.
  The editor shows play mode through its own pan/zoom camera and draws the
  game camera's frame as a red rectangle.

Sprites, then the text scripts drew with `draw_text` this frame (laid out in
the camera frame, see [render.md](render.md)), are clipped to the visible
area. Scripts' text is collected while they run and cleared when the next
frame's scripts start, so a paused or ended game keeps showing it.

Collider outlines (a debugging aid) are drawn on top only after
`set_show_colliders(true)`: exported games never show them, and the editor
shows them while playing unless View → Collider Outlines is unchecked.

## Known limitations

- Scripts run once per rendered frame, so per-frame counters depend on the
  refresh rate; use `accumulated_time` for timing.
- Shared entities (`SceneManager::shared_entities`) never reach physics or
  scripts.
- In the editor, the game only advances while the Viewer tab is shown.
- Scene switches are immediate, with no transition effect, and every scene
  stays in memory twice while playing: as played, and as it was at `run()`.
