use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};  // Use KeyCode for handling keyboard input

pub struct InputHandler {
    pub key_pressed: Option<KeyCode>,  // Stores the last key pressed
}

impl InputHandler {
    pub fn new() -> Self {
        InputHandler {
            key_pressed: None,
        }
    }

    pub fn handle_input(&mut self, event: &Event<()>) {
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } = event
        {
            match event.state {
                ElementState::Pressed => {
                    if let PhysicalKey::Code(keycode) = event.physical_key {
                        self.key_pressed = Some(keycode);
                    }
                }
                ElementState::Released => {
                    self.key_pressed = None;
                }
            }
        }
    }
}