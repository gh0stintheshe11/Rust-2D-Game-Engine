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
//! The project files (`scenes/`, `assets/`) are looked up next to the
//! executable (where the editor's Build command copies them), then in the
//! working directory (so `cargo run` works inside the project folder).
//!
//! The scene's default camera is scaled to fit the window. After a script
//! calls `end_game()`, Enter or R restarts the game.

use crate::audio::AudioEngine;
use crate::input::InputHandler;
use crate::physics::PhysicsEngine;
use crate::project::ProjectManager;
use crate::render::RenderEngine;
use crate::runtime::{CameraMode, GameRuntime, RuntimeState};
use crate::scene::SceneManager;
use eframe::egui;
use std::env;
use std::path::{Path, PathBuf};

/// Simulation rate of exported games, in steps per second.
const TARGET_FPS: u32 = 60;

/// Open the game window and run the project.
pub fn run(title: &str) -> eframe::Result<()> {
    let project_dir = find_project_dir();
    if let Err(e) = env::set_current_dir(&project_dir) {
        eprintln!("Can't enter {}: {}", project_dir.display(), e);
    }
    ProjectManager::set_project_path(project_dir.to_string_lossy().to_string());

    let mut runtime = GameRuntime::new(
        SceneManager::new(),
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::new(),
        TARGET_FPS,
    );
    runtime.set_camera_mode(CameraMode::Game);

    let mut window_size = [800.0, 600.0];
    let error = match ProjectManager::load_scene_hierarchy(&project_dir) {
        Ok(scene_manager) => {
            if let Some(size) = camera_frame_size(&scene_manager) {
                window_size = size;
            }
            runtime.set_scene_manager(scene_manager);
            runtime.run().err()
        }
        Err(e) => Some(format!("Failed to load the game's scenes: {}", e)),
    };
    if let Some(error) = &error {
        eprintln!("{}", error);
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(window_size),
        ..Default::default()
    };
    eframe::run_native(
        title,
        native_options,
        Box::new(move |_cc| Ok(Box::new(PlayerApp { runtime, error }))),
    )
}

/// The directory holding the game's `scenes/` folder: next to the
/// executable, else the working directory, else the executable's directory
/// (which then fails to load with a clear error).
fn find_project_dir() -> PathBuf {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let candidates = exe_dir.iter().cloned().chain(env::current_dir().ok());
    for dir in candidates {
        if dir.join("scenes").join("scene_manager.json").is_file() {
            return dir;
        }
    }
    exe_dir.unwrap_or_else(|| PathBuf::from("."))
}

/// Size of the active scene's camera frame, used as the initial window size.
fn camera_frame_size(scene_manager: &SceneManager) -> Option<[f32; 2]> {
    let scene = scene_manager.get_active_scene()?;
    let camera = scene.get_entity(scene.default_camera?).ok()?;
    let zoom = camera.get_camera_zoom();
    let size = [
        camera.get_camera_width() / zoom,
        camera.get_camera_height() / zoom,
    ];
    (size[0] >= 1.0 && size[1] >= 1.0 && size.iter().all(|v| v.is_finite())).then_some(size)
}

struct PlayerApp {
    runtime: GameRuntime,
    /// Shown instead of the game when it couldn't start
    error: Option<String>,
}

impl eframe::App for PlayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();

        if let Some(error) = &self.error {
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_gray(20));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                error,
                egui::FontId::proportional(16.0),
                egui::Color32::LIGHT_RED,
            );
            return;
        }

        self.runtime.update(&ctx, ui, rect);

        if self.runtime.get_state() == RuntimeState::Ended {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Game over - press Enter to play again",
                egui::FontId::proportional(24.0),
                egui::Color32::WHITE,
            );
            let restart =
                ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::R));
            if restart {
                self.error = self.runtime.restart().err();
            }
        }
    }
}
