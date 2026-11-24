# Audio (`src/audio/`)

A thin wrapper around [rodio](https://crates.io/crates/rodio). Sound files are
read into memory once, cached by path, and shared by all their playbacks
(still encoded); each playback decodes the file while it plays, in its own
rodio `Player`, identified by a **play id**.

The engine works without an audio device (headless machines, CI): construction
never fails, a warning is logged once, `has_output()` returns `false`, and
playback returns an error instead of decoding anything.

Formats: WAV, Ogg Vorbis, MP3 and FLAC.

## Volume and looping

`play_sound_with(path, PlayOptions { volume, looping })` controls how a sound
plays (`play_sound(path)` plays it once at volume 1.0):

- `volume` is a loudness factor: 1.0 plays the file as recorded, 0.5 at half
  the amplitude, 0.0 silently; values above 1.0 amplify and may clip. It must
  be a finite number ≥ 0. `set_volume(play_id, volume)` changes it while the
  sound plays.
- `looping` starts the sound over at its end until it is stopped. The decoder
  seeks back to the start of the file, so looping keeps no decoded audio in
  memory.
- The **master volume** (`set_master_volume`, default 1.0) scales every sound,
  playing or started later: a sound plays at `volume × master`. `cleanup()`
  restores it to 1.0.

Volume, pause and stop requests take effect within 5 ms (rodio applies them
periodically); a volume set at `play_sound_with` applies from the first
sample.

## API

- **Playback**: `play_sound(path)`, `play_sound_with(path, options)` → play
  id; `play_sound_immediate(path)` / `stop_immediate()` (a single "preview"
  slot used by the editor; starting a new preview stops the previous one)
- **Control**: `stop`, `pause`, `resume`, `set_volume` (by play id),
  `set_master_volume`, `pause_all` / `resume_all` (used when the game is
  paused), `stop_all`
- **Status**: `is_playing`, `is_paused`, `is_stopped`, `list_playing_sounds`,
  `volume(play_id)`, `master_volume()`
- **Maintenance**: `update()` drops finished playbacks (the runtime calls it
  every frame; looping sounds never finish); `cleanup()` stops everything,
  clears the caches and restores the master volume; `clear_cache()`,
  `unload_sound(path)`, `get_memory_usage()`
- **Metadata**: `get_audio_duration(path)` (reads the file's header with
  [lofty](https://crates.io/crates/lofty))
- **Offline output**: `AudioEngine::offline(sample_rate, channels)` returns an
  engine without a device together with an `OfflineOutput`: an endless
  iterator of interleaved samples that everything played is mixed into.
  Playback advances only as samples are read, which makes it testable on
  machines without audio (the tests measure volumes this way) and can render
  audio to a file.

A sound that can't be played (missing file, unsupported format) is reported in
the console once per file; the Lua `play_sound` binding then returns nil.

```rust
let mut audio = AudioEngine::new();
let music = audio.play_sound_with(
    &project.join("assets/sounds/music.ogg"),
    PlayOptions { volume: 0.4, looping: true },
)?;
audio.set_master_volume(0.8)?;
audio.stop(music)?;
```

## Interactions

- **`runtime`** owns the game's instance: `update()` each frame, `pause_all` /
  `resume_all` with the game, `stop_all` on game over, `cleanup()` on reset.
- **`scripting`** exposes `play_sound` (with `volume` and `loop` options),
  `set_sound_volume`, `set_master_volume`, `stop_sound`, `is_sound_playing`
  and `stop_all_sounds` (see [scripting.md](scripting.md)).
- **Editor**: the inspector previews sound files with its own instance.

## Known limitations

- No seeking, speed control, fades or crossfades, and no volume groups (e.g.
  music vs. effects) beyond the master volume.
- No stereo panning or positional audio.
- Sound files stay in memory (encoded) until `cleanup()`; a long music track
  costs its file size.
- `resume_all` resumes every sound, including one that was paused on its own
  before the game was paused.
- A finished sound counts as stopped only after the next `update()`;
  `is_stopped` is also true for ids that never existed.
