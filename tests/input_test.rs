//! InputHandler: presses, releases and taps from egui's per-frame input, and
//! input withheld from the game.

use egui::{Event, Key, Modifiers, PointerButton, RawInput, pos2, vec2};
use rust_2d_game_engine::input::{InputAccess, InputHandler};

/// Run one egui frame with `events` and hand its input to `handler`.
fn frame(ctx: &egui::Context, handler: &mut InputHandler, events: Vec<Event>, access: InputAccess) {
    let input = RawInput {
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        ui.ctx().input(|i| handler.handle_input_with(i, access));
    });
    // Nothing renders here; textures don't need uploading
    output.textures_delta.clear();
}

fn key(key: Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn click(button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos: pos2(10.0, 10.0),
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// (pressed, just pressed, just released)
fn key_state(handler: &InputHandler, key: Key) -> (bool, bool, bool) {
    (
        handler.is_key_pressed(key),
        handler.is_key_just_pressed(key),
        handler.is_key_just_released(key),
    )
}

fn button_state(handler: &InputHandler, button: PointerButton) -> (bool, bool, bool) {
    (
        handler.is_mouse_button_pressed(button),
        handler.is_mouse_button_just_pressed(button),
        handler.is_mouse_button_just_released(button),
    )
}

const ALL: InputAccess = InputAccess::ALL;

#[test]
fn test_holding_a_key() {
    let ctx = egui::Context::default();
    let mut input = InputHandler::new();

    frame(&ctx, &mut input, vec![key(Key::A, true)], ALL);
    assert_eq!(key_state(&input, Key::A), (true, true, false));
    frame(&ctx, &mut input, vec![], ALL);
    assert_eq!(key_state(&input, Key::A), (true, false, false), "held");
    // Key repeats aren't new presses
    let repeat = Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: Modifiers::NONE,
    };
    frame(&ctx, &mut input, vec![repeat], ALL);
    assert_eq!(key_state(&input, Key::A), (true, false, false));
    frame(&ctx, &mut input, vec![key(Key::A, false)], ALL);
    assert_eq!(key_state(&input, Key::A), (false, false, true));
    frame(&ctx, &mut input, vec![], ALL);
    assert_eq!(key_state(&input, Key::A), (false, false, false));
}

#[test]
fn test_a_key_tapped_within_one_frame_is_seen() {
    let ctx = egui::Context::default();
    let mut input = InputHandler::new();
    frame(
        &ctx,
        &mut input,
        vec![key(Key::Space, true), key(Key::Space, false)],
        ALL,
    );
    assert_eq!(
        key_state(&input, Key::Space),
        (false, true, true),
        "pressed and released between two frames"
    );
}

#[test]
fn test_mouse_buttons() {
    let ctx = egui::Context::default();
    let mut input = InputHandler::new();
    let moved = Event::PointerMoved(pos2(10.0, 10.0));

    frame(
        &ctx,
        &mut input,
        vec![moved.clone(), click(PointerButton::Primary, true)],
        ALL,
    );
    assert_eq!(
        button_state(&input, PointerButton::Primary),
        (true, true, false)
    );
    assert_eq!(input.get_mouse_pos(), pos2(10.0, 10.0));
    frame(&ctx, &mut input, vec![], ALL);
    assert_eq!(
        button_state(&input, PointerButton::Primary),
        (true, false, false)
    );
    frame(
        &ctx,
        &mut input,
        vec![click(PointerButton::Primary, false)],
        ALL,
    );
    assert_eq!(
        button_state(&input, PointerButton::Primary),
        (false, false, true)
    );

    // A quick click within one frame
    frame(
        &ctx,
        &mut input,
        vec![
            click(PointerButton::Secondary, true),
            click(PointerButton::Secondary, false),
        ],
        ALL,
    );
    assert_eq!(
        button_state(&input, PointerButton::Secondary),
        (false, true, true)
    );
}

#[test]
fn test_withheld_input_counts_as_released() {
    let ctx = egui::Context::default();
    let mut input = InputHandler::new();
    let no_keyboard = InputAccess {
        keyboard: false,
        pointer: true,
    };
    let no_pointer = InputAccess {
        keyboard: true,
        pointer: false,
    };

    frame(&ctx, &mut input, vec![key(Key::A, true)], no_keyboard);
    assert_eq!(key_state(&input, Key::A), (false, false, false));
    // Given the keyboard back while A is still held: a new press
    frame(&ctx, &mut input, vec![], ALL);
    assert_eq!(key_state(&input, Key::A), (true, true, false));
    // Taken away while held: a release
    frame(&ctx, &mut input, vec![], no_keyboard);
    assert_eq!(key_state(&input, Key::A), (false, false, true));

    let events = vec![
        click(PointerButton::Primary, true),
        Event::PointerMoved(pos2(30.0, 40.0)),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 50.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        },
    ];
    frame(&ctx, &mut input, events, no_pointer);
    assert_eq!(
        button_state(&input, PointerButton::Primary),
        (false, false, false)
    );
    assert_eq!(input.get_scroll_delta(), None);
    assert_eq!(
        input.get_mouse_pos(),
        pos2(30.0, 40.0),
        "the position is still tracked"
    );
}

mod gamepads {
    use rust_2d_game_engine::input::{
        GamepadAxis, GamepadButton, GamepadEvent, InputHandler, STICK_DEADZONE, parse_gamepad_axis,
        parse_gamepad_button,
    };

    fn press(gamepad: usize, button: GamepadButton) -> GamepadEvent {
        GamepadEvent::ButtonPressed { gamepad, button }
    }

    fn release(gamepad: usize, button: GamepadButton) -> GamepadEvent {
        GamepadEvent::ButtonReleased { gamepad, button }
    }

    fn axis(gamepad: usize, axis: GamepadAxis, value: f32) -> GamepadEvent {
        GamepadEvent::AxisMoved {
            gamepad,
            axis,
            value,
        }
    }

    #[test]
    fn test_buttons_are_just_pressed_for_one_frame() {
        use GamepadButton::South;
        let mut input = InputHandler::new();
        input.handle_gamepad_events([GamepadEvent::Connected { gamepad: 7 }, press(7, South)]);
        assert!(input.is_gamepad_pressed(South, None));
        assert!(input.is_gamepad_just_pressed(South, Some(0)));
        input.handle_gamepad_events([]);
        assert!(input.is_gamepad_pressed(South, None), "still held");
        assert!(
            !input.is_gamepad_just_pressed(South, None),
            "only for a frame"
        );
        input.handle_gamepad_events([release(7, South)]);
        assert!(!input.is_gamepad_pressed(South, None));
        assert!(input.is_gamepad_just_released(South, None));

        // Pressed and released between two frames
        input.handle_gamepad_events([press(7, South), release(7, South)]);
        assert!(input.is_gamepad_just_pressed(South, None));
        assert!(input.is_gamepad_just_released(South, None));
        assert!(!input.is_gamepad_pressed(South, None));
    }

    #[test]
    fn test_pads_are_numbered_in_connection_order() {
        use GamepadButton::{East, Start};
        let mut input = InputHandler::new();
        // A pad sending input before it's announced counts as connected
        input.handle_gamepad_events([press(3, East), GamepadEvent::Connected { gamepad: 1 }]);
        input.handle_gamepad_events([press(1, Start)]);
        assert_eq!(input.gamepad_count(), 2);
        assert!(input.is_gamepad_pressed(East, Some(0)));
        assert!(!input.is_gamepad_pressed(East, Some(1)));
        assert!(input.is_gamepad_pressed(Start, Some(1)));
        assert!(input.is_gamepad_pressed(Start, None), "any pad");
        assert!(!input.is_gamepad_pressed(Start, Some(5)), "no such pad");

        // The first one leaves: the second becomes the first
        input.handle_gamepad_events([GamepadEvent::Disconnected { gamepad: 3 }]);
        assert_eq!(input.gamepad_count(), 1);
        assert!(!input.is_gamepad_pressed(East, None));
        assert!(input.is_gamepad_pressed(Start, Some(0)));
    }

    #[test]
    fn test_axes_have_dead_zones() {
        use GamepadAxis::{LeftStickX, LeftStickY, LeftTrigger, RightStickX};
        let mut input = InputHandler::new();
        // A worn stick resting slightly off center reads as centered
        input.handle_gamepad_events([axis(0, LeftStickX, 0.1), axis(0, LeftStickY, -0.05)]);
        assert_eq!(input.gamepad_axis(LeftStickX, None), 0.0);
        // All the way: 1; past the dead zone, values start again from 0
        input.handle_gamepad_events([axis(0, LeftStickX, 1.0), axis(0, LeftStickY, 0.0)]);
        assert_eq!(input.gamepad_axis(LeftStickX, None), 1.0);
        let just_past = STICK_DEADZONE + 0.01;
        input.handle_gamepad_events([axis(0, LeftStickX, -just_past)]);
        let value = input.gamepad_axis(LeftStickX, None);
        assert!(value < 0.0 && value > -0.05, "{}", value);
        // Out of range values are clamped
        input.handle_gamepad_events([axis(0, LeftTrigger, 7.0), axis(0, RightStickX, f32::NAN)]);
        assert_eq!(input.gamepad_axis(LeftTrigger, None), 1.0);
        assert_eq!(input.gamepad_axis(RightStickX, None), 0.0);

        // Without a pad number: the first pad where the axis isn't at rest
        input.handle_gamepad_events([axis(1, LeftStickY, 0.8), axis(0, LeftStickX, 0.0)]);
        assert!(input.gamepad_axis(LeftStickY, None) > 0.7);
        assert_eq!(input.gamepad_axis(LeftStickY, Some(0)), 0.0);
    }

    #[test]
    fn test_gamepad_names() {
        assert_eq!(parse_gamepad_button("A"), Some(GamepadButton::South));
        assert_eq!(parse_gamepad_button("cross"), Some(GamepadButton::South));
        assert_eq!(parse_gamepad_button("Start"), Some(GamepadButton::Start));
        assert_eq!(parse_gamepad_button("up"), Some(GamepadButton::DPadUp));
        assert_eq!(parse_gamepad_button("jump"), None);
        assert_eq!(parse_gamepad_axis("LEFT_X"), Some(GamepadAxis::LeftStickX));
        assert_eq!(parse_gamepad_axis("rt"), Some(GamepadAxis::RightTrigger));
        assert_eq!(parse_gamepad_axis("left"), None);
    }
}
