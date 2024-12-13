use crate::{
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
    input_handler::{InputHandler, InputContext},
    audio_engine::AudioEngine,
    ecs::SceneManager,
};
use std::any::Any;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimeState {
    Playing,
    Paused,
    Stopped,
}

pub trait Game: Any {
    fn init(&mut self, scene_manager: &mut SceneManager);
    fn update(&mut self, scene_manager: &mut SceneManager, input: &InputHandler, delta_time: f32);
    fn reset(&mut self, scene_manager: &mut SceneManager);
}

pub struct GameRuntime {
    scene_manager: SceneManager,
    dev_state_snapshot: Option<SceneManager>,  // Store entire dev state
    physics_engine: PhysicsEngine,
    render_engine: RenderEngine,
    input_handler: InputHandler,
    audio_engine: AudioEngine,
    target_fps: u32,
    running: bool,
    state: RuntimeState,
    game: Option<Box<dyn Game>>,
}

impl GameRuntime {
    pub fn new(
        scene_manager: SceneManager,
        physics_engine: PhysicsEngine,
        render_engine: RenderEngine,
        input_handler: InputHandler,
        audio_engine: AudioEngine,
        target_fps: u32,
    ) -> Self {
        // Make sure we start in EngineUI mode
        let mut input_handler = input_handler;
        input_handler.set_context(InputContext::EngineUI);
        
        Self {
            scene_manager,
            dev_state_snapshot: None,
            physics_engine,
            render_engine,
            input_handler,
            audio_engine,
            target_fps,
            running: false,
            state: RuntimeState::Stopped,
            game: None,
        }
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, RuntimeState::Playing)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.state, RuntimeState::Paused)
    }

    pub fn get_state(&self) -> RuntimeState {
        self.state
    }

    pub fn set_state(&mut self, state: RuntimeState) {
        println!("Setting game state to {:?}", state); // Debug print
        match state {
            RuntimeState::Playing => {
                // Take dev snapshot before playing
                if self.dev_state_snapshot.is_none() {
                    self.dev_state_snapshot = Some(self.scene_manager.clone());
                }
                
                // Switch to game mode only when playing
                println!("Switching to game input context");
                self.input_handler.set_context(InputContext::Game);
                self.running = true;
            }
            RuntimeState::Paused => {
                // Stay in game mode but paused
                self.running = false;
            }
            RuntimeState::Stopped => {
                // Switch back to editor mode
                println!("Switching back to engine UI input context");
                self.input_handler.set_context(InputContext::EngineUI);
                self.physics_engine.cleanup();
                self.running = false;
            }
        }
        self.state = state;
    }

    pub fn reset(&mut self) {
        // First stop everything
        self.set_state(RuntimeState::Stopped);

        // Clear all engines
        self.physics_engine.cleanup();
        self.render_engine.cleanup();
        self.audio_engine.cleanup();
        
        // remove the old scene manager
        self.scene_manager = SceneManager::new();
        
        // Create fresh scene manager from dev snapshot
        if let Some(snapshot) = &self.dev_state_snapshot {
            self.scene_manager = snapshot.clone();
        }
        
        // Stay in Stopped state, waiting for user to hit play
    }

    pub fn run(&mut self) -> Result<(), String> {
        println!("Attempting to run game..."); // Debug print

        // Initialize game if we have one
        if let Some(game) = &mut self.game {
            game.init(&mut self.scene_manager);
        }

        // Debug print the scene list
        let scenes = self.scene_manager.list_scene();
        println!("Available scenes: {:?}", scenes);

        // Check if there's anything to run
        if scenes.is_empty() {
            // Try to save the current scene first
            if let Some(scene) = self.scene_manager.get_active_scene() {
                println!("Found active scene: {}", scene.name);
                return Ok(());
            } else {
                return Err("Cannot run: No scenes in project. Create a scene first.".to_string());
            }
        }

        // If no active scene but we have scenes, set the first one as active
        if self.scene_manager.get_active_scene().is_none() && !scenes.is_empty() {
            println!("No active scene, setting first scene as active"); // Debug print
            self.scene_manager.set_active_scene(scenes[0].0)?;
        }

        println!("Game starting with active scene"); // Debug print
        self.running = true;
        self.state = RuntimeState::Playing;
        Ok(())
    }

    // This will be called from the eframe update loop
    pub fn update(&mut self, ctx: &egui::Context) {
        // Update input state first - IMPORTANT!
        ctx.input(|input| {
            self.input_handler.handle_input(input);
        });

        // Only update game logic if we're running and in Playing state
        if self.running && self.state == RuntimeState::Playing {
            //println!("Game is running, active inputs: {:?}", self.input_handler.get_all_active_inputs()); // Debug print
            
            // Update game logic with the input handler
            if let Some(game) = &mut self.game {
                game.update(&mut self.scene_manager, &self.input_handler, 1.0/60.0);
            }

            if let Some(scene) = self.scene_manager.get_active_scene_mut() {
                // Run physics
                let physics_updates = self.physics_engine.step(scene);
                scene.update_entity_attributes(physics_updates);
                // Run audio
                self.audio_engine.update();
                // Render
                self.render_engine.render(scene);
            } else {
                // If we lost the active scene, stop the game
                self.cleanup_and_reset();
            }
        } else if self.state == RuntimeState::Paused {
            // Just render current state if paused
            if let Some(scene) = self.scene_manager.get_active_scene_mut() {
                self.render_engine.render(scene);
            }
        }
    }

    pub fn stop(&mut self) {
        self.cleanup_and_reset();
    }

    fn cleanup_and_reset(&mut self) {
        // Stop all running systems
        self.running = false;
        self.state = RuntimeState::Stopped;
        
        // Cleanup engines
        self.physics_engine.cleanup();
        self.render_engine.cleanup();
        self.audio_engine.cleanup();
        
        // Restore dev state if needed
        if let Some(snapshot) = &self.dev_state_snapshot {
            self.scene_manager = snapshot.clone();
        }
        
        // Reset input context
        self.input_handler.set_context(InputContext::EngineUI);
    }

    pub fn set_scene_manager(&mut self, scene_manager: SceneManager) {
        self.scene_manager = scene_manager;
    }

    pub fn get_scene_manager(&self) -> &SceneManager {
        &self.scene_manager
    }

    pub fn get_input_handler(&mut self) -> &mut InputHandler {
        &mut self.input_handler
    }

    pub fn get_input_context(&self) -> &InputContext {
        self.input_handler.get_context()
    }

    pub fn set_game(&mut self, game: Box<dyn Game>) {
        self.game = Some(game);
    }
}