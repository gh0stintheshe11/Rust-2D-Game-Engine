use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::{BufReader, Read};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use crate::ecs::{Scene, Entity};
use lofty::{Probe, AudioFile};

pub struct AudioEngine {
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    active_sounds: HashMap<Uuid, Sink>,
    sound_cache: HashMap<Uuid, Vec<u8>>,  // Path hash -> sound data
    immediate_sink: Option<Sink>,
    duration_cache: HashMap<Uuid, f32>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        AudioEngine {
            stream,
            stream_handle,
            active_sounds: HashMap::new(),
            sound_cache: HashMap::new(),
            immediate_sink: None,
            duration_cache: HashMap::new(),
        }
    }

    // Generate deterministic UUID from path
    fn path_to_uuid(path: &Path) -> Uuid {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let result = hasher.finalize();
        
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&result[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        
        Uuid::from_bytes(bytes)
    }

    // === Loading Operations ===
    
    fn load_sound(&mut self, path: &Path) -> Result<Uuid, String> {
        let sound_id = Self::path_to_uuid(path);
        
        if self.sound_cache.contains_key(&sound_id) {
            return Ok(sound_id);
        }

        let file = File::open(path)
            .map_err(|e| format!("Failed to open sound file {:?}: {}", path, e))?;
        
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read sound file {:?}: {}", path, e))?;
        
        self.sound_cache.insert(sound_id, data);
        
        if let Ok(duration) = self.get_audio_duration(path) {
            self.duration_cache.insert(sound_id, duration);
        }

        Ok(sound_id)
    }

    // Load sounds for an entity
    pub fn load_entity_sounds(&mut self, entity: &Entity) -> Result<(), String> {
        for path in &entity.sounds {
            self.load_sound(path)?;
        }
        Ok(())
    }

    // Load sounds for a scene
    pub fn load_scene_sounds(&mut self, scene: &Scene) -> Result<(), String> {
        for (_, entity) in &scene.entities {
            self.load_entity_sounds(entity)?;
        }
        Ok(())
    }

    // === Playback Operations ===
    
    pub fn play_sound(&mut self, path: &Path) -> Result<Uuid, String> {
        let sound_id = self.load_sound(path)?;
        let data = self.sound_cache.get(&sound_id)
            .ok_or("Sound not found in cache")?;
        
        let cursor = std::io::Cursor::new(data.clone());
        let source = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode sound: {}", e))?;

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create sink: {}", e))?;
        
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
        let data = self.sound_cache.get(&sound_id)
            .ok_or("Sound not found in cache")?;

        let cursor = std::io::Cursor::new(data.clone());
        let source = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode sound: {}", e))?;

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create sink: {}", e))?;
        
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
        self.active_sounds.get(&sound_id)
            .map_or(false, |sink| !sink.empty() && !sink.is_paused())
    }

    pub fn is_paused(&self, sound_id: Uuid) -> bool {
        self.active_sounds.get(&sound_id)
            .map_or(false, |sink| sink.is_paused())
    }

    pub fn is_stopped(&self, sound_id: Uuid) -> bool {
        !self.active_sounds.contains_key(&sound_id)
    }

    pub fn list_playing_sounds(&self) -> Vec<Uuid> {
        self.active_sounds.iter()
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

    // === Memory Management ===
    
    pub fn cleanup(&mut self) {
        self.stop_all();
        self.stop_immediate();
        self.sound_cache.clear();
        self.duration_cache.clear();
    }

    pub fn clear_cache(&mut self) {
        self.sound_cache.clear();
        self.duration_cache.clear();
    }

    pub fn unload_sound(&mut self, path: &Path) {
        let sound_id = Self::path_to_uuid(path);
        self.sound_cache.remove(&sound_id);
        self.duration_cache.remove(&sound_id);
    }

    pub fn get_memory_usage(&self) -> usize {
        self.sound_cache.values()
            .map(|data| data.len())
            .sum()
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
