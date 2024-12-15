use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::{BufReader, Read};
use std::collections::HashMap;
use uuid::Uuid;
use crate::ecs::{Scene, Resource, ResourceType};

pub struct AudioEngine {
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    active_sounds: HashMap<Uuid, Sink>,
    scene_sound_cache: HashMap<Uuid, HashMap<Uuid, Vec<u8>>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        AudioEngine {
            stream,
            stream_handle,
            active_sounds: HashMap::new(),
            scene_sound_cache: HashMap::new(),
        }
    }

    // === Loading Operations ===
    
    // Single file loading
    pub fn load_sound(&self, path: &str) -> Result<Vec<u8>, String> {
        let file = File::open(path)
            .map_err(|e| format!("Failed to open sound file {}: {}", path, e))?;
        
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read sound file {}: {}", path, e))?;
        
        Ok(data)
    }

    // Batch loading
    pub fn load_scene_sounds(&mut self, scene: &Scene) -> Result<(), String> {
        let mut scene_cache = HashMap::new();
        
        let sound_resources: Vec<(&Uuid, &Resource)> = scene.resources.iter()
            .filter(|(_, resource)| resource.resource_type == ResourceType::Sound)
            .collect();

        for (resource_id, resource) in sound_resources {
            let data = self.load_sound(&resource.file_path)?;
            scene_cache.insert(*resource_id, data);
        }

        self.scene_sound_cache.insert(scene.id, scene_cache);
        Ok(())
    }

    // === Playback Operations ===
    
    // Core playback with optional cleanup
    fn play_sound_data(&mut self, data: &Vec<u8>, cleanup: bool) -> Result<Uuid, String> {
        let cursor = std::io::Cursor::new(data.clone());
        let source = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode sound: {}", e))?;

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create sink: {}", e))?;
        
        sink.append(source);
        
        let id = Uuid::new_v4();
        self.active_sounds.insert(id, sink);

        // If cleanup is true, the sink will be removed once it's empty
        if cleanup {
            self.update();  // Clean up any finished sounds
        }
        
        Ok(id)
    }

    // Play a sound file directly (with cleanup)
    pub fn play_sound(&mut self, path: &str) -> Result<Uuid, String> {
        let data = self.load_sound(path)?;
        self.play_sound_data(&data, true)  // Set cleanup to true
    }

    // Play a cached scene sound (no cleanup needed)
    pub fn play_scene_sound(&mut self, scene_id: Uuid, resource_id: Uuid) -> Result<Uuid, String> {
        let data = self.scene_sound_cache
            .get(&scene_id)
            .ok_or("Scene sounds not loaded")?
            .get(&resource_id)
            .ok_or("Sound resource not found in scene cache")?
            .clone();
        
        self.play_sound_data(&data, false)  // No cleanup for cached sounds
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
}

// Scene integration
impl Scene {
    pub fn play_sound(&self, resource_id: Uuid, audio_engine: &mut AudioEngine) -> Result<Uuid, String> {
        let resource = self.resources.get(&resource_id)
            .ok_or("Resource not found")?;
            
        if resource.resource_type != ResourceType::Sound {
            return Err("Resource is not a sound".to_string());
        }

        audio_engine.play_sound(&resource.file_path)
    }
}

// Resource trait implementation
impl Resource {
    pub fn play_as_resource(&self, audio_engine: &mut AudioEngine) -> Result<Uuid, String> {
        match self.resource_type {
            ResourceType::Sound => audio_engine.play_sound(&self.file_path),
            _ => Err("Resource is not a sound".to_string()),
        }
    }
}
