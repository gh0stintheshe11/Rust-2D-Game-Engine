use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::KeyCode;  // Use KeyCode for handling keyboard input

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
        match event {
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                if let winit::keyboard::PhysicalKey::Code(keycode) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.key_pressed = Some(keycode);
                        }
                        ElementState::Released => {
                            self.key_pressed = None;
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
