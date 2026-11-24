# Audio (`src/audio/`)

A thin wrapper around [rodio](https://crates.io/crates/rodio). Sound files are
read into memory once and cached by path; every playback decodes the cached
bytes into its own rodio `Player`, identified by a **play id**.

The engine works without an audio device (headless machines, CI): construction
never fails, a warning is logged once, `has_output()` returns `false`, and
playback returns an error instead of decoding anything.

## API

- **Playback**: `play_sound(path) -> play_id`, `play_sound_immediate(path)` /
  `stop_immediate()` (a single "preview" slot used by the editor; starting a
  new preview stops the previous one)
- **Control**: `stop(play_id)`, `pause(play_id)`, `resume(play_id)`,
  `pause_all()` / `resume_all()` (used when the game is paused), `stop_all()`
- **Status**: `is_playing`, `is_paused`, `is_stopped`, `list_playing_sounds`
- **Maintenance**: `update()` drops finished players (the runtime calls it
  every frame); `cleanup()` stops everything and clears the caches;
  `clear_cache()`, `unload_sound(path)`, `get_memory_usage()`
- **Metadata**: `get_audio_duration(path)` (reads the file's header with
  [lofty](https://crates.io/crates/lofty))

A sound that can't be played (missing file, unsupported format) is reported in
the console once per file; the Lua `play_sound` binding then returns nil.

## Interactions

- **`runtime`** owns the game's instance: `update()` each frame, `pause_all` /
  `resume_all` with the game, `stop_all` on game over, `cleanup()` on reset.
- **`scripting`** exposes `play_sound`, `stop_sound`, `is_sound_playing` and
  `stop_all_sounds` (see [scripting.md](scripting.md)).
- **Editor**: the inspector previews sound files with its own instance.

## Known limitations

- No volume, looping, seeking or speed controls; background music can only
  be looped by re-triggering it from a script.
- Every playback clones and decodes the whole cached file; nothing is
  streamed.
- A finished sound counts as stopped only after the next `update()`;
  `is_stopped` is also true for ids that never existed.
