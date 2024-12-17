use rust_2d_game_engine::{
    ecs::{SceneManager, Entity, Scene},
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
    audio_engine::AudioEngine,
    game_runtime::{GameRuntime, RuntimeState},
    input_handler::{InputHandler, InputContext},
};
use egui::Key;
use uuid::Uuid;

pub struct FlappyBird {
    bird_id: Option<Uuid>,
    jump_force: f32,
    gravity: f32,
    original_scene: Option<Scene>,  // Store the original scene data
}

impl FlappyBird {
    pub fn new() -> Self {
        Self {
            bird_id: None,
            jump_force: -300.0,
            gravity: 800.0,
            original_scene: None,
        }
    }

    pub fn init(&mut self, scene_manager: &mut SceneManager) {
        // Store the original scene data
        if let Some(scene) = scene_manager.get_active_scene() {
            self.original_scene = Some(scene.clone());
            
            // Find the bird entity in the runtime scene
            for (id, entity) in &scene.entities {
                if entity.name == "bird" {
                    self.bird_id = Some(*id);
                    println!("Found bird entity with ID: {}", id);
                    break;
                }
            }
        }
    }

    pub fn reset(&mut self, scene_manager: &mut SceneManager) {
        // Reset to original scene state
        if let Some(original) = &self.original_scene {
            if let Some(scene_id) = scene_manager.active_scene {
                scene_manager.scenes.insert(scene_id, original.clone());
                
                // Re-find the bird ID in the new scene instance
                for (id, entity) in &original.entities {
                    if entity.name == "bird" {
                        self.bird_id = Some(*id);
                        break;
                    }
                }
            }
        }
    }

    pub fn update(&mut self, scene_manager: &mut SceneManager, input: &InputHandler, delta_time: f32) {
        if let Some(bird_id) = self.bird_id {
            if let Some(scene) = scene_manager.get_active_scene_mut() {
                if let Ok(bird) = scene.get_entity_mut(bird_id) {
                    // Apply gravity
                    let mut velocity_y = 0.0;  // Current vertical velocity
                    velocity_y += self.gravity * delta_time;

                    // Handle jump with SPACE key
                    if input.is_key_pressed(Key::Space) {
                        println!("SPACE pressed - Bird jumping!");  // Debug print
                        velocity_y = self.jump_force;
                    }

                    // Update position
                    let current_y = bird.get_y();
                    bird.set_y(current_y + velocity_y * delta_time).unwrap();

                    // Debug print position
                    println!("Bird position: ({}, {}), velocity_y: {}", 
                        bird.get_x(), bird.get_y(), velocity_y);
                }
            }
        }
    }
}

fn main() {
    // Create engine components
    let scene_manager = SceneManager::new();
    let physics_engine = PhysicsEngine::new();
    let render_engine = RenderEngine::new();
    let input_handler = InputHandler::new();
    let audio_engine = AudioEngine::new();

    // Create game runtime with our game
    let mut game_runtime = GameRuntime::new(
        scene_manager,
        physics_engine,
        render_engine,
        input_handler,
        audio_engine,
        60
    );

    // Create and initialize our game
    let mut game = FlappyBird::new();
    game.init(game_runtime.get_scene_manager());

    // Start the game loop
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Flappy Bird",
        options,
        Box::new(|cc| Box::new(game_runtime)),
    ).unwrap();
}

