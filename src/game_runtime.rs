use crate::{
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
    input_handler::{InputHandler, InputContext},
    audio_engine::AudioEngine,
    ecs::SceneManager,
};
use std::time::{Duration, Instant};

pub enum RuntimeState {
    Stopped,  // Not running, initial state
    Playing,  // Game is running
    Paused,   // Game is paused but preserves state
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
        }
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, RuntimeState::Playing)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self.state, RuntimeState::Paused)
    }

    pub fn get_state(&self) -> &RuntimeState {
        &self.state
    }

    pub fn set_state(&mut self, state: RuntimeState) {
        match state {
            RuntimeState::Playing => {
                // Take a snapshot of entire dev state
                self.dev_state_snapshot = Some(self.scene_manager.clone());
                self.input_handler.set_context(InputContext::Game);
            }
            RuntimeState::Paused => {
                // Keep game state but stop processing
                // Just keep rendering the current state
            }
            RuntimeState::Stopped => {
                // Restore entire dev state
                if let Some(snapshot) = self.dev_state_snapshot.take() {
                    self.scene_manager = snapshot;
                }
                self.input_handler.set_context(InputContext::EngineUI);
                self.physics_engine.cleanup();
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
        self.running = true;
        let frame_duration = Duration::from_secs_f32(1.0 / self.target_fps as f32);

        while self.running {
            let frame_start = Instant::now();

            if let Some(scene) = self.scene_manager.get_active_scene_mut() {
                match self.state {
                    RuntimeState::Playing => {
                        // Run physics
                        let physics_updates = self.physics_engine.step(scene);
                        scene.update_entity_attributes(physics_updates);
                        // Run audio
                        self.audio_engine.update();
                    }
                    RuntimeState::Paused => {
                        // Just render current state
                    }
                    RuntimeState::Stopped => {
                        break; // Exit run loop
                    }
                }

                // Always render
                self.render_engine.render(scene);
            }

            // Frame timing
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                std::thread::sleep(frame_duration - elapsed);
            }
        }

        Ok(())
    }
}