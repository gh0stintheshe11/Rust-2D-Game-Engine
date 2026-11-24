use crate::logger::LOGGER;
use lofty::file::AudioFile;
use lofty::probe::Probe;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Sound playback with rodio. Sound files are read once and kept in memory;
/// each playback gets its own player, identified by a play id.
pub struct AudioEngine {
    // None when no audio output device is available (e.g. headless/CI);
    // the engine then runs with playback disabled instead of crashing.
    output: Option<MixerDeviceSink>,
    active_sounds: HashMap<Uuid, Player>,
    sound_cache: HashMap<Uuid, Vec<u8>>, // Path hash -> sound data
    immediate_sink: Option<Player>,
    // Files whose playback failed and was already reported
    reported_failures: HashSet<PathBuf>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        let output = match DeviceSinkBuilder::open_default_sink() {
            Ok(output) => Some(output),
            Err(e) => {
                LOGGER.warning(format!(
                    "No audio output device available ({}). Audio playback is disabled.",
                    e
                ));
                None
            }
        };
        AudioEngine {
            output,
            active_sounds: HashMap::new(),
            sound_cache: HashMap::new(),
            immediate_sink: None,
            reported_failures: HashSet::new(),
        }
    }

    /// Whether an audio output device is available for playback.
    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }

    fn new_player(&self) -> Result<Player, String> {
        self.output
            .as_ref()
            .map(|sink| Player::connect_new(sink.mixer()))
            .ok_or_else(|| "No audio output device available".to_string())
    }

    /// Deterministic cache key for a sound path.
    fn path_to_uuid(path: &Path) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_URL, path.to_string_lossy().as_bytes())
    }

    // === Loading Operations ===

    fn load_sound(&mut self, path: &Path) -> Result<Uuid, String> {
        let sound_id = Self::path_to_uuid(path);

        if self.sound_cache.contains_key(&sound_id) {
            return Ok(sound_id);
        }

        let file =
            File::open(path).map_err(|e| format!("Failed to open sound file {:?}: {}", path, e))?;

        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .map_err(|e| format!("Failed to read sound file {:?}: {}", path, e))?;

        self.sound_cache.insert(sound_id, data);
        Ok(sound_id)
    }

    // === Playback Operations ===

    /// Start playing a sound file; returns its play id. Failures are also
    /// reported in the console, once per file (the missing output device is
    /// reported once at startup instead).
    pub fn play_sound(&mut self, path: &Path) -> Result<Uuid, String> {
        if !self.has_output() {
            return Err("No audio output device available".to_string());
        }
        let result = self.start_playback(path);
        if let Err(e) = &result
            && self.reported_failures.insert(path.to_path_buf())
        {
            LOGGER.warning(format!("Can't play {}: {}", path.display(), e));
        }
        result
    }

    fn start_playback(&mut self, path: &Path) -> Result<Uuid, String> {
        let sound_id = self.load_sound(path)?;
        let data = self
            .sound_cache
            .get(&sound_id)
            .ok_or("Sound not found in cache")?;

        let cursor = std::io::Cursor::new(data.clone());
        let source = Decoder::new(cursor).map_err(|e| format!("Failed to decode sound: {}", e))?;

        let sink = self.new_player()?;

        sink.append(source);

        let play_id = Uuid::new_v4();
        self.active_sounds.insert(play_id, sink);

        Ok(play_id)
    }

    // Play a sound file immediately
    pub fn play_sound_immediate(&mut self, path: &Path) -> Result<(), String> {
        if let Some(sink) = &self.immediate_sink {
            sink.stop();
        }

        let sound_id = self.load_sound(path)?;
        let data = self
            .sound_cache
            .get(&sound_id)
            .ok_or("Sound not found in cache")?;

        let cursor = std::io::Cursor::new(data.clone());
        let source = Decoder::new(cursor).map_err(|e| format!("Failed to decode sound: {}", e))?;

        let sink = self.new_player()?;

        sink.append(source);
        self.immediate_sink = Some(sink);

        Ok(())
    }

    pub fn stop_immediate(&mut self) {
        if let Some(sink) = &self.immediate_sink {
            sink.stop();
        }
        self.immediate_sink = None;
    }

    // === Control Operations ===

    pub fn stop(&mut self, sound_id: Uuid) -> Result<(), String> {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            sink.stop();
            self.active_sounds.remove(&sound_id);
            Ok(())
        } else {
            Err("Sound not found".to_string())
        }
    }

    pub fn pause(&mut self, sound_id: Uuid) -> Result<(), String> {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            sink.pause();
            Ok(())
        } else {
            Err("Sound not found".to_string())
        }
    }

    pub fn resume(&mut self, sound_id: Uuid) -> Result<(), String> {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            sink.play();
            Ok(())
        } else {
            Err("Sound not found".to_string())
        }
    }

    // === Status Operations ===

    pub fn is_playing(&self, sound_id: Uuid) -> bool {
        self.active_sounds
            .get(&sound_id)
            .is_some_and(|sink| !sink.empty() && !sink.is_paused())
    }

    pub fn is_paused(&self, sound_id: Uuid) -> bool {
        self.active_sounds
            .get(&sound_id)
            .is_some_and(|sink| sink.is_paused())
    }

    pub fn is_stopped(&self, sound_id: Uuid) -> bool {
        !self.active_sounds.contains_key(&sound_id)
    }

    pub fn list_playing_sounds(&self) -> Vec<Uuid> {
        self.active_sounds
            .iter()
            .filter(|(_, sink)| !sink.empty() && !sink.is_paused())
            .map(|(id, _)| *id)
            .collect()
    }

    // === Maintenance Operations ===

    pub fn update(&mut self) {
        self.active_sounds.retain(|_, sink| !sink.empty());
    }

    pub fn stop_all(&mut self) {
        for (_, sink) in self.active_sounds.drain() {
            sink.stop();
        }
    }

    /// Pause every playing sound (e.g. while the game is paused).
    pub fn pause_all(&mut self) {
        for sink in self.active_sounds.values() {
            sink.pause();
        }
    }

    /// Resume every sound paused by [`pause_all`](Self::pause_all).
    pub fn resume_all(&mut self) {
        for sink in self.active_sounds.values() {
            sink.play();
        }
    }

    // === Memory Management ===

    /// Stop everything and forget cached sound data and reported failures.
    pub fn cleanup(&mut self) {
        self.stop_all();
        self.stop_immediate();
        self.clear_cache();
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

    pub fn get_memory_usage(&self) -> usize {
        self.sound_cache.values().map(|data| data.len()).sum()
    }

    // === Metadata Operations ===
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
