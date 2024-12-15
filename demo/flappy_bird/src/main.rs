use my_game_engine::game_runtime;
use my_game_engine::script_interpreter::LuaScriptEngine;

fn main() {
    println!("Starting flappy_bird...");
    
    // Initialize the game runtime
    let mut runtime = game_runtime::GameRuntime::new();
    
    // Initialize Lua script engine
    let script_engine = LuaScriptEngine::new();
    
    // Load the default scene
    runtime.load_default_scene();
    
    // Start the game loop
    runtime.run();
}
