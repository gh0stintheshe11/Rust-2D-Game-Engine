use crate::{
    audio_engine::AudioEngine,
    ecs::AttributeValue,
    ecs::SceneManager,
    input_handler::{InputContext, InputHandler},
    logger::LOGGER,
    lua_scripting::LuaScripting,
    physics_engine::PhysicsEngine,
    render_engine::RenderEngine,
};
use egui::Rect;
use std::any::Any;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimeState {
    Playing,
    Paused,
    /// A script ended the game (e.g. player death). The final frame stays
    /// on screen; the only way out is Reset.
    Ended,
    Stopped,
}

pub trait Game: Any {
    fn init(&mut self, scene_manager: &mut SceneManager);
    fn update(&mut self, scene_manager: &mut SceneManager, input: &InputHandler, delta_time: f32);
    fn reset(&mut self, scene_manager: &mut SceneManager);
}

/// Owns the play-mode state of the engine.
///
/// The scene manager, physics engine and input handler are shared via
/// `Rc<RefCell<...>>` because Lua script bindings hold handles to them;
/// see `LuaScripting::start_session`.
pub struct GameRuntime {
    scene_manager: Rc<RefCell<SceneManager>>,
    dev_state_snapshot: Option<SceneManager>, // Store entire dev state
    physics_engine: Rc<RefCell<PhysicsEngine>>,
    render_engine: RenderEngine,
    input_handler: Rc<RefCell<InputHandler>>,
    audio_engine: AudioEngine,
    running: bool,
    state: RuntimeState,
    game: Option<Box<dyn Game>>,
    lua_scripting: LuaScripting,
    // Fixed-timestep accumulator: real time not yet consumed by physics steps
    time_accumulator: f32,
}

impl GameRuntime {
    pub fn new(
        scene_manager: SceneManager,
        mut physics_engine: PhysicsEngine,
        render_engine: RenderEngine,
        mut input_handler: InputHandler,
        audio_engine: AudioEngine,
        target_fps: u32,
    ) -> Self {
        // Make sure we start in EngineUI mode
        input_handler.set_context(InputContext::EngineUI);

        // target_fps is the fixed simulation rate; rendering runs at the
        // display refresh rate and physics catches up in fixed steps
        physics_engine.set_time_step(1.0 / target_fps as f32);

        Self {
            scene_manager: Rc::new(RefCell::new(scene_manager)),
            dev_state_snapshot: None,
            physics_engine: Rc::new(RefCell::new(physics_engine)),
            render_engine,
            input_handler: Rc::new(RefCell::new(input_handler)),
            audio_engine,
            running: false,
            state: RuntimeState::Stopped,
            game: None,
            lua_scripting: LuaScripting::new(),
            time_accumulator: 0.0,
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
        match state {
            RuntimeState::Playing => {
                // Take dev snapshot before playing
                if self.dev_state_snapshot.is_none() {
                    self.dev_state_snapshot = Some(self.scene_manager.borrow().clone());
                }

                // Switch to game mode only when playing
                self.input_handler
                    .borrow_mut()
                    .set_context(InputContext::Game);
                self.running = true;
            }
            RuntimeState::Paused => {
                // Stay in game mode but paused
                self.running = false;
            }
            RuntimeState::Ended => {
                // Game over: freeze the frame, hand input back to the editor.
                // The world stays loaded so the final state remains visible.
                self.input_handler
                    .borrow_mut()
                    .set_context(InputContext::EngineUI);
                self.running = false;
            }
            RuntimeState::Stopped => {
                // Switch back to editor mode
                self.input_handler
                    .borrow_mut()
                    .set_context(InputContext::EngineUI);
                self.physics_engine.borrow_mut().cleanup();
                self.running = false;
            }
        }
        self.state = state;
    }

    pub fn reset(&mut self) {
        // First stop everything
        self.set_state(RuntimeState::Stopped);

        // Clear all engines
        self.physics_engine.borrow_mut().cleanup();
        self.render_engine.cleanup();
        self.audio_engine.cleanup();

        // Restore the pre-play editor state, then drop the snapshot so the
        // next Play captures the current editor state instead of this stale one
        if let Some(snapshot) = self.dev_state_snapshot.take() {
            *self.scene_manager.borrow_mut() = snapshot;
        }

        // Stay in Stopped state, waiting for user to hit play
    }

    pub fn run(&mut self) -> Result<(), String> {
        // Initialize game if we have one
        if let Some(game) = &mut self.game {
            game.init(&mut self.scene_manager.borrow_mut());
        }

        {
            let mut manager = self.scene_manager.borrow_mut();
            let scene_ids: Vec<uuid::Uuid> =
                manager.list_scene().iter().map(|(id, _)| *id).collect();

            if scene_ids.is_empty() {
                return Err("Cannot run: No scenes in project. Create a scene first.".to_string());
            }

            // If no active scene but we have scenes, set the first one as active
            if manager.get_active_scene().is_none() {
                manager.set_active_scene(scene_ids[0])?;
            }
        }

        // Load the scene into physics engine, replacing any previously
        // loaded physics world so repeated runs don't leak duplicate bodies
        {
            let mut physics = self.physics_engine.borrow_mut();
            physics.cleanup();
            let manager = self.scene_manager.borrow();
            if let Some(scene) = manager.get_active_scene() {
                physics.load_scene(scene);
            }
        }

        // Start a fresh Lua session (new VM + engine bindings)
        self.lua_scripting
            .start_session(
                Rc::clone(&self.scene_manager),
                Rc::clone(&self.physics_engine),
                Rc::clone(&self.input_handler),
            )
            .map_err(|e| format!("Failed to start Lua session: {}", e))?;

        LOGGER.info("Game starting with active scene");
        self.time_accumulator = 0.0;
        self.running = true;
        self.state = RuntimeState::Playing;
        Ok(())
    }

    // This will be called from the eframe update loop
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, viewport_rect: Rect) {
        // Update viewport of the render engine
        self.render_engine
            .update_viewport_size(viewport_rect.width(), viewport_rect.height());

        // Update input state first - IMPORTANT!
        ctx.input(|input| {
            self.input_handler.borrow_mut().handle_input(input);
        });

        // Only update game logic if we're running and in Playing state
        if self.running && self.state == RuntimeState::Playing {
            // Real elapsed time since the last frame (clamped so a stall -
            // window drag, breakpoint - doesn't cause a huge catch-up burst)
            let frame_dt = ctx.input(|i| i.stable_dt).min(0.25);

            // Update game logic with the input handler
            if let Some(game) = &mut self.game {
                game.update(
                    &mut self.scene_manager.borrow_mut(),
                    &self.input_handler.borrow(),
                    frame_dt,
                );
            }

            // Run scripts once per rendered frame with real delta time
            if let Err(e) = self.lua_scripting.update_global_time(frame_dt) {
                LOGGER.error(format!("Failed to update Lua time: {}", e));
            }
            if let Err(e) = self
                .lua_scripting
                .bind_keys_pressed(&self.input_handler.borrow())
            {
                LOGGER.error(format!("Failed to update Lua keys_pressed: {}", e));
            }

            let active_scene_id = self.scene_manager.borrow().active_scene;
            if let Some(active_scene_id) = active_scene_id {
                if let Err(err) = self.lua_scripting.run_scripts_for_scene(active_scene_id) {
                    LOGGER.error(format!(
                        "Error running scripts for scene {}: {}",
                        active_scene_id, err
                    ));
                }
            }

            // A script may have requested the game to stop (e.g. player died)
            if self.lua_scripting.take_game_stop_request() {
                LOGGER.info("Game over: a script called end_game()");
                self.set_state(RuntimeState::Ended);
                self.paint_scene(ui, viewport_rect);
                ctx.request_repaint();
                return;
            }

            // Run physics on a fixed timestep, decoupled from the display
            // refresh rate: accumulate real time and consume it in fixed
            // steps so simulation speed is identical on 60Hz and 144Hz
            // monitors.
            let step_dt = self.physics_engine.borrow().get_time_step();
            self.time_accumulator += frame_dt;
            // Never run more than a handful of catch-up steps per frame
            self.time_accumulator = self.time_accumulator.min(step_dt * 5.0);

            let mut scene_lost = false;
            while self.time_accumulator >= step_dt {
                self.time_accumulator -= step_dt;

                let mut manager = self.scene_manager.borrow_mut();
                match manager.get_active_scene_mut() {
                    Some(scene) => {
                        let physics_updates = self.physics_engine.borrow_mut().step(scene);

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
                            LOGGER.error(format!("Failed to update entity attributes: {}", err));
                        }
                    }
                    None => {
                        scene_lost = true;
                        break;
                    }
                }
            }

            if scene_lost {
                // If we lost the active scene, stop the game
                self.cleanup_and_reset();
                ctx.request_repaint();
                return;
            }

            // Run audio
            self.audio_engine.update();

            // Render
            self.paint_scene(ui, viewport_rect);
        } else if matches!(self.state, RuntimeState::Paused | RuntimeState::Ended) {
            // Keep drawing the current state while paused or after game over
            self.paint_scene(ui, viewport_rect);
        }

        ctx.request_repaint();
    }

    /// Paint the active scene (sprites + collider debug shapes) into the
    /// viewport. Used by both the playing and paused states.
    fn paint_scene(&mut self, ui: &mut egui::Ui, viewport_rect: Rect) {
        let render_queue = {
            let manager = self.scene_manager.borrow();
            match manager.get_active_scene() {
                Some(scene) => self.render_engine.render(scene),
                None => return,
            }
        };

        for (texture_id, pos, size, _layer) in render_queue {
            let Some(texture) = self.render_engine.get_egui_texture(ui.ctx(), texture_id) else {
                continue;
            };

            let texture_rect = egui::Rect::from_min_size(
                egui::pos2(viewport_rect.min.x + pos.0, viewport_rect.min.y + pos.1),
                egui::vec2(size.0, size.1),
            );

            let intersection = texture_rect.intersect(viewport_rect);
            if !intersection.is_positive() {
                continue;
            }

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

        // render colliders
        let collider_data = self.physics_engine.borrow().get_collider_data();
        let collider_render_queue = self.render_engine.render_colliders(&collider_data);

        for (screen_position, screen_size, shape) in collider_render_queue {
            match shape.as_str() {
                "Circle" => {
                    let center = egui::pos2(
                        viewport_rect.min.x + screen_position.0,
                        viewport_rect.min.y + screen_position.1,
                    );
                    let radius = screen_size.0 / 2.0;
                    ui.painter().circle_stroke(
                        center,
                        radius,
                        egui::Stroke::new(1.0_f32, egui::Color32::RED),
                    );
                }
                "Rectangle" => {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(
                            viewport_rect.min.x + screen_position.0 - screen_size.0 / 2.0,
                            viewport_rect.min.y + screen_position.1 - screen_size.1 / 2.0,
                        ),
                        egui::vec2(screen_size.0, screen_size.1),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(1.0_f32, egui::Color32::BLUE),
                        egui::StrokeKind::Middle,
                    );
                }
                _ => {}
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
        self.physics_engine.borrow_mut().cleanup();
        self.render_engine.cleanup();
        self.audio_engine.cleanup();

        // Restore dev state if needed
        if let Some(snapshot) = self.dev_state_snapshot.take() {
            *self.scene_manager.borrow_mut() = snapshot;
        }

        // Reset input context
        self.input_handler
            .borrow_mut()
            .set_context(InputContext::EngineUI);
    }

    pub fn set_scene_manager(&mut self, scene_manager: SceneManager) {
        *self.scene_manager.borrow_mut() = scene_manager;
    }

    pub fn get_scene_manager(&self) -> Ref<'_, SceneManager> {
        self.scene_manager.borrow()
    }

    /// Feed the runtime's input handler with the latest egui input state.
    pub fn handle_input(&self, input: &egui::InputState) {
        self.input_handler.borrow_mut().handle_input(input);
    }

    pub fn get_input_context(&self) -> InputContext {
        self.input_handler.borrow().get_context().clone()
    }

    pub fn set_game(&mut self, game: Box<dyn Game>) {
        self.game = Some(game);
    }

    pub fn set_camera_state(&mut self, position: (f32, f32), zoom: f32) {
        self.render_engine.camera.position = position;
        self.render_engine.camera.zoom = zoom;
    }
}
