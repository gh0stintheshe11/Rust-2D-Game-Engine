use rust_2d_game_engine::{
    EngineGui,
    eframe,
    ecs::{SceneManager, Scene},
    input_handler::{InputHandler, InputContext, Key},
    physics_engine::PhysicsEngine,
    game_runtime::{GameRuntime, RuntimeState},
    project_manager::ProjectManager,
};
use std::{fs, path::Path};
use serde_json::from_str;
use rapier2d::prelude::*;

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Game panicked: {}", panic_info);
    }));

    println!("Starting flappy_bird...");
    
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(eframe::egui::vec2(1920.0, 1080.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Flappy Bird",
        native_options,
        Box::new(|cc| {
            let mut engine = EngineGui::new(cc);
            
            // Load the project using ProjectManager
            let project_path = Path::new("demo/flappy_bird");
            match ProjectManager::load_project_full(project_path) {
                Ok(loaded_project) => {
                    println!("Successfully loaded project");
                    
                    // Replace the engine's scene manager with the loaded one
                    if let Some(scene_manager) = engine.get_scene_manager_mut() {
                        *scene_manager = loaded_project.scene_manager;
                        println!("Loaded scenes: {:?}", scene_manager.list_scene());
                    }
                }
                Err(e) => println!("Failed to load project: {}", e),
            }

            // Set up input handler with debug prints
            if let Some(input_handler) = engine.get_input_handler_mut() {
                println!("Setting up input handler");
                
                // Test callback that should always work
                input_handler.register_key_callback(Key::T, Box::new(|_runtime| {
                    println!("T key pressed - Basic test");
                }));

                // Register game controls
                input_handler.register_key_callback(Key::Space, Box::new(|runtime| {
                    println!("Space key pressed - Attempting bird jump");
                    if let Some(scene_manager) = runtime.get_scene_manager() {
                        println!("Got scene manager");
                        if let Some(scene) = scene_manager.get_active_scene() {
                            println!("Found active scene: {}", scene.name);
                            if let Some(bird) = scene.get_entity_by_name("bird") {
                                let current_y = bird.get_y();
                                bird.set_y(current_y - 10.0);
                                println!("Bird jumped to y={}", bird.get_y());
                            } else {
                                println!("Bird entity not found in scene!");
                            }
                        } else {
                            println!("No active scene found!");
                        }
                    } else {
                        println!("Failed to get scene manager!");
                    }
                }));

                println!("Input handler setup complete - Try pressing T for basic test");
            } else {
                println!("Failed to get input handler!");
            }
            
            Box::new(engine)
        })
    )
}

