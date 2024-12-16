use rust_2d_game_engine::{
    EngineGui,
    eframe,
    ecs::SceneManager,
};

fn main() -> eframe::Result<()> {
    // Set up panic handler for safety
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Game panicked: {}", panic_info);
    }));

    println!("Starting flappy_bird...");
    
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "flappy_bird",
        native_options,
        Box::new(|cc| {
            // Create engine with default scene manager
            let mut engine = EngineGui::new(cc);
            
            // Scene creation and entity management will be done through the UI
            // You can use the Scene Hierarchy window to:
            // - Create new scenes
            // - Add entities
            // - Configure components
            // - Manage resources
            
            Box::new(engine)
        })
    )
}
