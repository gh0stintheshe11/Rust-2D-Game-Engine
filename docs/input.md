# Input Handler (`src/input/`)

A per-frame snapshot of [egui](https://crates.io/crates/egui)'s input state. Each frame, `handle_input(&egui::InputState)` copies the current keyboard/mouse/modifier state into the handler; consumers then query it (`is_key_pressed`, `get_mouse_delta`, …).

## Key types

| Type | Responsibility |
|---|---|
| `InputHandler` | Holds held, just-pressed and just-released keys and mouse buttons, current + previous mouse position, the pointer in game coordinates, scroll delta, and modifiers |
| `InputAccess` | Which parts of a frame's input belong to the game: `keyboard` (keys and modifiers) and `pointer` (mouse buttons and the scroll wheel); `InputAccess::ALL` gives it everything |

"Just pressed" and "just released" come from the frame's key and pointer
button events, and also from comparing what is held with the previous frame.
So a key or button pressed and released between two frames is still seen:
just pressed and just released in the same frame, without ever being held.
Key repeats aren't new presses.

`handle_input_with(&input, access)` takes in only what `access` allows;
withheld keys and buttons count as released (a key held when the keyboard is
withheld is just released; held when it comes back, just pressed). The mouse
position is tracked either way, and keeps its last value when egui reports no
hover position.

The pointer's **game position** isn't computed here: the runtime maps the
window position through its camera every frame and records it with
`set_game_pointer(world, screen)` (see [runtime.md](runtime.md)).

## Interactions with other modules

- **Editor** keeps its own handler for viewport camera controls, fed only while the game isn't playing.
- **`runtime`** owns the game's handler and feeds it every frame in `GameRuntime::update`, deciding the `InputAccess`.
- **`scripting`** binds `is_key_just_pressed` etc. for scripts and exposes `get_all_active_inputs()` as a Lua table (`bind_keys_pressed`).

## Public API overview

- **Lifecycle**: `new`, `handle_input(&egui::InputState)`, `handle_input_with(&egui::InputState, InputAccess)`
- **Keyboard**: `is_key_pressed`, `is_key_just_pressed`, `is_key_just_released`
- **Mouse**: `is_mouse_button_pressed`, `is_mouse_button_just_pressed`, `is_mouse_button_just_released`, `get_mouse_pos` (window coordinates), `get_mouse_delta`, `get_scroll_delta`
- **Game pointer**: `set_game_pointer(world, screen)`, `get_mouse_world_pos`, `get_mouse_screen_pos`
- **Introspection**: `get_all_active_inputs` — sorted names of held modifiers (`"Ctrl"`, `"Shift"`, `"Alt"`, `"Cmd"` for the macOS Command key), keys (egui names such as `"Space"`, `"A"`, `"ShiftLeft"`) and mouse buttons (`"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"`)
- **Name lookup**: `parse_key(name)` (egui key names, case-insensitive), `parse_mouse_button(name)` (`"left"`/`"right"`/`"middle"`, optional `"Mouse"` prefix, case-insensitive)

## Known limitations

- **`get_mouse_delta` always returns `Some`** — the `Option` is meaningless (unlike `get_scroll_delta`, which returns `None` when zero).
- A key pressed and released several times within one frame counts once.
- No rebinding/mapping layer, no gamepad support, no text input — entirely tied to egui's `InputState`.
