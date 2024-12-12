use egui::{Key, PointerButton};
use std::collections::HashSet;

pub enum InputContext {
    Editor,
    Game,
}

pub struct InputHandler {
    context: InputContext,
    keys_pressed: HashSet<Key>,
    mouse_buttons: Vec<PointerButton>,
    mouse_pos: egui::Pos2,
    prev_mouse_pos: egui::Pos2,
    scroll_delta: egui::Vec2,
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            context: InputContext::Editor,
            keys_pressed: HashSet::new(),
            mouse_buttons: Vec::new(),
            mouse_pos: egui::pos2(0.0, 0.0),
            prev_mouse_pos: egui::pos2(0.0, 0.0),
            scroll_delta: egui::vec2(0.0, 0.0),
        }
    }

    pub fn set_context(&mut self, context: InputContext) {
        self.context = context;
    }

    pub fn handle_input(&mut self, input: &egui::InputState) {
        // Update key states
        self.keys_pressed.clear();
        input.keys_down.iter().for_each(|key| {
            self.keys_pressed.insert(*key);
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
}
