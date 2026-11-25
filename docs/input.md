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
`set_game_pointer(world, screen)` (see [runtime.md](runtime.md)). When the
left button has just gone down, that screen position is also kept as where
it was pressed (`get_mouse_press_screen_pos`): scripts' buttons are clicked
when the press and the release are both on them.

## Gamepads

Gamepads come in as `GamepadEvent`s (connected, disconnected, button
pressed or released, axis moved), which `handle_gamepad_events(events)`
takes in once per frame, also without events, since it starts the frame
like `handle_input` does for the keyboard: a button pressed and released
within one frame is just pressed and just released. A pad that sends input
before it's announced counts as connected.

Buttons (`GamepadButton`) are named after their place on an Xbox-style pad;
`parse_gamepad_button` takes the names scripts use (`"a"`, `"lb"`, `"up"`,
…, and PlayStation names like `"cross"`, see `GAMEPAD_BUTTONS`). Axes
(`GamepadAxis`, `parse_gamepad_axis`) are the two sticks, from -1 to 1 with
+x right and +y **down** like world coordinates, and the two triggers, from
0 to 1. Sticks have a round dead zone (`STICK_DEADZONE`, 0.15): closer to
the middle reads as 0, so diagonals behave like the other directions, and
past it the value starts again from 0, so there's no jump. Triggers read 0
below `TRIGGER_DEADZONE` (0.05).

Pads are numbered in the order they connected (from 0 here, from 1 in
scripts); `None` asks about any pad: a button held on any of them, or an
axis of the first pad where it isn't at rest.

`GamepadBackend` reads the system's gamepads through
[gilrs](https://crates.io/crates/gilrs), which knows the button layout of
common pads (SDL's controller database): `open()` starts it (`None`, with a
message in the console, where the system has no gamepad support or its
devices can't be read), and `poll()` returns the events since the last call,
announcing the pads already connected the first time. Stick Y axes are
flipped to point down, analog triggers become the trigger axes, and d-pads
reported as axes become d-pad buttons. Building on Linux needs libudev's
development files (`libudev-dev` on Debian and Ubuntu).

## Interactions with other modules

- **Editor** keeps its own handler for viewport camera controls, fed only while the game isn't playing.
- **`runtime`** owns the game's handler and feeds it every frame in `GameRuntime::update`, deciding the `InputAccess`, followed by the frame's gamepad events: the system's, once `set_gamepads(true)` (exported games and the editor do it), and those queued with `queue_gamepad_events` (tests).
- **`scripting`** binds `is_key_just_pressed` etc. for scripts and exposes `get_all_active_inputs()` as a Lua table (`bind_keys_pressed`).

## Public API overview

- **Lifecycle**: `new`, `handle_input(&egui::InputState)`, `handle_input_with(&egui::InputState, InputAccess)`
- **Keyboard**: `is_key_pressed`, `is_key_just_pressed`, `is_key_just_released`
- **Mouse**: `is_mouse_button_pressed`, `is_mouse_button_just_pressed`, `is_mouse_button_just_released`, `get_mouse_pos` (window coordinates), `get_mouse_delta`, `get_scroll_delta`
- **Game pointer**: `set_game_pointer(world, screen)`, `get_mouse_world_pos`, `get_mouse_screen_pos`, `get_mouse_press_screen_pos`
- **Gamepads**: `handle_gamepad_events`, `gamepad_count`, `is_gamepad_pressed`, `is_gamepad_just_pressed`, `is_gamepad_just_released`, `gamepad_axis`; `GamepadBackend::open`, `poll`
- **Introspection**: `get_all_active_inputs` — sorted names of held modifiers (`"Ctrl"`, `"Shift"`, `"Alt"`, `"Cmd"` for the macOS Command key), keys (egui names such as `"Space"`, `"A"`, `"ShiftLeft"`) and mouse buttons (`"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"`)
- **Name lookup**: `parse_key(name)` (egui key names, case-insensitive), `parse_mouse_button(name)` (`"left"`/`"right"`/`"middle"`, optional `"Mouse"` prefix, case-insensitive)

## Known limitations

- **`get_mouse_delta` always returns `Some`** — the `Option` is meaningless (unlike `get_scroll_delta`, which returns `None` when zero).
- A key pressed and released several times within one frame counts once.
- No rebinding/mapping layer and no text input; keyboard and mouse are entirely tied to egui's `InputState`.
- Gamepads: no rumble, and pads are renumbered when one disconnects (the later ones move up). Analog buttons other than the triggers read as pressed or not, and pads gilrs has no mapping for may report their triggers on axes the engine ignores. Gamepads are read even while the window isn't focused.
