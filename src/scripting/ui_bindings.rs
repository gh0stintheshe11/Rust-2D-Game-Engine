//! Bindings drawing game UI over the game for the current frame:
//! rectangles, images and buttons, in the camera frame's pixels like
//! `draw_text`.

use std::cell::RefCell;
use std::rc::Rc;

use egui::{Color32, PointerButton};
use mlua::{Table, Value};

use super::LuaScripting;
use crate::input::InputHandler;
use crate::project::ProjectManager;
use crate::render::{HudButton, HudImage, HudItem, HudRect, parse_color};

/// What a button looks like unless its options say otherwise.
const BUTTON_TEXT_SIZE: f32 = 22.0;
const BUTTON_TEXT_COLOR: Color32 = Color32::WHITE;
const BUTTON_BACKGROUND: Color32 = Color32::from_rgba_unmultiplied_const(46, 52, 70, 230);
const BUTTON_HOVER: Color32 = Color32::from_rgba_unmultiplied_const(66, 76, 102, 240);
const BUTTON_PRESSED: Color32 = Color32::from_rgba_unmultiplied_const(34, 38, 52, 240);
const BUTTON_RADIUS: f32 = 8.0;
const BUTTON_OUTLINE: (Color32, f32) = (
    Color32::from_rgba_unmultiplied_const(255, 255, 255, 64),
    1.5,
);
/// The outline marking the button with the keyboard and gamepad focus.
const BUTTON_FOCUS: (Color32, f32) = (
    Color32::from_rgba_unmultiplied_const(255, 255, 255, 220),
    3.0,
);

/// Width of an outline given only a color.
const DEFAULT_OUTLINE_WIDTH: f32 = 2.0;

fn ui_error(function: &str, message: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::external(format!("{}: {}", function, message))
}

/// A position and size, which must be finite (and the size at least 0).
fn area(
    function: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> mlua::Result<((f32, f32), (f32, f32))> {
    if ![x, y, width, height].iter().all(|v| v.is_finite()) {
        return Err(ui_error(
            function,
            "the position and size must be finite numbers",
        ));
    }
    if width < 0.0 || height < 0.0 {
        return Err(ui_error(
            function,
            "the width and height must be at least 0",
        ));
    }
    Ok(((x, y), (width, height)))
}

/// Go through an options table: `apply` takes each option, and returns
/// whether its value has the right type. Unknown options are errors.
fn read_options(
    function: &str,
    options: Option<&Table>,
    known: &[&str],
    mut apply: impl FnMut(&str, Value) -> mlua::Result<bool>,
) -> mlua::Result<()> {
    let Some(options) = options else {
        return Ok(());
    };
    for pair in options.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::String(key) = key else {
            return Err(ui_error(
                function,
                format!("option names must be strings, got a {}", key.type_name()),
            ));
        };
        let key = key.to_str()?.to_string();
        if !known.contains(&key.as_str()) {
            return Err(ui_error(
                function,
                format!("unknown option '{}' (expected {})", key, known.join(", ")),
            ));
        }
        let type_name = value.type_name();
        if !apply(&key, value)? {
            return Err(ui_error(
                function,
                format!("{} has the wrong type ({})", key, type_name),
            ));
        }
    }
    Ok(())
}

/// A boolean option: `None` when it isn't a boolean.
fn boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Boolean(value) => Some(*value),
        _ => None,
    }
}

/// A string option: `None` when it isn't a string.
fn string(value: &Value) -> mlua::Result<Option<String>> {
    match value {
        Value::String(text) => Ok(Some(text.to_str()?.to_string())),
        _ => Ok(None),
    }
}

/// A color option: `None` when it isn't a string.
fn color(function: &str, value: &Value) -> mlua::Result<Option<Color32>> {
    match value {
        Value::String(text) => parse_color(&text.to_str()?)
            .map(Some)
            .map_err(|e| ui_error(function, e)),
        _ => Ok(None),
    }
}

/// A number option that must be at least 0 (above 0 with `positive`):
/// `None` when it isn't a number.
fn number(function: &str, key: &str, value: &Value, positive: bool) -> mlua::Result<Option<f32>> {
    let number = match value {
        Value::Number(number) => *number as f32,
        Value::Integer(number) => *number as f32,
        _ => return Ok(None),
    };
    let valid = number.is_finite()
        && if positive {
            number > 0.0
        } else {
            number >= 0.0
        };
    if valid {
        Ok(Some(number))
    } else {
        let expected = if positive { "above 0" } else { "at least 0" };
        Err(ui_error(function, format!("{} must be {}", key, expected)))
    }
}

impl LuaScripting {
    pub(crate) fn register_ui_bindings(
        &mut self,
        input_handler: &Rc<RefCell<InputHandler>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // draw_rect(x, y, width, height [, {color = "#ffffff", radius = 0,
        // outline = color, outline_width = 2}]): a filled rectangle over the
        // game for the current frame (panels, health bars)
        let hud = Rc::clone(&self.hud);
        let draw_rect = self.lua.create_function(
            move |_, (x, y, width, height, options): (f32, f32, f32, f32, Option<Table>)| {
                const NAME: &str = "draw_rect";
                let (position, size) = area(NAME, x, y, width, height)?;
                let mut rect = HudRect {
                    position,
                    size,
                    color: Color32::WHITE,
                    radius: 0.0,
                    outline: (Color32::TRANSPARENT, 0.0),
                };
                let mut outline_width = None;
                read_options(
                    NAME,
                    options.as_ref(),
                    &["color", "radius", "outline", "outline_width"],
                    |key, value| {
                        Ok(match key {
                            "color" => color(NAME, &value)?.map(|c| rect.color = c).is_some(),
                            "radius" => number(NAME, key, &value, false)?
                                .map(|r| rect.radius = r)
                                .is_some(),
                            "outline" => color(NAME, &value)?.map(|c| rect.outline.0 = c).is_some(),
                            _ => number(NAME, key, &value, false)?
                                .map(|w| outline_width = Some(w))
                                .is_some(),
                        })
                    },
                )?;
                if rect.outline.0 != Color32::TRANSPARENT {
                    rect.outline.1 = outline_width.unwrap_or(DEFAULT_OUTLINE_WIDTH);
                }
                hud.borrow_mut().push(HudItem::Rect(rect));
                Ok(())
            },
        )?;
        globals.set("draw_rect", draw_rect)?;

        // draw_image(path, x, y [, {width, height, color = "#ffffff",
        // opacity = 1}]): an image file over the game for the current frame
        // (icons, lives). Without width and height it has its own size; with
        // one of them, its proportions.
        let hud = Rc::clone(&self.hud);
        let draw_image = self.lua.create_function(
            move |_, (path, x, y, options): (String, f32, f32, Option<Table>)| {
                const NAME: &str = "draw_image";
                let (position, _) = area(NAME, x, y, 0.0, 0.0)?;
                let (mut width, mut height) = (None, None);
                let (mut tint, mut opacity) = (Color32::WHITE, 1.0);
                read_options(
                    NAME,
                    options.as_ref(),
                    &["width", "height", "color", "opacity"],
                    |key, value| {
                        Ok(match key {
                            "width" => number(NAME, key, &value, true)?
                                .map(|w| width = Some(w))
                                .is_some(),
                            "height" => number(NAME, key, &value, true)?
                                .map(|h| height = Some(h))
                                .is_some(),
                            "color" => color(NAME, &value)?.map(|c| tint = c).is_some(),
                            _ => number(NAME, key, &value, false)?
                                .map(|o| opacity = o.min(1.0))
                                .is_some(),
                        })
                    },
                )?;
                hud.borrow_mut().push(HudItem::Image(HudImage {
                    path: ProjectManager::resolve_asset_path(&path),
                    position,
                    size: (width, height),
                    tint: tint.gamma_multiply(opacity),
                }));
                Ok(())
            },
        )?;
        globals.set("draw_image", draw_image)?;

        // button(label, x, y, width, height [, options]) -> whether it was
        // clicked this frame (pressed and released with the left button on
        // it, or pressed with Enter or gamepad A while it has the focus),
        // and whether it has the keyboard and gamepad focus. Drawn for the
        // current frame, lighter while hovered or focused, darker while
        // pressed, outlined while focused; call it every frame, from update.
        // Options: size, color (the label's), background, hover, pressed,
        // radius, outline, outline_width, focus (the focus outline's color),
        // default (focused when the buttons appear), focusable (false keeps
        // it out of keyboard and gamepad focus), id (who it is for the
        // focus, instead of its label).
        let hud = Rc::clone(&self.hud);
        let input = Rc::clone(input_handler);
        let focus = Rc::clone(&self.button_focus);
        let button = self.lua.create_function(
            move |_,
                  (label, x, y, width, height, options): (
                String,
                f32,
                f32,
                f32,
                f32,
                Option<Table>,
            )| {
                const NAME: &str = "button";
                let (position, size) = area(NAME, x, y, width, height)?;
                let mut name = label.clone();
                let mut button = HudButton {
                    label,
                    position,
                    size,
                    text_size: BUTTON_TEXT_SIZE,
                    text_color: BUTTON_TEXT_COLOR,
                    background: BUTTON_BACKGROUND,
                    radius: BUTTON_RADIUS,
                    outline: BUTTON_OUTLINE,
                };
                let (mut hover, mut pressed) = (BUTTON_HOVER, BUTTON_PRESSED);
                let mut outline_width = None;
                let mut focus_color = BUTTON_FOCUS.0;
                let (mut default, mut focusable) = (false, true);
                read_options(
                    NAME,
                    options.as_ref(),
                    &[
                        "size",
                        "color",
                        "background",
                        "hover",
                        "pressed",
                        "radius",
                        "outline",
                        "outline_width",
                        "focus",
                        "default",
                        "focusable",
                        "id",
                    ],
                    |key, value| {
                        Ok(match key {
                            "size" => number(NAME, key, &value, true)?
                                .map(|s| button.text_size = s)
                                .is_some(),
                            "color" => color(NAME, &value)?
                                .map(|c| button.text_color = c)
                                .is_some(),
                            "background" => color(NAME, &value)?
                                .map(|c| button.background = c)
                                .is_some(),
                            "hover" => color(NAME, &value)?.map(|c| hover = c).is_some(),
                            "pressed" => color(NAME, &value)?.map(|c| pressed = c).is_some(),
                            "radius" => number(NAME, key, &value, false)?
                                .map(|r| button.radius = r)
                                .is_some(),
                            "outline" => {
                                color(NAME, &value)?.map(|c| button.outline.0 = c).is_some()
                            }
                            "outline_width" => number(NAME, key, &value, false)?
                                .map(|w| outline_width = Some(w))
                                .is_some(),
                            "focus" => color(NAME, &value)?.map(|c| focus_color = c).is_some(),
                            "default" => boolean(&value).map(|d| default = d).is_some(),
                            "focusable" => boolean(&value).map(|f| focusable = f).is_some(),
                            _ => string(&value)?.map(|id| name = id).is_some(),
                        })
                    },
                )?;
                if let Some(width) = outline_width {
                    button.outline.1 = width;
                }
                let (focused, pressed_by_key) = focus
                    .borrow_mut()
                    .button(&name, position, size, focusable, default);

                let input = input.borrow();
                let hovered = button.contains(input.get_mouse_screen_pos());
                let armed = input
                    .get_mouse_press_screen_pos()
                    .is_some_and(|press| button.contains(press));
                let held = input.is_mouse_button_pressed(PointerButton::Primary);
                let clicked =
                    hovered && armed && input.is_mouse_button_just_released(PointerButton::Primary);
                drop(input);
                if hovered && armed && held {
                    button.background = pressed;
                } else if hovered || focused {
                    button.background = hover;
                }
                if focused {
                    button.outline = (focus_color, BUTTON_FOCUS.1);
                }
                hud.borrow_mut().push(HudItem::Button(button));
                Ok((clicked || pressed_by_key, focused))
            },
        )?;
        globals.set("button", button)?;

        Ok(())
    }
}
