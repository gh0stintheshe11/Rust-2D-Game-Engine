# Audio (`src/audio/`)

A thin wrapper around [rodio](https://crates.io/crates/rodio). Sound files are
read into memory once, cached by path, and shared by all their playbacks
(still encoded); each playback decodes the file while it plays, in its own
rodio `Player`, identified by a **play id**.

The engine works without an audio device (headless machines, CI): construction
never fails, a warning is logged once, `has_output()` returns `false`, and
playback returns an error instead of decoding anything.

Formats: WAV, Ogg Vorbis, MP3 and FLAC.

## Volume, fades and groups

`play_sound_with(path, PlayOptions { volume, looping, group, fade_in })`
controls how a sound plays (`play_sound(path)` plays it once at volume 1.0,
in the `"sfx"` group):

- `volume` is a loudness factor: 1.0 plays the file as recorded, 0.5 at half
  the amplitude, 0.0 silently; values above 1.0 amplify and may clip. It must
  be a finite number ≥ 0.
- `looping` starts the sound over at its end until it is stopped. The decoder
  seeks back to the start of the file, so looping keeps no decoded audio in
  memory.
- `group` is the **volume group** it plays in (`DEFAULT_GROUP`, `"sfx"`,
  unless set): a bus such as `"music"` or `"voices"`, named freely (1 to 64
  characters). `set_group_volume(group, volume)` scales every sound of the
  group, playing or started later (a game's music and effects settings);
  groups start at 1.0, and `group_volume(group)` reads one.
- `fade_in` (seconds) starts the sound silent and fades it up to `volume`.
- The **master volume** (`set_master_volume`) scales every sound. A sound
  plays at `volume × group volume × master`; `cleanup()` restores the master
  and group volumes to 1.0.

A playback's own volume fades on the audio thread, sample by sample, so a
fade has no steps whatever the frame rate (`fade.rs`: the game sets where
the volume goes and how fast, a source wrapper follows; it costs a load and
a comparison per sample while the volume holds). `fade(play_id, volume,
seconds)` fades it linearly from where it is, `set_volume(play_id, volume)`
gets there over 5 ms (a jump would click), and `stop_with_fade(play_id,
seconds)` fades it out and stops it once it's silent; it counts as playing
until then, and a sound silent already stops at once. `volume(play_id)` is
where the volume is going.

Group and master volumes, pause and stop take effect within 5 ms (rodio
applies them periodically) and jump.

## Music

One playback at a time is the **music**. `play_music(path, MusicOptions {
volume, crossfade, looping })` plays a file in the `MUSIC_GROUP` (`"music"`),
looping unless told otherwise: the music playing fades out over `crossfade`
seconds (then stops) while the new one fades in from silence; 0 switches at
once. If the music playing is that same file, it plays on, fading to the new
volume instead, so a level that asks for its music every time it starts
doesn't restart it. `stop_music(seconds)` fades the music out and stops it,
and `music()` returns its play id while it plays.

## API

- **Playback**: `play_sound(path)`, `play_sound_with(path, options)` → play
  id; `play_sound_immediate(path)` / `stop_immediate()` (a single "preview"
  slot used by the editor; starting a new preview stops the previous one)
- **Control**: `stop`, `stop_with_fade`, `pause`, `resume`, `set_volume`,
  `fade` (by play id), `set_group_volume`, `set_master_volume`, `pause_all` /
  `resume_all` (used when the game is paused), `stop_all`
- **Music**: `play_music(path, options)`, `stop_music(seconds)`, `music()`
- **Status**: `is_playing`, `is_paused`, `is_stopped`, `list_playing_sounds`,
  `volume(play_id)`, `group_volume(group)`, `master_volume()`
- **Maintenance**: `update()` drops finished playbacks (the runtime calls it
  every frame; looping sounds never finish); `cleanup()` stops everything,
  clears the caches and restores the master and group volumes; `clear_cache()`,
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
let wind = audio.play_sound_with(
    &project.join("assets/sounds/wind.ogg"),
    PlayOptions { volume: 0.4, looping: true, fade_in: 2.0, ..PlayOptions::default() },
)?;
let options = MusicOptions { crossfade: 1.0, ..MusicOptions::default() };
audio.play_music(&project.join("assets/sounds/level.ogg"), options)?;
audio.set_group_volume(MUSIC_GROUP, 0.5)?;
audio.stop_with_fade(wind, 1.5)?;
```

## Interactions

- **`runtime`** owns the game's instance: `update()` each frame, `pause_all` /
  `resume_all` with the game, `stop_all` on game over, `cleanup()` on reset.
- **`scripting`** exposes `play_sound` (with `volume`, `loop`, `group` and
  `fade_in` options), `set_sound_volume`, `fade_sound`, `stop_sound` (with a
  fade), `play_music`, `stop_music`, `set_group_volume`, `get_group_volume`,
  `set_master_volume`, `is_sound_playing` and `stop_all_sounds` (see
  [scripting.md](scripting.md)). Music keeps playing across scene changes.
- **Editor**: the inspector previews sound files with its own instance.

## Known limitations

- No seeking or speed control. Fades are linear; group and master volumes
  change at once (no fades), and a sound belongs to one group.
- No stereo panning or positional audio.
- Sound files stay in memory (encoded) until `cleanup()`; a long music track
  costs its file size.
- `resume_all` resumes every sound, including one that was paused on its own
  before the game was paused.
- A finished sound counts as stopped only after the next `update()`;
  `is_stopped` is also true for ids that never existed.
