# Input Handler (`src/input/`)

A per-frame snapshot of [egui](https://crates.io/crates/egui)'s input state. Each frame, `handle_input(&egui::InputState)` copies the current keyboard/mouse/modifier state into the handler; consumers then query it (`is_key_pressed`, `get_mouse_delta`, …).

## Key types

| Type | Responsibility |
|---|---|
| `InputHandler` | Holds pressed/just-pressed key sets, mouse buttons, current + previous mouse position, scroll delta, and modifiers |

"Just pressed" is computed by diffing the current `keys_down` against the previous frame's set. Mouse position retains its last value when egui reports no hover position.

## Interactions with other modules

- **Editor** keeps its own handler for viewport camera controls, fed only while the game isn't playing.
- **`runtime`** owns the game's handler and feeds it every frame in `GameRuntime::update`.
- **`scripting`** binds `is_key_just_pressed` etc. for scripts and exposes `get_all_active_inputs()` as a Lua table (`bind_keys_pressed`).

## Public API overview

- **Lifecycle**: `new`, `handle_input(&egui::InputState)`
- **Keyboard**: `is_key_pressed`, `is_key_just_pressed`
- **Mouse**: `is_mouse_button_pressed`, `get_mouse_pos`, `get_mouse_delta`, `get_scroll_delta`
- **Introspection**: `get_all_active_inputs` (string list of held modifiers, keys, and mouse buttons)

## Known limitations / TODO

- **`get_mouse_delta` always returns `Some`** — the `Option` is meaningless (unlike `get_scroll_delta`, which returns `None` when zero).
- **Keys tapped within a single frame are missed** (press+release between two `handle_input` calls); there are no key-released or mouse just-pressed/released events.
- No rebinding/mapping layer, no gamepad support, no text input — entirely tied to egui's `InputState`.
