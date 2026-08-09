# Input Handler (`src/input_handler/`)

A per-frame snapshot of [egui](https://crates.io/crates/egui)'s input state. Each frame, `handle_input(&egui::InputState)` copies the current keyboard/mouse/modifier state into the handler; consumers then query it (`is_key_pressed`, `get_mouse_delta`, …). It also carries an `InputContext` flag (`EngineUI` vs `Game`) that marks *who should be consuming input* — editor UI or the running game.

## Key types

| Type | Responsibility |
|---|---|
| `InputHandler` | Holds pressed/just-pressed key sets, mouse buttons, current + previous mouse position, scroll delta, and modifiers |
| `InputContext` | Enum: `EngineUI` \| `Game` |

"Just pressed" is computed by diffing the current `keys_down` against the previous frame's set. Mouse position retains its last value when egui reports no hover position.

## Interactions with other modules

- **`engine_gui`** creates the handler (starting in `EngineUI` context) and feeds it egui's `InputState`.
- **`game_runtime`** holds its **own clone** (`InputHandler` is `Clone`), exposed via `get_input_handler()`; it flips the context to `Game` on play and back to `EngineUI` on stop.
- **`lua_scripting`** binds `is_key_just_pressed` etc. for scripts and exposes `get_all_active_inputs()` as a Lua table (`bind_keys_pressed`).

## Public API overview

- **Lifecycle**: `new`, `handle_input(&egui::InputState)`
- **Context**: `get_context`, `set_context`
- **Keyboard**: `is_key_pressed`, `is_key_just_pressed`
- **Mouse**: `is_mouse_button_pressed`, `get_mouse_pos`, `get_mouse_delta`, `get_scroll_delta`
- **Introspection**: `get_all_active_inputs` (string list of held modifiers, keys, and mouse buttons)

## Known limitations / TODO

- **Context is informational only** — nothing in the handler filters or routes input by context; each consumer must check `get_context()` itself.
- **Two copies exist at runtime**: the GUI's handler and the `GameRuntime`'s clone are separate objects; whichever one receives `handle_input` has the real state. Easy to desync.
- **`get_mouse_delta` always returns `Some`** — the `Option` is meaningless (unlike `get_scroll_delta`, which returns `None` when zero).
- **Keys tapped within a single frame are missed** (press+release between two `handle_input` calls); there are no key-released or mouse just-pressed/released events.
- No rebinding/mapping layer, no gamepad support, no text input — entirely tied to egui's `InputState`.
- `set_context` logs with `println!` instead of the engine `LOGGER`.
