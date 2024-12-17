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
                        
                        // Ensure we have an active scene
                        if scene_manager.get_active_scene().is_none() {
                            if let Some((first_id, _)) = scene_manager.list_scene().first() {
                                println!("Setting first scene as active: {}", first_id);
                                scene_manager.set_active_scene(*first_id).unwrap();
                            }
                        }
                    }
                }
                Err(e) => println!("Failed to load project: {}", e),
            }

            // Set up input handler
            if let Some(input_handler) = engine.get_input_handler_mut() {
                println!("Setting up input handler");
                input_handler.set_context(InputContext::Game);
                
                input_handler.register_key_callback(Key::W, Box::new(|runtime| {
                    println!("W key pressed");
                    if let Some(scene) = runtime.scene_manager.get_active_scene_mut() {
                        if let Some(bird) = scene.get_entity_by_name("bird") {
                            let current_y = bird.get_y();
                            bird.set_y(current_y - 5.0);
                        }
                    }
                }));
            }

            // Start the game runtime
            if let Some(runtime) = engine.get_game_runtime_mut() {
                println!("Starting game runtime...");
                runtime.set_state(RuntimeState::Playing);
                if let Err(e) = runtime.run() {
                    println!("Failed to start game runtime: {}", e);
                } else {
                    println!("Game runtime started successfully");
                }
            } else {
                println!("Failed to get game runtime!");
            }
            
            Box::new(engine)
        })
    )
}

