use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Table, Value};

use super::{LuaScripting, parse_uuid};
use crate::render::{HudText, TextAlign, parse_color, parse_frames};
use crate::scene::{AttributeValue, SceneManager};

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

fn play_animation_error(message: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::external(format!("play_animation: {}", message))
}

/// `play_animation`'s options.
#[derive(Default)]
struct AnimationOptions {
    fps: Option<f32>,
    looping: Option<bool>,
    restart: bool,
}

/// Read `play_animation`'s options table: `fps` (number > 0), `loop` and
/// `restart` (booleans). Unknown options are errors. Runs before the scene
/// is borrowed: reading a table can run Lua code through metamethods.
fn animation_options(options: Option<&Table>) -> mlua::Result<AnimationOptions> {
    let mut parsed = AnimationOptions::default();
    let Some(options) = options else {
        return Ok(parsed);
    };
    for pair in options.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::String(key) = key else {
            return Err(play_animation_error(format!(
                "option names must be strings, got a {}",
                key.type_name()
            )));
        };
        let fps = match value {
            Value::Number(fps) => Some(fps as f32),
            Value::Integer(fps) => Some(fps as f32),
            _ => None,
        };
        match (key.to_str()?.as_ref(), value) {
            ("fps", _) => match fps.filter(|fps| fps.is_finite() && *fps > 0.0) {
                Some(fps) => parsed.fps = Some(fps),
                None => return Err(play_animation_error("fps must be a number > 0")),
            },
            ("loop", Value::Boolean(looping)) => parsed.looping = Some(looping),
            ("restart", Value::Boolean(restart)) => parsed.restart = restart,
            (name @ ("loop" | "restart"), other) => {
                return Err(play_animation_error(format!(
                    "{} must be true or false, got a {}",
                    name,
                    other.type_name()
                )));
            }
            (unknown, _) => {
                return Err(play_animation_error(format!(
                    "unknown option '{}' (expected fps, loop or restart)",
                    unknown
                )));
            }
        }
    }
    Ok(parsed)
}

impl LuaScripting {
    pub(crate) fn register_render_bindings(
        &mut self,
        scene_manager: &Rc<RefCell<SceneManager>>,
    ) -> Result<(), mlua::Error> {
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

        // play_animation(scene_id, entity_id, frames [, {fps = 10, loop = true, restart = false}]):
        // play sprite sheet frames ("0-3", "0,1,2,1"). Starts over when the
        // frames differ from the current animation (or with restart = true),
        // so calling it every frame keeps the animation running. Creates the
        // animation attributes the entity doesn't have.
        let manager = Rc::clone(scene_manager);
        let play_animation = self.lua.create_function(
            move |_,
                  (scene_id, entity_id, frames, options): (
                String,
                String,
                String,
                Option<Table>,
            )| {
                let options = animation_options(options.as_ref())?;
                parse_frames(&frames).map_err(play_animation_error)?;
                let scene_uuid = parse_uuid(&scene_id, "scene")?;
                let entity_uuid = parse_uuid(&entity_id, "entity")?;

                let mut manager = manager.borrow_mut();
                let entity = manager
                    .get_scene_mut(scene_uuid)
                    .ok_or_else(|| {
                        play_animation_error(format!("scene '{}' not found", scene_uuid))
                    })?
                    .get_entity_mut(entity_uuid)
                    .map_err(play_animation_error)?;

                let restart =
                    options.restart || entity.get_string("animation") != Some(frames.as_str());
                let mut set = |name: &str, value: AttributeValue| {
                    entity
                        .set_or_create_attribute(name, value)
                        .map_err(play_animation_error)
                };
                set("animation", AttributeValue::String(frames))?;
                if let Some(fps) = options.fps {
                    set("animation_fps", AttributeValue::Float(fps))?;
                }
                if let Some(looping) = options.looping {
                    set("animation_loop", AttributeValue::Boolean(looping))?;
                }
                if restart {
                    set("animation_time", AttributeValue::Float(0.0))?;
                }
                Ok(())
            },
        )?;
        globals.set("play_animation", play_animation)?;

        Ok(())
    }
}
