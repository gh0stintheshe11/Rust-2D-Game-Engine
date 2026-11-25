//! Play mode: the state machine and the per-frame game loop tying scenes,
//! scripts, physics, audio and rendering together.
//!
//! [`GameRuntime::update`] is what the editor and the standalone player call
//! every frame. It is [`tick`](GameRuntime::tick) (simulation, headless and
//! testable) followed by [`paint`](GameRuntime::paint).

use crate::{
    audio::AudioEngine,
    input::{InputAccess, InputHandler},
    logger::LOGGER,
    physics::PhysicsEngine,
    render::{Camera, GameView, RenderEngine},
    save::SaveData,
    scene::{Scene, SceneManager},
    scripting::LuaScripting,
};
use egui::Rect;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashSet;
use std::rc::Rc;
use uuid::Uuid;

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
    // What scripts save between runs; shared with the scripting session
    save_data: Rc<RefCell<SaveData>>,
    state: RuntimeState,
    lua_scripting: LuaScripting,
    camera_mode: CameraMode,
    // Draw physics collider outlines over the game (a debugging aid)
    show_colliders: bool,
    // Invalid animations already reported this session
    reported_animation_problems: HashSet<String>,
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
        let save_data = Rc::new(RefCell::new(SaveData::in_memory()));
        let mut lua_scripting = LuaScripting::new();
        lua_scripting.set_save_data(Rc::clone(&save_data));

        Self {
            scene_manager: Rc::new(RefCell::new(scene_manager)),
            initial_state: None,
            physics_engine: Rc::new(RefCell::new(physics_engine)),
            render_engine,
            input_handler: Rc::new(RefCell::new(input_handler)),
            audio_engine: Rc::new(RefCell::new(audio_engine)),
            save_data,
            state: RuntimeState::Stopped,
            lua_scripting,
            camera_mode: CameraMode::Game,
            show_colliders: false,
            reported_animation_problems: HashSet::new(),
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
        self.reported_animation_problems.clear();
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
        self.render_engine.clear_particles();
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
    ///
    /// The game gets the keyboard unless another egui widget has keyboard
    /// focus (an editor text field, say), and mouse buttons and scrolling
    /// only while the pointer is over the game itself, not over widgets
    /// drawn on top of it. Scripts see the pointer in game coordinates,
    /// through the camera that paints this frame.
    pub fn update(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, viewport_rect: Rect) {
        // Click-only, so it can't take keyboard focus; widgets added later
        // in the frame (like the editor's play buttons) are on top of it
        let surface = ui.interact(
            viewport_rect,
            ui.make_persistent_id("game_surface"),
            egui::Sense::CLICK,
        );
        let access = InputAccess {
            keyboard: !ctx.egui_wants_keyboard_input(),
            pointer: surface.hovered() || surface.is_pointer_button_down_on(),
        };
        ctx.input(|input| {
            self.input_handler
                .borrow_mut()
                .handle_input_with(input, access)
        });
        self.update_game_pointer(viewport_rect);

        let frame_dt = ctx.input(|i| i.stable_dt);
        self.tick(frame_dt);
        if self.state != RuntimeState::Stopped {
            self.paint(ui, viewport_rect);
        }
        ctx.request_repaint();
    }

    /// Advance the game by one rendered frame of `frame_dt` seconds: scripts
    /// run once, then physics catches up in fixed steps, each followed by
    /// the collision hooks of contacts it started or ended, and animations
    /// and particles move on. Does nothing unless playing.
    ///
    /// When a script calls `end_game()` or `change_scene()`, the request
    /// takes effect as soon as the current pass of hooks (the scripts'
    /// updates, or one step's collision hooks) finishes, and the rest of the
    /// frame is skipped.
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
        if self.end_if_requested() || self.switch_if_requested() {
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
            if self.end_if_requested() || self.switch_if_requested() {
                return;
            }
        }

        // Sprite sheet animations and particles move on by the frame's time
        {
            let mut manager = self.scene_manager.borrow_mut();
            if let Some(scene) = manager.get_scene_mut(scene_id) {
                for problem in crate::render::advance_animations(scene, frame_dt) {
                    if self.reported_animation_problems.insert(problem.clone()) {
                        LOGGER.warning(problem);
                    }
                }
                let bursts = self.lua_scripting.take_particle_bursts();
                self.render_engine
                    .update_particles(scene, frame_dt, |_| true, bursts);
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

    /// Switch scenes if a script called `change_scene()`.
    fn switch_if_requested(&mut self) -> bool {
        let Some(scene_id) = self.lua_scripting.take_scene_change_request() else {
            return false;
        };
        match self.switch_scene(scene_id) {
            Ok(()) => true,
            Err(e) => {
                LOGGER.error(format!("change_scene failed: {}", e));
                false
            }
        }
    }

    /// Make another scene the active one while the game runs (what the
    /// `change_scene` script binding does). The scene starts over as it was
    /// when the game started, so switching to the current scene restarts
    /// it: a new physics world (keeping the current gravity) is built from
    /// it, and its scripted entities run `init` again. The scripting session
    /// (`script_state`, loaded scripts) and playing sounds carry over.
    ///
    /// Fails when no game is running or the scene doesn't exist.
    pub fn switch_scene(&mut self, scene_id: Uuid) -> Result<(), String> {
        if self.state == RuntimeState::Stopped {
            return Err("no game is running".to_string());
        }
        let name = {
            let mut manager = self.scene_manager.borrow_mut();
            let Some(scene) = manager.get_scene(scene_id) else {
                return Err(format!("scene {} not found", scene_id));
            };
            let name = scene.name.clone();
            if let Some(fresh) = self
                .initial_state
                .as_ref()
                .and_then(|initial| initial.get_scene(scene_id))
            {
                manager.scenes.insert(scene_id, fresh.clone());
            }
            manager.set_active_scene(scene_id)?;
            name
        };

        {
            let mut physics = self.physics_engine.borrow_mut();
            let (gravity_x, gravity_y) = physics.get_gravity();
            physics.cleanup();
            physics.set_gravity(gravity_x, gravity_y);
            if let Some(scene) = self.scene_manager.borrow().get_scene(scene_id) {
                physics.load_scene(scene);
            }
        }
        self.lua_scripting.begin_scene();
        self.render_engine.clear_particles();
        self.time_accumulator = 0.0;
        LOGGER.info(format!("Switched to scene '{}'", name));
        Ok(())
    }

    /// How the active scene appears in a viewport of `viewport_size`: the
    /// render camera and the viewport-relative frame it fills (smaller than
    /// the viewport when letterboxed). `None` when the game camera can't be
    /// used.
    fn view(&self, scene: &Scene, viewport_size: (f32, f32)) -> Option<GameView> {
        match self.camera_mode {
            CameraMode::Free { position, zoom } => Some(GameView {
                camera: Camera { position, zoom },
                frame: (0.0, 0.0, viewport_size.0, viewport_size.1),
            }),
            CameraMode::Game => crate::render::game_view(scene, viewport_size),
        }
    }

    /// Record where the pointer is in the game: in world coordinates
    /// through this frame's camera, and in the camera frame's pixels.
    fn update_game_pointer(&mut self, viewport_rect: Rect) {
        let manager = self.scene_manager.borrow();
        let Some(scene) = manager.get_active_scene() else {
            return;
        };
        let Some(view) = self.view(scene, (viewport_rect.width(), viewport_rect.height())) else {
            return;
        };
        let mut input = self.input_handler.borrow_mut();
        let local = input.get_mouse_pos() - viewport_rect.min;
        let world = view.camera.screen_to_world((local.x, local.y));
        let screen =
            crate::render::camera_frame(scene).map_or(world, |frame| frame.world_to_pixels(world));
        input.set_game_pointer(world, screen);
    }

    /// Paint the active scene into `viewport_rect`, through the current
    /// [`CameraMode`]: sprites, then the text scripts drew this frame, then
    /// collider outlines when enabled with
    /// [`set_show_colliders`](Self::set_show_colliders).
    pub fn paint(&mut self, ui: &mut egui::Ui, viewport_rect: Rect) {
        let manager = self.scene_manager.borrow();
        let Some(scene) = manager.get_active_scene() else {
            return;
        };

        let mut clip_rect = viewport_rect;
        if let Some(view) = self.view(scene, (viewport_rect.width(), viewport_rect.height())) {
            self.render_engine.camera = view.camera;
            let (x, y, w, h) = view.frame;
            clip_rect = Rect::from_min_size(viewport_rect.min + egui::vec2(x, y), egui::vec2(w, h));
            if self.camera_mode == CameraMode::Game {
                // Letterbox bars around the camera frame
                ui.painter()
                    .rect_filled(viewport_rect, 0.0, egui::Color32::BLACK);
            }
        }

        self.render_engine
            .update_viewport_size(viewport_rect.width(), viewport_rect.height());
        let render_queue = self.render_engine.render(scene);
        let frame = crate::render::camera_frame(scene);
        drop(manager);

        // Clip all game drawing to the visible area; the GPU scissor also
        // handles rotated sprites correctly
        let painter = ui.painter().with_clip_rect(clip_rect);
        let ctx = ui.ctx().clone();
        crate::render::paint_sprites(&painter, viewport_rect.min, &render_queue, |id| {
            self.render_engine
                .get_egui_texture(&ctx, id)
                .map(|texture| texture.id())
        });

        // Script text is laid out in the camera frame's pixels and scales
        // with the frame, wherever the frame is on screen
        let texts = self.lua_scripting.hud_texts();
        if !texts.is_empty()
            && let Some(frame) = frame
        {
            let camera = self.render_engine.camera;
            let min = camera.world_to_screen(frame.world_min);
            let origin = viewport_rect.min + egui::vec2(min.0, min.1);
            let scale = camera.zoom * frame.world_size.0 / frame.size.0;
            let frame_rect = Rect::from_min_size(
                origin,
                egui::vec2(frame.size.0 * scale, frame.size.1 * scale),
            );
            let text_painter = ui.painter().with_clip_rect(frame_rect.intersect(clip_rect));
            crate::render::paint_texts(&text_painter, &texts, origin, scale);
        }
        drop(texts);

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

    /// The render engine painting the game, with its particles.
    pub fn render_engine(&self) -> &RenderEngine {
        &self.render_engine
    }

    /// The input the game saw in the last [`update`](Self::update).
    pub fn input(&self) -> Ref<'_, InputHandler> {
        self.input_handler.borrow()
    }

    /// Keep the game's save data (`save_data` / `load_data` in scripts) in
    /// `save_data`, e.g. [`SaveData::for_game`]. A new runtime keeps it in
    /// memory only.
    pub fn set_save_data(&mut self, save_data: SaveData) {
        *self.save_data.borrow_mut() = save_data;
    }

    /// The game's save data, e.g. to clear it.
    pub fn save_data(&self) -> RefMut<'_, SaveData> {
        self.save_data.borrow_mut()
    }

    /// The scripting session, e.g. to inspect script state in tests.
    pub fn scripting(&self) -> &LuaScripting {
        &self.lua_scripting
    }
}
