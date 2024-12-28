use rust_2d_game_engine::{
    eframe,
    ecs::SceneManager,
    render_engine::RenderEngine,
    input_handler::InputHandler,
    physics_engine::PhysicsEngine,
    audio_engine::AudioEngine,
    game_runtime::{GameRuntime, RuntimeState},
    project_manager::ProjectManager,
};
use std::path::{Path, PathBuf};
use eframe::egui;
use std::env;

fn main() -> eframe::Result<()> {
    // Set up panic handler for safety
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Game panicked: {}", panic_info);
    }));

    println!("Starting...");

    // Set the path to the current executable
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path.parent().expect("Failed to get executable directory");

    env::set_current_dir(exe_dir).expect("Failed to set working directory");
    println!("Set working directory to: {:?}", exe_dir);

    let project_path: PathBuf = exe_dir.to_path_buf();
    println!("Resolved project path: {:?}", project_path);

    let mut game_runtime = GameRuntime::new(
        SceneManager::new(),
        PhysicsEngine::new(),
        RenderEngine::new(),
        InputHandler::new(),
        AudioEngine::new(),
        60, // target fps
    );

    let mut camera_width = 800.0;
    let mut camera_height = 600.0;

    ProjectManager::set_project_path(project_path.to_string_lossy().to_string());
    let scene_manager = match ProjectManager::load_scene_hierarchy(&project_path) {
        Ok(manager) => manager,
        Err(e) => {
            println!("Failed to load scene hierarchy: {}", e);
            SceneManager::new()
        }
    };

    if let Some(scene) = scene_manager.get_active_scene() {
        if let Some(camera_id) = scene.default_camera {
            if let Ok(camera_entity) = scene.get_entity(camera_id) {
                camera_width = camera_entity.get_camera_width();
                camera_height = camera_entity.get_camera_height();
            }
        }
    }

    // Set initial window size using `NativeOptions`
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([camera_width, camera_height])
            .with_min_inner_size([camera_width, camera_height])
            .with_maximized(true),
        ..Default::default()
    };

    game_runtime.set_scene_manager(scene_manager.clone());
    game_runtime.run();

    eframe::run_native(
        "Game Window",
        native_options,
        Box::new(|cc| {

            Ok(Box::new(MyApp {
                game_runtime,
                camera_width,
                camera_height,
            }))
        }),
    )
}

struct MyApp {
    game_runtime: GameRuntime,
    camera_width: f32,
    camera_height: f32,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            let game_rect = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(self.camera_width, self.camera_height),
            );

            let game_view_rect = ui.available_rect_before_wrap();
            self.game_runtime.update(ctx, ui, game_rect);

        });

        ctx.request_repaint();
    }
}
