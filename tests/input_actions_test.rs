//! Input actions: bindings written as text, and actions' state from keys,
//! mouse buttons and gamepads, frame after frame.

use egui::{Event, Key, Modifiers, PointerButton, RawInput, pos2};
use rust_2d_game_engine::input::{
    ActionState, AxisDirection, GamepadAxis, GamepadButton, GamepadEvent, InputAccess,
    InputActions, InputBinding, InputHandler, MAX_ACTION_NAME_CHARS,
};

/// Run one egui frame with `events`, hand its input and `pad_events` to
/// `handler`, and work out its actions, as the runtime does every frame.
fn frame(
    ctx: &egui::Context,
    handler: &mut InputHandler,
    events: Vec<Event>,
    pad_events: Vec<GamepadEvent>,
) {
    let input = RawInput {
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        ui.ctx()
            .input(|i| handler.handle_input_with(i, InputAccess::ALL));
    });
    output.textures_delta.clear();
    handler.handle_gamepad_events(pad_events);
    handler.update_actions();
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

fn binding(text: &str) -> InputBinding {
    InputBinding::parse(text).unwrap()
}

fn handler_with(actions: &[(&str, &[&str])]) -> InputHandler {
    let mut set = InputActions::new();
    for (name, bindings) in actions {
        set.set(name, bindings.iter().map(|text| binding(text)).collect())
            .unwrap();
    }
    let mut handler = InputHandler::new();
    handler.set_actions(set);
    handler
}

/// (pressed, just pressed, just released)
fn state(handler: &InputHandler, action: &str) -> (bool, bool, bool) {
    let ActionState {
        pressed,
        just_pressed,
        just_released,
        ..
    } = handler.action(action).unwrap();
    (pressed, just_pressed, just_released)
}

#[test]
fn test_bindings_are_written_and_read_as_text() {
    use AxisDirection::{Negative, Positive};
    let cases = [
        ("key:Space", InputBinding::Key(Key::Space)),
        ("key:ArrowLeft", InputBinding::Key(Key::ArrowLeft)),
        ("mouse:left", InputBinding::Mouse(PointerButton::Primary)),
        ("mouse:extra2", InputBinding::Mouse(PointerButton::Extra2)),
        (
            "pad:a",
            InputBinding::GamepadButton {
                button: GamepadButton::South,
                pad: None,
            },
        ),
        (
            "pad2:start",
            InputBinding::GamepadButton {
                button: GamepadButton::Start,
                pad: Some(1),
            },
        ),
        (
            "pad:left_y-",
            InputBinding::GamepadAxis {
                axis: GamepadAxis::LeftStickY,
                direction: Negative,
                pad: None,
            },
        ),
        (
            "pad1:rt+",
            InputBinding::GamepadAxis {
                axis: GamepadAxis::RightTrigger,
                direction: Positive,
                pad: Some(0),
            },
        ),
        (
            "pad:lt",
            InputBinding::GamepadButton {
                button: GamepadButton::LeftTrigger,
                pad: None,
            },
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(binding(text), expected, "{}", text);
        assert_eq!(expected.to_string(), text, "written back the same way");
    }
    // Names ignore case and spaces around them; other names of a button are
    // accepted and written as its main one
    assert_eq!(binding(" KEY: space "), InputBinding::Key(Key::Space));
    assert_eq!(binding("pad:cross").to_string(), "pad:a");

    for text in [
        "Space",
        "key:Spcae",
        "mouse:wheel",
        "joystick:a",
        "pad0:a",
        "padx:a",
        "pad:z",
        "pad:left_z+",
        "pad:lt-",
        "",
    ] {
        let error = InputBinding::parse(text).unwrap_err();
        assert!(error.contains(&format!("\"{}\"", text)), "{}", error);
    }

    assert_eq!(binding("key:Space").label(), "Space");
    assert_eq!(
        binding("key:Left").to_string(),
        "key:ArrowLeft",
        "egui's name too"
    );
    assert_eq!(binding("key:ArrowLeft").label(), "Left arrow");
    assert_eq!(binding("pad:a").label(), "Gamepad A");
    assert_eq!(
        binding("pad2:left_x+").label(),
        "Gamepad 2 left stick right"
    );
    assert_eq!(binding("mouse:right").label(), "Right mouse button");
}

#[test]
fn test_actions_follow_their_keys() {
    let ctx = egui::Context::default();
    let mut handler = handler_with(&[("jump", &["key:Space", "key:W"]), ("left", &["key:A"])]);

    frame(&ctx, &mut handler, vec![key(Key::Space, true)], vec![]);
    assert_eq!(state(&handler, "jump"), (true, true, false));
    assert_eq!(state(&handler, "left"), (false, false, false));
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(state(&handler, "jump"), (true, false, false), "held");
    // Another of its keys, while one is held, doesn't press it again...
    frame(&ctx, &mut handler, vec![key(Key::W, true)], vec![]);
    assert_eq!(state(&handler, "jump"), (true, false, false));
    // ...and it's held until the last of them is released
    frame(&ctx, &mut handler, vec![key(Key::Space, false)], vec![]);
    assert_eq!(state(&handler, "jump"), (true, false, false));
    frame(&ctx, &mut handler, vec![key(Key::W, false)], vec![]);
    assert_eq!(state(&handler, "jump"), (false, false, true));
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(state(&handler, "jump"), (false, false, false));

    // A tap within one frame is seen
    frame(
        &ctx,
        &mut handler,
        vec![key(Key::Space, true), key(Key::Space, false)],
        vec![],
    );
    assert_eq!(state(&handler, "jump"), (false, true, true));
    assert_eq!(handler.action("jump").unwrap().strength, 0.0);

    assert_eq!(handler.action("fly"), None, "no such action");
    assert_eq!(handler.action_axis("left", "fly"), None);
}

#[test]
fn test_gamepad_buttons_on_any_or_one_gamepad() {
    let ctx = egui::Context::default();
    let mut handler = handler_with(&[("any", &["pad:a"]), ("second", &["pad2:a"])]);
    let press = |gamepad, pressed| {
        let button = GamepadButton::South;
        if pressed {
            GamepadEvent::ButtonPressed { gamepad, button }
        } else {
            GamepadEvent::ButtonReleased { gamepad, button }
        }
    };
    let connect = |gamepad| GamepadEvent::Connected { gamepad };

    // Pads 40 and 41 connect: the first and the second gamepad
    frame(&ctx, &mut handler, vec![], vec![connect(40), connect(41)]);
    frame(&ctx, &mut handler, vec![], vec![press(40, true)]);
    assert_eq!(state(&handler, "any"), (true, true, false));
    assert_eq!(state(&handler, "second"), (false, false, false));
    frame(
        &ctx,
        &mut handler,
        vec![],
        vec![press(40, false), press(41, true)],
    );
    assert_eq!(state(&handler, "any"), (true, false, false), "still held");
    assert_eq!(state(&handler, "second"), (true, true, false));
}

#[test]
fn test_sticks_and_triggers_are_analog() {
    let ctx = egui::Context::default();
    let mut handler = handler_with(&[
        ("left", &["key:A", "pad:left_x-"]),
        ("right", &["pad:left_x+"]),
        ("brake", &["pad:lt+"]),
    ]);
    let stick = |value| GamepadEvent::AxisMoved {
        gamepad: 0,
        axis: GamepadAxis::LeftStickX,
        value,
    };

    // Pushed a little: some strength, not pressed
    frame(&ctx, &mut handler, vec![], vec![stick(-0.3)]);
    let left = handler.action("left").unwrap();
    assert!(
        left.strength > 0.1 && left.strength < 0.5,
        "{}",
        left.strength
    );
    assert!(!left.pressed && !left.just_pressed);
    // Pushed far: pressed, once
    frame(&ctx, &mut handler, vec![], vec![stick(-0.9)]);
    assert_eq!(state(&handler, "left"), (true, true, false));
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(state(&handler, "left"), (true, false, false));
    let axis = handler.action_axis("left", "right").unwrap();
    assert!(axis < -0.8, "{}", axis);
    // A key at full strength beats the stick
    frame(&ctx, &mut handler, vec![key(Key::A, true)], vec![]);
    assert_eq!(handler.action("left").unwrap().strength, 1.0);
    frame(
        &ctx,
        &mut handler,
        vec![key(Key::A, false)],
        vec![stick(0.0)],
    );
    assert_eq!(state(&handler, "left"), (false, false, true));
    assert_eq!(state(&handler, "right"), (false, false, false));

    let trigger = |value| GamepadEvent::AxisMoved {
        gamepad: 0,
        axis: GamepadAxis::LeftTrigger,
        value,
    };
    frame(&ctx, &mut handler, vec![], vec![trigger(1.0)]);
    assert_eq!(state(&handler, "brake"), (true, true, false));
    assert_eq!(handler.action("brake").unwrap().strength, 1.0);
}

#[test]
fn test_what_went_down_as_a_binding() {
    let ctx = egui::Context::default();
    let mut handler = handler_with(&[]);
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(handler.just_pressed_binding(), None);

    frame(&ctx, &mut handler, vec![key(Key::Q, true)], vec![]);
    assert_eq!(handler.just_pressed_binding(), Some(binding("key:Q")));
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(
        handler.just_pressed_binding(),
        None,
        "held, not pressed again"
    );

    let click = Event::PointerButton {
        pos: pos2(5.0, 5.0),
        button: PointerButton::Secondary,
        pressed: true,
        modifiers: Modifiers::NONE,
    };
    frame(&ctx, &mut handler, vec![click], vec![]);
    assert_eq!(handler.just_pressed_binding(), Some(binding("mouse:right")));

    let pad = vec![
        GamepadEvent::Connected { gamepad: 3 },
        GamepadEvent::ButtonPressed {
            gamepad: 3,
            button: GamepadButton::North,
        },
    ];
    frame(&ctx, &mut handler, vec![], pad);
    assert_eq!(handler.just_pressed_binding(), Some(binding("pad:y")));

    // A stick only once it's pushed past halfway
    let stick = |value| {
        vec![GamepadEvent::AxisMoved {
            gamepad: 3,
            axis: GamepadAxis::RightStickY,
            value,
        }]
    };
    frame(&ctx, &mut handler, vec![], stick(0.3));
    assert_eq!(handler.just_pressed_binding(), None);
    frame(&ctx, &mut handler, vec![], stick(0.9));
    assert_eq!(
        handler.just_pressed_binding(),
        Some(binding("pad:right_y+"))
    );
    frame(&ctx, &mut handler, vec![], stick(0.95));
    assert_eq!(handler.just_pressed_binding(), None);
}

#[test]
fn test_actions_are_named_kept_in_order_and_stored_as_text() {
    let mut actions = InputActions::new();
    actions.set("jump", vec![binding("key:Space")]).unwrap();
    actions
        .set("run", vec![binding("key:ShiftLeft"), binding("pad:rb")])
        .unwrap();
    actions.set("aim", vec![]).unwrap();
    for bad in ["", "  ", " jump", "jump "] {
        assert!(actions.set(bad, vec![]).is_err(), "{:?}", bad);
    }
    assert!(
        actions
            .set(&"x".repeat(MAX_ACTION_NAME_CHARS + 1), vec![])
            .is_err()
    );

    // Renaming keeps the place
    actions.rename("run", "dash").unwrap();
    assert!(actions.rename("dash", "jump").is_err(), "taken");
    assert!(actions.rename("fly", "soar").is_err(), "no such action");
    let names: Vec<&str> = actions.iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["jump", "dash", "aim"]);

    let json = serde_json::to_string(&actions).unwrap();
    assert_eq!(
        json,
        r#"{"jump":["key:Space"],"dash":["key:ShiftLeft","pad:rb"],"aim":[]}"#
    );
    let read: InputActions = serde_json::from_str(&json).unwrap();
    assert_eq!(read, actions);
    let order: Vec<&str> = read.iter().map(|(name, _)| name).collect();
    assert_eq!(order, ["jump", "dash", "aim"]);

    let error = serde_json::from_value::<InputActions>(serde_json::json!({"jump": ["key:Spcae"]}))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("jump") && error.contains("key:Spcae"),
        "{}",
        error
    );
    assert!(serde_json::from_value::<InputActions>(serde_json::json!({" jump": []})).is_err());

    assert!(actions.remove("dash"));
    assert!(!actions.remove("dash"));
    assert_eq!(actions.len(), 2);

    // New projects' actions survive the trip too
    let defaults = InputActions::defaults();
    let json = serde_json::to_string(&defaults).unwrap();
    assert_eq!(
        serde_json::from_str::<InputActions>(&json).unwrap(),
        defaults
    );
    let names: Vec<&str> = defaults.iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["left", "right", "up", "down", "jump", "pause"]);
}

#[test]
fn test_actions_that_stay_keep_their_state() {
    let ctx = egui::Context::default();
    let mut handler = handler_with(&[("jump", &["key:Space"]), ("duck", &["key:S"])]);
    frame(&ctx, &mut handler, vec![key(Key::Space, true)], vec![]);

    let mut actions = handler.actions().clone();
    actions.remove("duck");
    actions.set("dash", vec![binding("key:D")]).unwrap();
    handler.set_actions(actions);
    // Still held: not pressed anew
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(state(&handler, "jump"), (true, false, false));
    assert_eq!(handler.action("duck"), None);
    assert_eq!(state(&handler, "dash"), (false, false, false));

    // Rebinding while held
    handler
        .set_action_bindings("jump", vec![binding("key:Enter")])
        .unwrap();
    frame(&ctx, &mut handler, vec![], vec![]);
    assert_eq!(state(&handler, "jump"), (false, false, true));
    assert!(handler.set_action_bindings("", vec![]).is_err());
}

#[test]
fn test_actions_can_need_a_stick_pushed_further() {
    let ctx = egui::Context::default();
    let mut handler = handler_with(&[("down", &["pad:left_y+"])]);
    let mut actions = handler.actions().clone();
    for bad in [0.0, -0.5, 1.5, f32::NAN] {
        assert!(actions.set_threshold("down", bad).is_err(), "{}", bad);
    }
    assert!(actions.set_threshold("up", 0.7).is_err(), "no such action");
    actions.set_threshold("down", 0.7).unwrap();
    assert_eq!(actions.threshold("down"), Some(0.7));
    handler.set_actions(actions.clone());

    let stick = |x, y| {
        vec![
            GamepadEvent::AxisMoved {
                gamepad: 0,
                axis: GamepadAxis::LeftStickX,
                value: x,
            },
            GamepadEvent::AxisMoved {
                gamepad: 0,
                axis: GamepadAxis::LeftStickY,
                value: y,
            },
        ]
    };
    // Pushed diagonally (running down a slope, say): not down yet
    frame(&ctx, &mut handler, vec![], stick(0.7, 0.7));
    let down = handler.action("down").unwrap();
    assert!(down.strength > 0.5 && !down.pressed, "{:?}", down);
    frame(&ctx, &mut handler, vec![], stick(0.0, 1.0));
    assert_eq!(state(&handler, "down"), (true, true, false));

    // Kept with the action in project files, only when it isn't the default
    actions.set("jump", vec![binding("key:Space")]).unwrap();
    let json = serde_json::to_string(&actions).unwrap();
    assert_eq!(
        json,
        r#"{"down":{"bindings":["pad:left_y+"],"threshold":0.7},"jump":["key:Space"]}"#
    );
    assert_eq!(
        serde_json::from_str::<InputActions>(&json).unwrap(),
        actions
    );
    let error = serde_json::from_str::<InputActions>(
        r#"{"down": {"bindings": ["pad:left_y+"], "threshold": 2}}"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("down") && error.contains("threshold"),
        "{}",
        error
    );
}
