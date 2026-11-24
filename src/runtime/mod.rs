//! Play mode: the state machine and the per-frame game loop tying scenes,
//! scripts, physics, audio and rendering together.
//!
//! [`GameRuntime::update`] is what the editor and the standalone player call
//! every frame. It is [`tick`](GameRuntime::tick) (simulation, headless and
//! testable) followed by [`paint`](GameRuntime::paint).

use crate::{
    audio::AudioEngine, input::InputHandler, logger::LOGGER, physics::PhysicsEngine,
    render::RenderEngine, scene::SceneManager, scripting::LuaScripting,
};
use egui::Rect;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

/// At most this many fixed physics steps run per rendered frame; slower
/// frames make the whole game (scripts included) run in slow motion instead
/// of spiraling into ever longer catch-up work.
const MAX_STEPS_PER_FRAME: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Playing,
    Paused,
    /// A script ended the game (e.g. player death). The final frame stays
    /// on screen; the only way out is a reset.
    Ended,
    Stopped,
}

/// Which view of the world [`GameRuntime::paint`] draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraMode {
    /// The scene's default camera entity, scaled to fit the viewport and
    /// letterboxed. Used by exported games.
    Game,
    /// A camera controlled by the caller (the editor's pan and zoom).
    Free { position: (f32, f32), zoom: f32 },
}

/// Owns the play-mode state of the engine.
///
/// The scene manager, physics engine, input handler and audio engine are
/// shared via `Rc<RefCell<...>>` because Lua script bindings hold handles to
/// them; see `LuaScripting::start_session`.
pub struct GameRuntime {
    scene_manager: Rc<RefCell<SceneManager>>,
    /// The scenes as they were when `run` started, restored by `reset`
    initial_state: Option<SceneManager>,
    physics_engine: Rc<RefCell<PhysicsEngine>>,
    render_engine: RenderEngine,
    input_handler: Rc<RefCell<InputHandler>>,
    audio_engine: Rc<RefCell<AudioEngine>>,
    state: RuntimeState,
    lua_scripting: LuaScripting,
    camera_mode: CameraMode,
    // Draw physics collider outlines over the game (a debugging aid)
    show_colliders: bool,
    // Fixed-timestep accumulator: real time not yet consumed by physics steps
    time_accumulator: f32,
}

impl GameRuntime {
    /// `target_fps` is the fixed simulation rate; rendering runs at the
    /// display refresh rate and physics catches up in fixed steps.
    pub fn new(
        scene_manager: SceneManager,
        mut physics_engine: PhysicsEngine,
        render_engine: RenderEngine,
        input_handler: InputHandler,
        audio_engine: AudioEngine,
        target_fps: u32,
    ) -> Self {
        physics_engine.set_time_step(1.0 / target_fps.max(1) as f32);

        Self {
            scene_manager: Rc::new(RefCell::new(scene_manager)),
            initial_state: None,
            physics_engine: Rc::new(RefCell::new(physics_engine)),
            render_engine,
            input_handler: Rc::new(RefCell::new(input_handler)),
            audio_engine: Rc::new(RefCell::new(audio_engine)),
            state: RuntimeState::Stopped,
            lua_scripting: LuaScripting::new(),
            camera_mode: CameraMode::Game,
            show_colliders: false,
            time_accumulator: 0.0,
        }
    }

    pub fn get_state(&self) -> RuntimeState {
        self.state
    }

    pub fn is_playing(&self) -> bool {
        self.state == RuntimeState::Playing
    }

    pub fn is_paused(&self) -> bool {
        self.state == RuntimeState::Paused
    }

    /// Start playing the active scene (the first scene if none is active):
    /// fresh physics world, fresh Lua session. Starting while not
    /// [`Stopped`](RuntimeState::Stopped) resets first.
    pub fn run(&mut self) -> Result<(), String> {
        if self.state != RuntimeState::Stopped {
            self.reset();
        }

        {
            let mut manager = self.scene_manager.borrow_mut();
            let Some(first_scene) = manager.list_scene().first().map(|(id, _)| *id) else {
                return Err("Cannot run: No scenes in project. Create a scene first.".to_string());
            };
            if manager.get_active_scene().is_none() {
                manager.set_active_scene(first_scene)?;
            }
            self.initial_state = Some(manager.clone());
        }

        // Load the scene into a fresh physics world
        {
            let mut physics = self.physics_engine.borrow_mut();
            physics.cleanup();
            if let Some(scene) = self.scene_manager.borrow().get_active_scene() {
                physics.load_scene(scene);
            }
        }

        if let Err(e) = self.lua_scripting.start_session(
            Rc::clone(&self.scene_manager),
            Rc::clone(&self.physics_engine),
            Rc::clone(&self.input_handler),
            Rc::clone(&self.audio_engine),
        ) {
            self.reset();
            return Err(format!("Failed to start Lua session: {}", e));
        }

        LOGGER.info("Game starting with active scene");
        self.time_accumulator = 0.0;
        self.state = RuntimeState::Playing;
        Ok(())
    }

    /// Freeze the simulation (and audio); the scene keeps rendering.
    pub fn pause(&mut self) {
        if self.state == RuntimeState::Playing {
            self.audio_engine.borrow_mut().pause_all();
            self.state = RuntimeState::Paused;
        }
    }

    /// Continue a paused game where it left off.
    pub fn resume(&mut self) {
        if self.state == RuntimeState::Paused {
            self.audio_engine.borrow_mut().resume_all();
            self.state = RuntimeState::Playing;
        }
    }

    /// Stop the game and restore the scenes to how they were when it
    /// started.
    pub fn reset(&mut self) {
        self.state = RuntimeState::Stopped;
        self.time_accumulator = 0.0;
        self.physics_engine.borrow_mut().cleanup();
        self.render_engine.clear_cache();
        self.audio_engine.borrow_mut().cleanup();
        if let Some(initial) = self.initial_state.take() {
            *self.scene_manager.borrow_mut() = initial;
        }
    }

    /// Reset and start again, e.g. after game over.
    pub fn restart(&mut self) -> Result<(), String> {
        self.reset();
        self.run()
    }

    /// One rendered frame: feed input, advance the game by the frame's real
    /// duration, and paint it (unless stopped).
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, viewport_rect: Rect) {
        ctx.input(|input| self.input_handler.borrow_mut().handle_input(input));
        let frame_dt = ctx.input(|i| i.stable_dt);
        self.tick(frame_dt);
        if self.state != RuntimeState::Stopped {
            self.paint(ui, viewport_rect);
        }
        ctx.request_repaint();
    }

    /// Advance the game by one rendered frame of `frame_dt` seconds: scripts
    /// run once, then physics catches up in fixed steps, each followed by
    /// the `on_collision` hooks of contacts it started. Does nothing unless
    /// playing.
    pub fn tick(&mut self, frame_dt: f32) {
        if self.state != RuntimeState::Playing {
            return;
        }
        let Some(scene_id) = self.scene_manager.borrow().active_scene else {
            LOGGER.error("The active scene is gone; stopping the game");
            self.reset();
            return;
        };

        let step_dt = self.physics_engine.borrow().get_time_step();
        // Scripts and physics see the same (clamped) time
        let frame_dt = frame_dt.clamp(0.0, step_dt * MAX_STEPS_PER_FRAME as f32);
        self.physics_engine.borrow_mut().begin_frame(frame_dt);

        if let Err(e) = self.lua_scripting.update_global_time(frame_dt) {
            LOGGER.error(format!("Failed to update Lua time: {}", e));
        }
        if let Err(e) = self
            .lua_scripting
            .bind_keys_pressed(&self.input_handler.borrow())
        {
            LOGGER.error(format!("Failed to update Lua keys_pressed: {}", e));
        }
        if let Err(e) = self.lua_scripting.run_scripts_for_scene(scene_id) {
            LOGGER.error(format!(
                "Error running scripts for scene {}: {}",
                scene_id, e
            ));
        }
        if self.end_if_requested() {
            return;
        }

        self.time_accumulator += frame_dt;
        while self.time_accumulator >= step_dt {
            self.time_accumulator -= step_dt;
            {
                let mut manager = self.scene_manager.borrow_mut();
                let Some(scene) = manager.get_scene_mut(scene_id) else {
                    break;
                };
                self.physics_engine.borrow_mut().step(scene);
            }
            if let Err(e) = self.lua_scripting.dispatch_collision_events(scene_id) {
                LOGGER.error(format!("Error dispatching collision events: {}", e));
            }
            if self.end_if_requested() {
                return;
            }
        }

        self.audio_engine.borrow_mut().update();
    }

    /// Switch to [`Ended`](RuntimeState::Ended) if a script called
    /// `end_game()`.
    fn end_if_requested(&mut self) -> bool {
        if !self.lua_scripting.take_game_stop_request() {
            return false;
        }
        LOGGER.info("Game over: a script called end_game()");
        self.audio_engine.borrow_mut().stop_all();
        self.state = RuntimeState::Ended;
        true
    }

    /// Paint the active scene into `viewport_rect`, through the current
    /// [`CameraMode`]. Collider outlines are drawn on top when enabled with
    /// [`set_show_colliders`](Self::set_show_colliders).
    pub fn paint(&mut self, ui: &mut egui::Ui, viewport_rect: Rect) {
        let manager = self.scene_manager.borrow();
        let Some(scene) = manager.get_active_scene() else {
            return;
        };

        let mut clip_rect = viewport_rect;
        match self.camera_mode {
            CameraMode::Free { position, zoom } => {
                self.render_engine.camera.position = position;
                self.render_engine.camera.zoom = zoom;
            }
            CameraMode::Game => {
                let viewport_size = (viewport_rect.width(), viewport_rect.height());
                if let Some(view) = crate::render::game_view(scene, viewport_size) {
                    self.render_engine.camera = view.camera;
                    let (x, y, w, h) = view.frame;
                    clip_rect =
                        Rect::from_min_size(viewport_rect.min + egui::vec2(x, y), egui::vec2(w, h));
                    // Letterbox bars around the camera frame
                    ui.painter()
                        .rect_filled(viewport_rect, 0.0, egui::Color32::BLACK);
                }
            }
        }

        self.render_engine
            .update_viewport_size(viewport_rect.width(), viewport_rect.height());
        let render_queue = self.render_engine.render(scene);
        drop(manager);

        // Clip all game drawing to the visible area; the GPU scissor also
        // handles rotated sprites correctly
        let painter = ui.painter().with_clip_rect(clip_rect);
        for entry in render_queue {
            let Some(texture) = self
                .render_engine
                .get_egui_texture(ui.ctx(), entry.texture_id)
            else {
                continue;
            };
            let rect = Rect::from_min_size(
                viewport_rect.min + egui::vec2(entry.screen_pos.0, entry.screen_pos.1),
                egui::vec2(entry.screen_size.0, entry.screen_size.1),
            );
            crate::render::paint_sprite(&painter, texture.id(), rect, entry.rotation);
        }

        if self.show_colliders {
            let outlines = self.physics_engine.borrow().collider_outlines();
            for outline in self.render_engine.render_colliders(&outlines) {
                crate::render::paint_collider_outline(&painter, &outline, viewport_rect.min);
            }
        }
    }

    pub fn set_camera_mode(&mut self, mode: CameraMode) {
        self.camera_mode = mode;
    }

    /// Draw physics collider outlines over the game, for debugging. Off by
    /// default, so exported games never show them.
    pub fn set_show_colliders(&mut self, show: bool) {
        self.show_colliders = show;
    }

    /// Replace the scenes, e.g. with the editor's current state before
    /// playing. Takes effect at the next [`run`](Self::run).
    pub fn set_scene_manager(&mut self, scene_manager: SceneManager) {
        *self.scene_manager.borrow_mut() = scene_manager;
    }

    pub fn get_scene_manager(&self) -> Ref<'_, SceneManager> {
        self.scene_manager.borrow()
    }
}
