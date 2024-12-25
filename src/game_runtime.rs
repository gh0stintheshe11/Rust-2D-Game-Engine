use crate::{
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
    input_handler::{InputHandler, InputContext},
    audio_engine::AudioEngine,
    ecs::SceneManager,
    ecs::AttributeValue,
    lua_scripting::LuaScripting,
};
use std::any::Any;
use egui::Rect;
use uuid::uuid;

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
    lua_scripting: LuaScripting,
}

impl GameRuntime {
    pub fn new(
        scene_manager: SceneManager,
        mut physics_engine: PhysicsEngine,
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
            lua_scripting: LuaScripting::new(),
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

        // Load the scene into physics engine
        self.physics_engine.load_scene(self.scene_manager.get_active_scene().unwrap());

        println!("Game starting with active scene"); // Debug print
        self.running = true;
        self.state = RuntimeState::Playing;
        Ok(())
    }

    // This will be called from the eframe update loop
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, viewport_rect: Rect) {

        // Update viewport of the render engine
        self.render_engine.update_viewport_size(viewport_rect.width(), viewport_rect.height());

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

            // Run script
            self.lua_scripting.update_global_time(1.0/self.target_fps as f32).expect("Failed to update global time");
            match self.lua_scripting.load_scene_manager(&self.scene_manager) {
                Ok(_) => println!("SceneManager loaded into Lua successfully."),
                Err(err) => eprintln!("Error loading SceneManager into Lua: {}", err),
            }
            if let Some(active_scene_id) = self.scene_manager.active_scene {
                self.lua_scripting.initializing_global_variables(&self.input_handler);
                self.lua_scripting.initialize_bindings_physics_engine(&mut self.physics_engine, &mut self.scene_manager).unwrap();
                self.lua_scripting.initialize_bindings_ecs(&mut self.scene_manager).unwrap();
                self.lua_scripting.initialize_bindings_input_handler(&mut self.input_handler).unwrap();


                match self.lua_scripting.run_scripts_for_scene(&mut self.scene_manager, active_scene_id) {
                    Ok(()) => {
                        println!("SceneManager successfully updated after running scripts.");
                    }
                    Err(err) => {
                        eprintln!("Error running scripts for scene {}: {}", active_scene_id, err);
                    }
                }
            } else {
                eprintln!("No active scene set in SceneManager.");
            }

            if let Some(scene) = self.scene_manager.get_active_scene_mut() {
                // Run physics
                let physics_updates = self.physics_engine.step(scene);

                // Filter out those values are NaN
                let filtered_physics_updates: Vec<(_, _, AttributeValue)> = physics_updates
                    .into_iter()
                    .filter(|(_, _, attr)| match attr {
                        AttributeValue::Float(val) => !val.is_nan(),
                        AttributeValue::Vector2(x, y) => !x.is_nan() && !y.is_nan(),
                        _ => true,
                    })
                    .collect();

                if let Err(err) = scene.update_entity_attributes(filtered_physics_updates) {
                    eprintln!("Failed to update entity attributes: {}", err);
                }
                // Run audio
                self.audio_engine.update();
                // Render
                let render_queue = self.render_engine.render(scene);

                // Function for calculate intersection
                let calculate_intersection = |rect1: egui::Rect, rect2: egui::Rect| -> Option<egui::Rect> {
                    let min_x = rect1.min.x.max(rect2.min.x);
                    let min_y = rect1.min.y.max(rect2.min.y);
                    let max_x = rect1.max.x.min(rect2.max.x);
                    let max_y = rect1.max.y.min(rect2.max.y);

                    if min_x < max_x && min_y < max_y {
                        Some(egui::Rect::from_min_max(
                            egui::pos2(min_x, min_y),
                            egui::pos2(max_x, max_y),
                        ))
                    } else {
                        None
                    }
                };

                for (texture_id, pos, size, _layer) in render_queue {
                    if let Some(texture_info) = self.render_engine.texture_cache.get(&texture_id) {
                        let texture_rect = egui::Rect::from_min_size(
                            egui::pos2(viewport_rect.min.x + pos.0, viewport_rect.min.y + pos.1),
                            egui::vec2(size.0, size.1),
                        );

                        if let Some(intersection) = calculate_intersection(texture_rect, viewport_rect) {
                            // Adjust UV coordinates for the clipped area
                            let uv_min = (
                                (intersection.min.x - texture_rect.min.x) / size.0,
                                (intersection.min.y - texture_rect.min.y) / size.1,
                            );
                            let uv_max = (
                                (intersection.max.x - texture_rect.min.x) / size.0,
                                (intersection.max.y - texture_rect.min.y) / size.1,
                            );

                            // Render only the visible part
                            let texture = ui.ctx().load_texture(
                                format!("texture_{}", texture_id),
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [texture_info.dimensions.0 as usize, texture_info.dimensions.1 as usize],
                                    &texture_info.data,
                                ),
                                Default::default(),
                            );

                            ui.painter().image(
                                texture.id(),
                                intersection,
                                egui::Rect::from_min_max(
                                    egui::pos2(uv_min.0, uv_min.1),
                                    egui::pos2(uv_max.0, uv_max.1),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    }
                }
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

        ctx.request_repaint();
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

    pub fn set_camera_state(&mut self, position: (f32, f32), zoom: f32) {
        self.render_engine.camera.position = position;
        self.render_engine.camera.zoom = zoom;
    }
}