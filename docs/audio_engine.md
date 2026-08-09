# Audio Engine (`src/audio_engine/`)

A thin wrapper around [rodio](https://crates.io/crates/rodio) for sound playback. Sound files are read into memory once and cached by a deterministic UUID (SHA-256 of the path), then decoded per play into `rodio::Sink`s. The engine **degrades gracefully when no audio output device exists** (headless/CI): construction never fails — `output` stays `None`, a warning is logged via `LOGGER`, `has_output()` reports `false`, and any playback attempt returns `Err` instead of panicking.

## Key type

`AudioEngine` is the only type:

| Field | Role |
|---|---|
| `output: Option<(OutputStream, OutputStreamHandle)>` | `None` when no output device is available |
| `sound_cache: HashMap<Uuid, Vec<u8>>` | Raw (still-encoded) file bytes, keyed by deterministic path-UUID |
| `active_sounds: HashMap<Uuid, Sink>` | Live playbacks, keyed by a **random per-play UUID** (not the path UUID) |
| `immediate_sink: Option<Sink>` | Single slot for "play now" sounds; starting a new one stops the previous |
| `duration_cache: HashMap<Uuid, f32>` | Durations recorded at load time (currently never read back) |

Two ID spaces: `load_sound` returns the deterministic *sound id* (same path → same id), while `play_sound` returns a fresh *play id* per invocation — that's the handle for `stop`/`pause`/`resume`.

## Interactions with other modules

- **`game_runtime`** owns an instance; calls `update()` every frame (reaps finished sinks) and `cleanup()` on stop/reset.
- **Editor GUI** (`gui/inspector.rs`) calls `play_sound_immediate` to preview audio files.
- **`ecs`**: `load_entity_sounds`/`load_scene_sounds` read `Entity::sounds` paths — but nothing in `src/` currently calls them (or `play_sound`); only tests do.
- **`logger`**: warns once at construction when no output device is found.
- **`lua_scripting`**: no bindings — scripts cannot play audio.

## Public API overview

- **Device**: `has_output`
- **Loading**: `load_entity_sounds`, `load_scene_sounds` (per-file `load_sound` is private; loading also probes duration via [lofty](https://crates.io/crates/lofty))
- **Playback**: `play_sound(path) -> play_id`, `play_sound_immediate(path)`, `stop_immediate()`
- **Control**: `stop(play_id)` (also removes it), `pause(play_id)`, `resume(play_id)`
- **Status**: `is_playing`, `is_paused`, `is_stopped`, `list_playing_sounds`
- **Maintenance**: `update()` (drop finished sinks), `stop_all()`
- **Memory**: `cleanup()` (stop everything + clear caches), `clear_cache()`, `unload_sound(path)`, `get_memory_usage()` (sum of cached byte buffers)
- **Metadata**: `get_audio_duration(path)` (reads the file with lofty on every call)

## Known limitations / TODO

- **No volume, looping, seeking, or playback-speed controls** — none of rodio's `Sink` controls beyond play/pause/stop are exposed. Background music on loop is currently impossible without re-triggering.
- **No Lua bindings and no runtime hookup**: game scripts can't trigger sounds, and the runtime never plays entity sounds automatically. The only in-engine playback today is the editor inspector's preview button. `load_scene_sounds`/`play_sound` are effectively test-only.
- **Full buffer clone per play**: the cache stores encoded file bytes; every play clones the whole buffer and decodes from scratch. No streaming — large files live entirely in memory.
- **Status tri-state quirk**: a finished sound is not `is_playing`, but `is_stopped` stays `false` until `update()` reaps the sink; `is_stopped` also returns `true` for IDs that never existed.
- **`duration_cache` is dead weight**: populated by `load_sound` but never read; `get_audio_duration` re-probes the file every time.
- Errors are `String`s; no typed error distinction between "no device", "bad file", and "unknown id".
