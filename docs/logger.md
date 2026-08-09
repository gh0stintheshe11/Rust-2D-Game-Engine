# Logger (`src/logger/`)

A global, in-memory logger for the editor's console panel. `LOGGER` is a `once_cell::Lazy` static; any module can call `LOGGER.info/warning/error/debug(...)`. Messages (`text` + local timestamp + `ConsoleMessageType`) are pushed into two mutex-guarded buffers: a *console* buffer capped at the last 1 000 messages and a *stored* buffer capped at 50 000.

## Key types

| Type | Responsibility |
|---|---|
| `Logger` / `LOGGER` | Global sink; `info`/`warning`/`error`/`debug` plus `get_console_messages`/`get_stored_messages` |
| `ConsoleMessage` | `text`, `timestamp` (chrono `Local`), `message_type` |
| `ConsoleMessageType` | `Info` \| `Warning` \| `Error` \| `Debug` |

## Interactions with other modules

- **`engine_gui`** polls `get_console_messages()` every frame to render the console panel.
- **`audio_engine`** warns when no output device is available.
- **`project_manager`** streams `cargo build` output as `Debug` messages.

## Known limitations / TODO

- Not wired into the `log`/`tracing` ecosystems and writes nothing to stdout/file — messages are only visible in the editor console. Many modules still use bare `println!` instead.
- `get_console_messages` **clones the whole buffer** and the GUI calls it every frame.
- `get_stored_messages` currently has no callers (no log export feature).
- No level filtering; `.lock().unwrap()` panics if a mutex is ever poisoned.
