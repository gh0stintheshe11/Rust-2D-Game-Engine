//! Sprite sheet animation, advanced while the game runs.
//!
//! All state lives in attributes, so animations survive script reloads, show
//! up in the Inspector, and can be driven by scripts and the editor alike:
//!
//! - `animation` (String): the frames to play, e.g. `"0-3"` or `"0,1,2,1"`
//! - `animation_fps` (Float, default 10) and `animation_loop` (Boolean,
//!   default true)
//! - `animation_time` (Float): seconds since the animation started; created
//!   and advanced by [`advance_animations`], reset to start over
//! - `frame` (Integer or Float): the frame shown, written by
//!   [`advance_animations`]

use crate::scene::{AttributeType, AttributeValue, Entity, Scene};

/// Frames per second of an animation without an `animation_fps` attribute.
pub const DEFAULT_ANIMATION_FPS: f32 = 10.0;

/// Longest animation [`parse_frames`] accepts.
const MAX_ANIMATION_FRAMES: usize = 4096;

/// Parse an animation's frames: frame numbers and inclusive ranges,
/// separated by commas, e.g. `"0-3"`, `"0,1,2,1"` or `"7-4"` (backwards).
pub fn parse_frames(spec: &str) -> Result<Vec<u32>, String> {
    let invalid = |reason: String| format!("invalid animation \"{}\": {}", spec, reason);
    let number = |text: &str| {
        let text = text.trim();
        text.parse::<u32>()
            .map_err(|_| invalid(format!("\"{}\" is not a frame number", text)))
    };

    let mut frames = Vec::new();
    for part in spec.split(',') {
        match part.split_once('-') {
            Some((start, end)) => {
                let (start, end) = (number(start)?, number(end)?);
                let length = start.abs_diff(end) as usize + 1;
                if frames.len() + length > MAX_ANIMATION_FRAMES {
                    return Err(invalid(format!(
                        "more than {} frames",
                        MAX_ANIMATION_FRAMES
                    )));
                }
                if start <= end {
                    frames.extend(start..=end);
                } else {
                    frames.extend((end..=start).rev());
                }
            }
            None => frames.push(number(part)?),
        }
        if frames.len() > MAX_ANIMATION_FRAMES {
            return Err(invalid(format!(
                "more than {} frames",
                MAX_ANIMATION_FRAMES
            )));
        }
    }
    Ok(frames)
}

/// Advance every animated entity in the scene by `dt` seconds: for each
/// entity with a non-empty `animation`, increase `animation_time` and set
/// `frame` to the animation's current frame. A looping animation's time
/// wraps around at the end of each cycle; a non-looping one stops on its
/// last frame.
///
/// Returns a description of each animation that couldn't be played (an
/// invalid `animation` value); those entities are left unchanged.
pub fn advance_animations(scene: &mut Scene, dt: f32) -> Vec<String> {
    let mut problems = Vec::new();
    for entity in scene.entities.values_mut() {
        let Some(spec) = entity
            .get_string("animation")
            .filter(|spec| !spec.trim().is_empty())
        else {
            continue;
        };
        let frames = match parse_frames(spec) {
            Ok(frames) => frames,
            Err(e) => {
                problems.push(format!("Entity '{}': {}", entity.name, e));
                continue;
            }
        };

        let fps = entity
            .get_float("animation_fps")
            .filter(|fps| fps.is_finite() && *fps > 0.0)
            .unwrap_or(DEFAULT_ANIMATION_FPS);
        let looping = entity.get_bool("animation_loop").unwrap_or(true);
        let mut time = entity
            .get_float("animation_time")
            .filter(|time| time.is_finite())
            .unwrap_or(0.0)
            .max(0.0)
            + dt.max(0.0);
        let cycle = frames.len() as f32 / fps;
        if looping {
            // Keeps the time small, so it stays precise however long it plays
            time %= cycle;
        }

        // Float-to-int conversion saturates, so huge times can't overflow
        let step = (time * fps).floor() as usize;
        let index = if looping {
            step % frames.len()
        } else {
            step.min(frames.len() - 1)
        };
        let _ = entity.set_or_create_attribute("animation_time", AttributeValue::Float(time));
        set_frame(entity, frames[index]);
    }
    problems
}

/// Write the `frame` attribute, keeping its type if it exists (the Inspector
/// creates Float attributes by default).
fn set_frame(entity: &mut Entity, frame: u32) {
    let value = match entity
        .get_attribute_by_name("frame")
        .map(|attr| &attr.data_type)
    {
        Ok(AttributeType::Float) => AttributeValue::Float(frame as f32),
        _ => AttributeValue::Integer(frame.min(i32::MAX as u32) as i32),
    };
    let _ = entity.set_or_create_attribute("frame", value);
}
