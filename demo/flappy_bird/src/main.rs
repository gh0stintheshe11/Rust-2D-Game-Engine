use rust_2d_game_engine::{
    EngineGui,
    eframe,
    game_runtime::{Game, GameRuntime},
    ecs::SceneManager,
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
    input_handler::InputHandler,
    audio_engine::AudioEngine,
};
use egui::Key;
use uuid::Uuid;

pub struct FlappyBird {
    bird_id: Option<Uuid>,
    jump_force: f32,
    gravity: f32,
    velocity_y: f32,
    original_scene: Option<Scene>,
}

impl FlappyBird {
    pub fn new() -> Self {
        Self {
            bird_id: None,
            jump_force: -300.0,
            gravity: 800.0,
            velocity_y: 0.0,
            original_scene: None,
        }
    }
}

impl Game for FlappyBird {
    fn init(&mut self, scene_manager: &mut SceneManager) {
        println!("FlappyBird init called!");
        if let Some(scene) = scene_manager.get_active_scene() {
            println!("Found active scene: {}", scene.name);
            self.original_scene = Some(scene.clone());
            
            println!("Searching for bird entity...");
            for (id, entity) in &scene.entities {
                println!("Checking entity: {} ({})", entity.name, id);
                if entity.name == "bird" {
                    self.bird_id = Some(*id);
                    println!("Found bird entity with ID: {}", id);
                    break;
                }
            }
        } else {
            println!("No active scene found in init!");
        }
    }

    fn update(&mut self, scene_manager: &mut SceneManager, input: &InputHandler, delta_time: f32) {
        println!("FlappyBird update called with delta_time: {}", delta_time);
        
        if let Some(bird_id) = self.bird_id {
            println!("Have bird_id: {}", bird_id);
            if let Some(scene) = scene_manager.get_active_scene_mut() {
                println!("Found active scene: {}", scene.name);
                if let Ok(bird) = scene.get_entity_mut(bird_id) {
                    // Apply gravity to stored velocity
                    self.velocity_y += self.gravity * delta_time;

                    // Jump when space is pressed
                    if input.is_key_just_pressed(Key::Space) {
                        println!("SPACE pressed - Bird jumping! Current velocity: {}", self.velocity_y);
                        self.velocity_y = self.jump_force;
                    }

                    // Update position using stored velocity
                    let current_y = bird.get_y();
                    let new_y = current_y + self.velocity_y * delta_time;
                    println!("Updating bird position: {} -> {} (vel: {})", 
                        current_y, new_y, self.velocity_y);
                    bird.set_y(new_y).unwrap();
                } else {
                    println!("Failed to get bird entity!");
                }
            } else {
                println!("No active scene found in update!");
            }
        } else {
            println!("No bird_id set!");
        }
    }

    fn reset(&mut self, scene_manager: &mut SceneManager) {
        println!("FlappyBird reset called!");
        if let Some(original) = &self.original_scene {
            if let Some(scene_id) = scene_manager.active_scene {
                scene_manager.scenes.insert(scene_id, original.clone());
                
                for (id, entity) in &original.entities {
                    if entity.name == "bird" {
                        self.bird_id = Some(*id);
                        println!("Reset: Found bird entity with ID: {}", id);
                        break;
                    }
                }
            }
        }
        // Reset velocity
        self.velocity_y = 0.0;
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Flappy Bird",
        options,
        Box::new(|cc| {
            // Create the engine GUI first
            let mut engine = EngineGui::new(cc);
            
            // Create and attach the game
            let game = Box::new(FlappyBird::new());
            engine.get_game_runtime_mut().set_game(game);
            
            Box::new(engine)
        }),
    ).unwrap();
}

