# Logger (`src/logger/`)

The global in-memory log behind the editor console. Every subsystem reports
through `logger::LOGGER`:

```rust
use rust_2d_game_engine::logger::LOGGER;

LOGGER.info("Game started");
LOGGER.warning(format!("Can't load image {}", path.display()));
LOGGER.error(format!("Script update() error: {}", e));
LOGGER.debug(line_of_cargo_output);
```

## Behavior

- Messages carry a level (`Info`, `Warning`, `Error`, `Debug`), a timestamp
  and a repeat **count**: a message identical to the previous one (same text
  and level) only bumps that message's count and timestamp, so a script failing
  every frame produces one console line, not thousands.
- The newest 1000 messages are kept.
- `revision()` changes whenever something is logged; the editor polls it each
  frame and copies the messages only when it changed.
- `clear()` empties the log.
- `LOGGER` is a `LazyLock<Logger>` with a `Mutex` inside, so build threads can
  log too.

The console shows `Info` and above by level filter; `Debug` messages (cargo
build output) have their own filter.

## Known limitations

- Nothing is written to disk; the log is lost when the editor exits.
- Messages are plain strings; there are no structured fields or per-module
  targets.
