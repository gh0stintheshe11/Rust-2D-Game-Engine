use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use uuid::Uuid;

pub struct AudioEngine {
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    // Track active sounds and their sinks
    active_sounds: HashMap<Uuid, Sink>,
    // Cache loaded sounds to avoid reloading
    sound_cache: HashMap<String, Vec<u8>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        
        AudioEngine {
            stream,
            stream_handle,
            active_sounds: HashMap::new(),
            sound_cache: HashMap::new(),
        }
    }

    // Load and cache a sound file
    pub fn load_sound(&mut self, path: &str) -> Result<(), String> {
        if self.sound_cache.contains_key(path) {
            return Ok(());
        }

        let file = File::open(path)
            .map_err(|e| format!("Failed to open sound file: {}", e))?;
        
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read sound file: {}", e))?;
        
        self.sound_cache.insert(path.to_string(), data);
        Ok(())
    }

    // Play a sound and return its UUID for control
    pub fn play_sound(&mut self, path: &str) -> Result<Uuid, String> {
        // Load sound if not cached
        if !self.sound_cache.contains_key(path) {
            self.load_sound(path)?;
        }

        // Get cached sound data
        let data = self.sound_cache.get(path)
            .ok_or("Sound not found in cache")?;
        
        // Create cursor for sound data
        let cursor = std::io::Cursor::new(data.clone());
        let source = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode sound: {}", e))?;

        // Create new sink for this sound
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create sink: {}", e))?;
        
        sink.append(source);
        
        let id = Uuid::new_v4();
        self.active_sounds.insert(id, sink);
        
        Ok(id)
    }

    // Control methods using sound UUID
    pub fn stop_sound(&mut self, sound_id: Uuid) -> Result<(), String> {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            sink.stop();
            self.active_sounds.remove(&sound_id);
            Ok(())
        } else {
            Err("Sound not found".to_string())
        }
    }

    pub fn pause_sound(&mut self, sound_id: Uuid) -> Result<(), String> {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            sink.pause();
            Ok(())
        } else {
            Err("Sound not found".to_string())
        }
    }

    pub fn resume_sound(&mut self, sound_id: Uuid) -> Result<(), String> {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            sink.play();
            Ok(())
        } else {
            Err("Sound not found".to_string())
        }
    }

    pub fn is_playing(&self, sound_id: Uuid) -> bool {
        if let Some(sink) = self.active_sounds.get(&sound_id) {
            !sink.empty() && !sink.is_paused()
        } else {
            false
        }
    }

    // Clean up finished sounds
    pub fn update(&mut self) {
        self.active_sounds.retain(|_, sink| !sink.empty());
    }

    // Stop all sounds
    pub fn stop_all(&mut self) {
        for (_, sink) in self.active_sounds.drain() {
            sink.stop();
        }
    }
}
