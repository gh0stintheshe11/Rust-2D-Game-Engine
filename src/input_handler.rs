use std::collections::HashSet;
use winit::keyboard::KeyCode;
use egui::Key;

pub enum InputContext {
    Editor,  // When game is not running
    Game,    // When game is running
}

pub struct InputHandler {
    context: InputContext,
    keys_pressed: HashSet<KeyCode>,
}

impl InputHandler {
    pub fn new() -> Self {
        println!("Creating new InputHandler"); // Debug
        InputHandler {
            context: InputContext::Editor,
            keys_pressed: HashSet::new(),
        }
    }

    pub fn set_context(&mut self, context: InputContext) {
        println!("Setting context"); // Debug
        self.context = context;
        self.keys_pressed.clear();
    }

    // Update game input using egui's input
    pub fn update_game(&mut self, ctx: &egui::Context) {
        println!("Updating game input"); // Debug
        
        // Get input state
        let pressed_keys = ctx.input(|i| i.keys_down.clone());
        
        // Map egui keys to winit keys
        let key_mappings = [
            (Key::W, KeyCode::KeyW),
            (Key::A, KeyCode::KeyA),
            (Key::S, KeyCode::KeyS),
            (Key::D, KeyCode::KeyD),
            (Key::Q, KeyCode::KeyQ),
            (Key::E, KeyCode::KeyE),
            (Key::R, KeyCode::KeyR),
            (Key::F, KeyCode::KeyF),
        ];

        // Clear old keys
        self.keys_pressed.clear();

        // Process each key mapping
        for (egui_key, winit_key) in key_mappings.iter() {
            if pressed_keys.contains(egui_key) {
                println!("Key pressed: {:?}", winit_key); // Debug
                self.keys_pressed.insert(*winit_key);
            }
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        let is_pressed = self.keys_pressed.contains(&key);
        if is_pressed {
            println!("Key {:?} is currently pressed", key); // Debug
        }
        is_pressed
    }
}