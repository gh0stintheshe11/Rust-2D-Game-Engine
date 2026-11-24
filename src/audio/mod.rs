//! Sound playback with [rodio](https://crates.io/crates/rodio).
//!
//! Sound files are read once and kept in memory, still encoded and shared by
//! all their playbacks; each playback decodes the file while it plays, in its
//! own rodio `Player`, identified by a play id.

use crate::logger::LOGGER;
use lofty::file::AudioFile;
use lofty::probe::Probe;
use rodio::decoder::DecoderError;
use rodio::mixer::{Mixer, MixerSource};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// How [`AudioEngine::play_sound_with`] plays a sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayOptions {
    /// Loudness factor: 1.0 plays the file as recorded, 0.5 at half the
    /// amplitude, 0.0 silently. Values above 1.0 amplify, and may clip.
    pub volume: f32,
    /// Start over at the end, until the sound is stopped.
    pub looping: bool,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            looping: false,
        }
    }
}

/// Validate a volume factor: a finite number, at least 0.
pub(crate) fn check_volume(volume: f32) -> Result<f32, String> {
    if volume.is_finite() && volume >= 0.0 {
        Ok(volume)
    } else {
        Err(format!("volume must be a number >= 0, got {}", volume))
    }
}

/// Audio rendered by an [`AudioEngine::offline`] engine: an endless stream of
/// interleaved samples (1.0 is full scale), silent while nothing plays.
pub struct OfflineOutput(MixerSource);

impl Iterator for OfflineOutput {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        // The mixer itself ends whenever it has nothing to play
        Some(self.0.next().unwrap_or(0.0))
    }
}

/// One playback.
struct ActiveSound {
    player: Player,
    /// The sound's own volume; it plays at `volume * master_volume`.
    volume: f32,
}

/// Plays sounds through the default audio device (or an offline output) and
/// keeps track of every playback.
pub struct AudioEngine {
    // Keeps the audio device open. None without a device (headless machines,
    // CI) and for offline engines.
    _device: Option<MixerDeviceSink>,
    // Where playbacks are mixed. None when there is no output at all: the
    // engine then runs with playback disabled instead of crashing.
    mixer: Option<Mixer>,
    active_sounds: HashMap<Uuid, ActiveSound>,
    // Encoded file contents by path hash, shared by all playbacks
    sound_cache: HashMap<Uuid, Arc<[u8]>>,
    immediate_sink: Option<Player>,
    // Files whose playback failed and was already reported
    reported_failures: HashSet<PathBuf>,
    master_volume: f32,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    /// An engine playing through the default audio device, or with playback
    /// disabled (and a warning logged once) when there is none.
    pub fn new() -> Self {
        match DeviceSinkBuilder::open_default_sink() {
            Ok(device) => {
                let mixer = device.mixer().clone();
                Self::with_output(Some(device), Some(mixer))
            }
            Err(e) => {
                LOGGER.warning(format!(
                    "No audio output device available ({}). Audio playback is disabled.",
                    e
                ));
                Self::with_output(None, None)
            }
        }
    }

    /// An engine without an audio device: everything it plays is mixed into
    /// the returned [`OfflineOutput`], and advances only as samples are read
    /// from it. Makes playback testable on machines without audio, and can
    /// render audio to a file.
    pub fn offline(sample_rate: NonZero<u32>, channels: NonZero<u16>) -> (Self, OfflineOutput) {
        let (mixer, source) = rodio::mixer::mixer(channels, sample_rate);
        (Self::with_output(None, Some(mixer)), OfflineOutput(source))
    }

    fn with_output(device: Option<MixerDeviceSink>, mixer: Option<Mixer>) -> Self {
        Self {
            _device: device,
            mixer,
            active_sounds: HashMap::new(),
            sound_cache: HashMap::new(),
            immediate_sink: None,
            reported_failures: HashSet::new(),
            master_volume: 1.0,
        }
    }

    /// Whether sounds can be played: there is an audio device (or the engine
    /// is offline).
    pub fn has_output(&self) -> bool {
        self.mixer.is_some()
    }

    fn new_player(&self) -> Result<Player, String> {
        self.mixer
            .as_ref()
            .map(Player::connect_new)
            .ok_or_else(|| "No audio output device available".to_string())
    }

    /// Deterministic cache key for a sound path.
    fn path_to_uuid(path: &Path) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_URL, path.to_string_lossy().as_bytes())
    }

    /// The file's encoded contents, read from disk on first use.
    fn load_sound(&mut self, path: &Path) -> Result<Arc<[u8]>, String> {
        let sound_id = Self::path_to_uuid(path);
        if let Some(data) = self.sound_cache.get(&sound_id) {
            return Ok(Arc::clone(data));
        }
        let data: Arc<[u8]> = std::fs::read(path)
            .map_err(|e| format!("Failed to read sound file {}: {}", path.display(), e))?
            .into();
        self.sound_cache.insert(sound_id, Arc::clone(&data));
        Ok(data)
    }

    // === Playback ===

    /// Play a sound file once at full volume; returns its play id. See
    /// [`play_sound_with`](Self::play_sound_with).
    pub fn play_sound(&mut self, path: &Path) -> Result<Uuid, String> {
        self.play_sound_with(path, PlayOptions::default())
    }

    /// Start playing a sound file; returns its play id. An invalid volume is
    /// an error. Failures to play the file are also reported in the console,
    /// once per file (a missing output device is reported once at startup
    /// instead).
    pub fn play_sound_with(&mut self, path: &Path, options: PlayOptions) -> Result<Uuid, String> {
        check_volume(options.volume)?;
        if !self.has_output() {
            return Err("No audio output device available".to_string());
        }
        let result = self.start_playback(path, options);
        if let Err(e) = &result
            && self.reported_failures.insert(path.to_path_buf())
        {
            LOGGER.warning(format!("Can't play {}: {}", path.display(), e));
        }
        result
    }

    fn start_playback(&mut self, path: &Path, options: PlayOptions) -> Result<Uuid, String> {
        let data = self.load_sound(path)?;
        let player = self.new_player()?;
        // Set before the sound is queued, so it applies from the first sample
        player.set_volume(options.volume * self.master_volume);
        queue_sound(&player, data, options.looping)?;

        let play_id = Uuid::new_v4();
        self.active_sounds.insert(
            play_id,
            ActiveSound {
                player,
                volume: options.volume,
            },
        );
        Ok(play_id)
    }

    /// Preview a sound file in a single slot: starting a preview stops the
    /// previous one. Used by the editor.
    pub fn play_sound_immediate(&mut self, path: &Path) -> Result<(), String> {
        self.stop_immediate();
        let data = self.load_sound(path)?;
        let player = self.new_player()?;
        player.set_volume(self.master_volume);
        queue_sound(&player, data, false)?;
        self.immediate_sink = Some(player);
        Ok(())
    }

    pub fn stop_immediate(&mut self) {
        if let Some(player) = self.immediate_sink.take() {
            player.stop();
        }
    }

    // === Control ===

    pub fn stop(&mut self, play_id: Uuid) -> Result<(), String> {
        let sound = self
            .active_sounds
            .remove(&play_id)
            .ok_or("Sound not found")?;
        sound.player.stop();
        Ok(())
    }

    pub fn pause(&mut self, play_id: Uuid) -> Result<(), String> {
        self.sound(play_id)?.player.pause();
        Ok(())
    }

    pub fn resume(&mut self, play_id: Uuid) -> Result<(), String> {
        self.sound(play_id)?.player.play();
        Ok(())
    }

    /// Change a playback's volume (see [`PlayOptions::volume`]).
    pub fn set_volume(&mut self, play_id: Uuid, volume: f32) -> Result<(), String> {
        let volume = check_volume(volume)?;
        let master_volume = self.master_volume;
        let sound = self
            .active_sounds
            .get_mut(&play_id)
            .ok_or("Sound not found")?;
        sound.volume = volume;
        sound.player.set_volume(volume * master_volume);
        Ok(())
    }

    /// A playback's own volume, without the master volume.
    pub fn volume(&self, play_id: Uuid) -> Option<f32> {
        self.active_sounds.get(&play_id).map(|sound| sound.volume)
    }

    /// Scale every sound, playing or started later, by `volume` (see
    /// [`PlayOptions::volume`]). [`cleanup`](Self::cleanup) restores 1.0.
    pub fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        self.master_volume = check_volume(volume)?;
        for sound in self.active_sounds.values() {
            sound.player.set_volume(sound.volume * self.master_volume);
        }
        if let Some(player) = &self.immediate_sink {
            player.set_volume(self.master_volume);
        }
        Ok(())
    }

    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    fn sound(&self, play_id: Uuid) -> Result<&ActiveSound, String> {
        self.active_sounds
            .get(&play_id)
            .ok_or_else(|| "Sound not found".to_string())
    }

    // === Status ===

    pub fn is_playing(&self, play_id: Uuid) -> bool {
        self.active_sounds
            .get(&play_id)
            .is_some_and(|sound| !sound.player.empty() && !sound.player.is_paused())
    }

    pub fn is_paused(&self, play_id: Uuid) -> bool {
        self.active_sounds
            .get(&play_id)
            .is_some_and(|sound| sound.player.is_paused())
    }

    pub fn is_stopped(&self, play_id: Uuid) -> bool {
        !self.active_sounds.contains_key(&play_id)
    }

    pub fn list_playing_sounds(&self) -> Vec<Uuid> {
        self.active_sounds
            .iter()
            .filter(|(_, sound)| !sound.player.empty() && !sound.player.is_paused())
            .map(|(id, _)| *id)
            .collect()
    }

    // === Maintenance ===

    /// Drop finished playbacks (looping sounds never finish). The runtime
    /// calls this every frame.
    pub fn update(&mut self) {
        self.active_sounds.retain(|_, sound| !sound.player.empty());
    }

    pub fn stop_all(&mut self) {
        for (_, sound) in self.active_sounds.drain() {
            sound.player.stop();
        }
    }

    /// Pause every playing sound (e.g. while the game is paused).
    pub fn pause_all(&mut self) {
        for sound in self.active_sounds.values() {
            sound.player.pause();
        }
    }

    /// Resume every paused sound.
    pub fn resume_all(&mut self) {
        for sound in self.active_sounds.values() {
            sound.player.play();
        }
    }

    /// Stop everything, forget cached sound data and reported failures, and
    /// restore the master volume.
    pub fn cleanup(&mut self) {
        self.stop_all();
        self.stop_immediate();
        self.clear_cache();
        self.master_volume = 1.0;
    }

    /// Forget cached sound data; files are read again on next play.
    pub fn clear_cache(&mut self) {
        self.sound_cache.clear();
        self.reported_failures.clear();
    }

    /// Forget one file's cached data, e.g. after it changed on disk.
    pub fn unload_sound(&mut self, path: &Path) {
        self.sound_cache.remove(&Self::path_to_uuid(path));
        self.reported_failures.remove(path);
    }

    /// Bytes of sound file data held in the cache.
    pub fn get_memory_usage(&self) -> usize {
        self.sound_cache.values().map(|data| data.len()).sum()
    }

    // === Metadata ===

    pub fn get_audio_duration(&self, path: &Path) -> Result<f32, String> {
        let tagged_file = Probe::open(path)
            .map_err(|e| format!("Failed to open audio file: {:?}: {}", path, e))?
            .read()
            .map_err(|e| format!("Failed to read audio file: {:?}: {}", path, e))?;

        let properties = tagged_file.properties();
        let duration = properties.duration();

        Ok(duration.as_secs_f32())
    }
}

/// Queue an encoded sound file on `player`; it is decoded while it plays.
/// A looping sound seeks back to the start at its end instead of keeping
/// decoded samples around.
fn queue_sound(player: &Player, data: Arc<[u8]>, looping: bool) -> Result<(), String> {
    let decode_error = |e: DecoderError| format!("Failed to decode sound: {}", e);
    let reader = Cursor::new(data);
    if looping {
        player.append(Decoder::new_looped(reader).map_err(decode_error)?);
    } else {
        player.append(Decoder::new(reader).map_err(decode_error)?);
    }
    Ok(())
}
