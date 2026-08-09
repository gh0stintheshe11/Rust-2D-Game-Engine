use egui::{Key, PointerButton};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum InputContext {
    EngineUI,
    Game,
}

#[derive(Clone)]
pub struct InputHandler {
    context: InputContext,
    keys_pressed: HashSet<Key>,
    keys_just_pressed: HashSet<Key>,
    mouse_buttons: Vec<PointerButton>,
    mouse_pos: egui::Pos2,
    prev_mouse_pos: egui::Pos2,
    scroll_delta: egui::Vec2,
    modifiers: egui::Modifiers,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            context: InputContext::EngineUI,
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            mouse_buttons: Vec::new(),
            mouse_pos: egui::pos2(0.0, 0.0),
            prev_mouse_pos: egui::pos2(0.0, 0.0),
            scroll_delta: egui::vec2(0.0, 0.0),
            modifiers: egui::Modifiers::default(),
        }
    }

    pub fn get_context(&self) -> &InputContext {
        &self.context
    }

    pub fn set_context(&mut self, context: InputContext) {
        println!("Input context changed to: {:?}", context);
        self.context = context;
    }

    pub fn handle_input(&mut self, input: &egui::InputState) {
        // Store modifiers state
        self.modifiers = input.modifiers;

        // Track which keys were just pressed this frame
        let old_keys = self.keys_pressed.clone();
        
        // Update key states
        self.keys_pressed.clear();
        self.keys_just_pressed.clear();
        
        input.keys_down.iter().for_each(|key| {
            self.keys_pressed.insert(*key);
            if !old_keys.contains(key) {
                self.keys_just_pressed.insert(*key);
            }
        });

        // Update mouse position
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos = input.pointer.hover_pos().unwrap_or(self.mouse_pos);

        // Update mouse buttons
        self.mouse_buttons.clear();
        if input.pointer.middle_down() {
            self.mouse_buttons.push(PointerButton::Middle);
        }
        if input.pointer.primary_down() {
            self.mouse_buttons.push(PointerButton::Primary);
        }
        if input.pointer.secondary_down() {
            self.mouse_buttons.push(PointerButton::Secondary);
        }

        // Update scroll
        self.scroll_delta = input.raw_scroll_delta;
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: Key) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: PointerButton) -> bool {
        self.mouse_buttons.contains(&button)
    }

    pub fn get_mouse_pos(&self) -> egui::Pos2 {
        self.mouse_pos
    }

    pub fn get_mouse_delta(&self) -> Option<egui::Vec2> {
        Some(egui::vec2(
            self.mouse_pos.x - self.prev_mouse_pos.x,
            self.mouse_pos.y - self.prev_mouse_pos.y,
        ))
    }

    pub fn get_scroll_delta(&self) -> Option<egui::Vec2> {
        if self.scroll_delta.x != 0.0 || self.scroll_delta.y != 0.0 {
            Some(self.scroll_delta)
        } else {
            None
        }
    }

    pub fn get_all_active_inputs(&self) -> Vec<String> {
        let mut all_inputs = Vec::new();
        
        // Add modifier keys if pressed
        if self.modifiers.ctrl {
            all_inputs.push("Ctrl".to_string());
        }
        if self.modifiers.shift {
            all_inputs.push("Shift".to_string());
        }
        if self.modifiers.alt {
            all_inputs.push("Alt".to_string());
        }
        if self.modifiers.command {
            all_inputs.push("Cmd".to_string());
        }
        
        // Add all pressed keyboard keys
        for key in &self.keys_pressed {
            all_inputs.push(format!("{:?}", key));
        }
        
        // Add all pressed mouse buttons
        for button in &self.mouse_buttons {
            all_inputs.push(format!("{:?}", button));
        }
        
        all_inputs
    }
}
