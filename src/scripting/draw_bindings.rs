use std::rc::Rc;

use mlua::{Table, Value};

use super::LuaScripting;
use crate::render::{HudText, TextAlign, parse_color};

/// Font height of `draw_text` when no `size` is given, in camera frame
/// pixels.
const DEFAULT_TEXT_SIZE: f32 = 24.0;

fn draw_text_error(message: String) -> mlua::Error {
    mlua::Error::external(format!("draw_text: {}", message))
}

/// Apply `draw_text`'s options table: `size` (number > 0), `color`
/// ("#rrggbb" or "#rrggbbaa") and `align` ("left", "center" or "right").
/// Unknown options are errors, so typos don't go unnoticed.
///
/// This runs before the frame's text list is borrowed: reading a table can
/// run Lua code through metamethods.
fn apply_options(text: &mut HudText, options: &Table) -> mlua::Result<()> {
    for pair in options.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::String(key) = key else {
            return Err(draw_text_error(format!(
                "option names must be strings, got a {}",
                key.type_name()
            )));
        };
        match (key.to_str()?.as_ref(), value) {
            ("size", Value::Number(size)) => text.size = size_arg(size as f32)?,
            ("size", Value::Integer(size)) => text.size = size_arg(size as f32)?,
            ("color", Value::String(color)) => {
                text.color = parse_color(&color.to_str()?).map_err(draw_text_error)?
            }
            ("align", Value::String(align)) => {
                text.align = match align.to_str()?.as_ref() {
                    "left" => TextAlign::Left,
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    other => {
                        return Err(draw_text_error(format!(
                            "align must be \"left\", \"center\" or \"right\", got \"{}\"",
                            other
                        )));
                    }
                }
            }
            (name @ ("size" | "color" | "align"), other) => {
                return Err(draw_text_error(format!(
                    "{} has the wrong type ({})",
                    name,
                    other.type_name()
                )));
            }
            (unknown, _) => {
                return Err(draw_text_error(format!(
                    "unknown option '{}' (expected size, color or align)",
                    unknown
                )));
            }
        }
    }
    Ok(())
}

fn size_arg(size: f32) -> mlua::Result<f32> {
    if size.is_finite() && size > 0.0 {
        Ok(size)
    } else {
        Err(draw_text_error(format!(
            "size must be a number > 0, got {}",
            size
        )))
    }
}

impl LuaScripting {
    pub(crate) fn register_draw_bindings(&mut self) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // draw_text(text, x, y [, {size = 24, color = "#ffffff", align = "left"}]):
        // draw text over the game for the current frame. Positions and sizes
        // are in camera frame pixels; numbers are converted to text.
        let hud = Rc::clone(&self.hud);
        let draw_text = self.lua.create_function(
            move |_, (text, x, y, options): (String, f32, f32, Option<Table>)| {
                if !x.is_finite() || !y.is_finite() {
                    return Err(draw_text_error(format!(
                        "the position must be finite, got ({}, {})",
                        x, y
                    )));
                }
                let mut hud_text = HudText {
                    text,
                    position: (x, y),
                    size: DEFAULT_TEXT_SIZE,
                    color: egui::Color32::WHITE,
                    align: TextAlign::Left,
                };
                if let Some(options) = &options {
                    apply_options(&mut hud_text, options)?;
                }
                hud.borrow_mut().push(hud_text);
                Ok(())
            },
        )?;
        globals.set("draw_text", draw_text)?;

        Ok(())
    }
}
