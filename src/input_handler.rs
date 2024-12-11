use std::collections::HashSet;
use egui::Key;

pub enum InputContext {
    Editor,
    Game,
}

pub struct InputHandler {
    context: InputContext,
    keys_pressed: HashSet<Key>,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            context: InputContext::Editor,
            keys_pressed: HashSet::new(),
        }
    }

    pub fn set_context(&mut self, context: InputContext) {
        self.context = context;
    }

    pub fn handle_input(&mut self, input: &egui::InputState) {
        self.keys_pressed.clear();
        input.keys_down.iter().for_each(|key| {
            self.keys_pressed.insert(*key);
        });
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }
}