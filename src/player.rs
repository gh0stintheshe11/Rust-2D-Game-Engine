//! Standalone game window used by exported games.
//!
//! A game project's `main.rs` is a single call into this module:
//!
//! ```no_run
//! fn main() -> rust_2d_game_engine::eframe::Result<()> {
//!     rust_2d_game_engine::player::run("My Game")
//! }
//! ```
//!
//! The project files (`scenes/`, `assets/`) are expected next to the
//! executable; the editor's Build command copies them there.

use crate::audio::AudioEngine;
use crate::input::InputHandler;
use crate::physics::PhysicsEngine;
use crate::project::ProjectManager;
use crate::render::RenderEngine;
use crate::runtime::GameRuntime;
use crate::scene::SceneManager;
use eframe::egui;
use std::env;

/// Simulation rate of exported games, in steps per second.
const TARGET_FPS: u32 = 60;

/// Open the game window and run the project located next to the executable.
pub fn run(title: &str) -> eframe::Result<()> {
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path
        .parent()
        .expect("Failed to get executable directory")
        .to_path_buf();
    env::set_current_dir(&exe_dir).expect("Failed to set working directory");

    let mut game_runtime = GameRuntime::new(
        SceneManager::new(),
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::new(),
        TARGET_FPS,
    );

    ProjectManager::set_project_path(exe_dir.to_string_lossy().to_string());
    let scene_manager = match ProjectManager::load_scene_hierarchy(&exe_dir) {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to load scene hierarchy: {}", e);
            SceneManager::new()
        }
    };

    let mut camera_width = 800.0;
    let mut camera_height = 600.0;
    if let Some(scene) = scene_manager.get_active_scene()
        && let Some(camera_id) = scene.default_camera
        && let Ok(camera_entity) = scene.get_entity(camera_id)
    {
        camera_width = camera_entity.get_camera_width();
        camera_height = camera_entity.get_camera_height();
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([camera_width, camera_height])
            .with_min_inner_size([camera_width, camera_height])
            .with_maximized(true),
        ..Default::default()
    };

    game_runtime.set_scene_manager(scene_manager);
    if let Err(e) = game_runtime.run() {
        eprintln!("Failed to start game: {}", e);
    }

    eframe::run_native(
        title,
        native_options,
        Box::new(move |_cc| {
            Ok(Box::new(PlayerApp {
                game_runtime,
                camera_width,
                camera_height,
            }))
        }),
    )
}

struct PlayerApp {
    game_runtime: GameRuntime,
    camera_width: f32,
    camera_height: f32,
}

impl eframe::App for PlayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let game_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(self.camera_width, self.camera_height),
        );
        self.game_runtime.update(&ctx, ui, game_rect);
        ctx.request_repaint();
    }
}
