use super::{Entity, Scene};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============== Scene Manager (Top Level) ===============
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SceneManager {
    pub scenes: IndexMap<Uuid, Scene>,
    pub shared_entities: IndexMap<Uuid, Entity>,
    pub active_scene: Option<Uuid>, // Track currently active scene
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scenes: IndexMap::new(),
            shared_entities: IndexMap::new(),
            active_scene: None,
        }
    }

    pub fn create_scene(&mut self, name: &str) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let scene = Scene::new(name)?;
        self.scenes.insert(id, scene);
        Ok(id)
    }

    pub fn delete_scene(&mut self, id: Uuid) -> Result<bool, String> {
        if self.active_scene == Some(id) {
            return Err("Cannot delete active scene".to_string());
        }
        Ok(self.scenes.shift_remove(&id).is_some())
    }

    pub fn list_scene(&self) -> Vec<(Uuid, &str)> {
        self.scenes
            .iter()
            .map(|(id, scene)| (*id, scene.name.as_str()))
            .collect()
    }

    pub fn get_scene(&self, id: Uuid) -> Option<&Scene> {
        self.scenes.get(&id)
    }

    pub fn get_scene_mut(&mut self, id: Uuid) -> Option<&mut Scene> {
        self.scenes.get_mut(&id)
    }

    pub fn get_scene_by_name(&self, name: &str) -> Option<&Scene> {
        self.scenes
            .iter()
            .find(|(_, scene)| scene.name == name)
            .map(|(_, scene)| scene)
    }

    pub fn create_shared_entity(&mut self, name: &str) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let entity = Entity::new(id, name)?;
        self.shared_entities.insert(id, entity);
        Ok(id)
    }

    pub fn delete_shared_entity(&mut self, id: Uuid) -> Result<bool, String> {
        for scene in self.scenes.values() {
            if scene.shared_entity_refs.contains(&id) {
                return Err("Entity is still referenced by a scene".to_string());
            }
        }
        Ok(self.shared_entities.shift_remove(&id).is_some())
    }

    pub fn list_shared_entity(&self) -> Vec<(Uuid, &str)> {
        self.shared_entities
            .iter()
            .map(|(id, entity)| (*id, entity.name.as_str()))
            .collect()
    }

    pub fn get_shared_entity(&self, id: Uuid) -> Option<&Entity> {
        self.shared_entities.get(&id)
    }

    pub fn get_shared_entity_mut(&mut self, id: Uuid) -> Option<&mut Entity> {
        self.shared_entities.get_mut(&id)
    }

    pub fn get_shared_entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.shared_entities
            .iter()
            .find(|(_, entity)| entity.name == name)
            .map(|(_, entity)| entity)
    }

    // Helper to get all scenes using a shared entity
    pub fn get_scenes_using_entity(&self, entity_id: Uuid) -> Vec<&Scene> {
        self.scenes
            .values()
            .filter(|scene| scene.shared_entity_refs.contains(&entity_id))
            .collect()
    }

    // Add these methods for active scene management
    pub fn set_active_scene(&mut self, id: Uuid) -> Result<(), String> {
        if self.scenes.contains_key(&id) {
            self.active_scene = Some(id);
            Ok(())
        } else {
            Err("Scene not found".to_string())
        }
    }

    pub fn get_active_scene(&self) -> Option<&Scene> {
        self.active_scene.and_then(|id| self.scenes.get(&id))
    }

    pub fn get_active_scene_mut(&mut self) -> Option<&mut Scene> {
        self.active_scene.and_then(|id| self.scenes.get_mut(&id))
    }

    pub fn clear_active_scene(&mut self) {
        self.active_scene = None;
    }
}
