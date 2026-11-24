use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Table, Value};

use super::{LuaScripting, parse_uuid};
use crate::audio::{AudioEngine, PlayOptions, check_volume};
use crate::logger::LOGGER;
use crate::project::ProjectManager;

/// Validate a volume argument of `function`.
fn volume_arg(function: &str, volume: f32) -> mlua::Result<f32> {
    check_volume(volume).map_err(|e| mlua::Error::external(format!("{}: {}", function, e)))
}

fn play_sound_error(message: String) -> mlua::Error {
    mlua::Error::external(format!("play_sound: {}", message))
}

/// Read `play_sound`'s options table: `volume` (number >= 0) and `loop`
/// (boolean). Unknown options are errors, so typos don't go unnoticed.
///
/// This runs before any engine state is borrowed: reading a table can run
/// Lua code through metamethods.
fn play_options(options: Option<Table>) -> mlua::Result<PlayOptions> {
    let mut parsed = PlayOptions::default();
    let Some(options) = options else {
        return Ok(parsed);
    };
    for pair in options.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::String(key) = key else {
            return Err(play_sound_error(format!(
                "option names must be strings, got a {}",
                key.type_name()
            )));
        };
        match (key.to_str()?.as_ref(), value) {
            ("volume", Value::Number(volume)) => {
                parsed.volume = volume_arg("play_sound", volume as f32)?
            }
            ("volume", Value::Integer(volume)) => {
                parsed.volume = volume_arg("play_sound", volume as f32)?
            }
            ("volume", other) => {
                return Err(play_sound_error(format!(
                    "volume must be a number, got a {}",
                    other.type_name()
                )));
            }
            ("loop", Value::Boolean(looping)) => parsed.looping = looping,
            ("loop", other) => {
                return Err(play_sound_error(format!(
                    "loop must be true or false, got a {}",
                    other.type_name()
                )));
            }
            (unknown, _) => {
                return Err(play_sound_error(format!(
                    "unknown option '{}' (expected volume or loop)",
                    unknown
                )));
            }
        }
    }
    Ok(parsed)
}

impl LuaScripting {
    pub(crate) fn register_audio_bindings(
        &mut self,
        audio_engine: &Rc<RefCell<AudioEngine>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // play_sound(path [, {volume = 1.0, loop = false}]) -> play id, or
        // nil when playback isn't possible (missing file, no audio device)
        let audio = Rc::clone(audio_engine);
        let play_sound =
            self.lua
                .create_function(move |_, (path, options): (String, Option<Table>)| {
                    let options = play_options(options)?;
                    let full_path = ProjectManager::resolve_asset_path(&path);
                    // Failures (missing file, no audio device) are reported by
                    // the audio engine, once per file
                    Ok(audio
                        .borrow_mut()
                        .play_sound_with(&full_path, options)
                        .ok()
                        .map(|play_id| play_id.to_string()))
                })?;
        globals.set("play_sound", play_sound)?;

        // set_sound_volume(play_id, volume): change a playing sound's volume.
        // Accepts the nil play_sound returns when nothing could be played.
        let audio = Rc::clone(audio_engine);
        let set_sound_volume =
            self.lua
                .create_function(move |_, (play_id, volume): (Option<String>, f32)| {
                    let volume = volume_arg("set_sound_volume", volume)?;
                    let Some(play_id) = play_id else {
                        return Ok(());
                    };
                    let uuid = parse_uuid(&play_id, "sound")?;
                    if let Err(e) = audio.borrow_mut().set_volume(uuid, volume) {
                        LOGGER.debug(format!("set_sound_volume(\"{}\") failed: {}", play_id, e));
                    }
                    Ok(())
                })?;
        globals.set("set_sound_volume", set_sound_volume)?;

        // set_master_volume(volume): scale every sound, playing or started
        // later. Reset to 1.0 when the game stops.
        let audio = Rc::clone(audio_engine);
        let set_master_volume = self.lua.create_function(move |_, volume: f32| {
            let volume = volume_arg("set_master_volume", volume)?;
            audio
                .borrow_mut()
                .set_master_volume(volume)
                .map_err(mlua::Error::external)
        })?;
        globals.set("set_master_volume", set_master_volume)?;

        // stop_sound(play_id): stop a sound started by play_sound. Accepts
        // the nil play_sound returns when nothing could be played.
        let audio = Rc::clone(audio_engine);
        let stop_sound = self
            .lua
            .create_function(move |_, play_id: Option<String>| {
                let Some(play_id) = play_id else {
                    return Ok(());
                };
                let uuid = parse_uuid(&play_id, "sound")?;
                if let Err(e) = audio.borrow_mut().stop(uuid) {
                    LOGGER.debug(format!("stop_sound(\"{}\") failed: {}", play_id, e));
                }
                Ok(())
            })?;
        globals.set("stop_sound", stop_sound)?;

        // is_sound_playing(play_id) -> bool (false for nil)
        let audio = Rc::clone(audio_engine);
        let is_sound_playing = self
            .lua
            .create_function(move |_, play_id: Option<String>| {
                let Some(play_id) = play_id else {
                    return Ok(false);
                };
                let uuid = parse_uuid(&play_id, "sound")?;
                Ok(audio.borrow().is_playing(uuid))
            })?;
        globals.set("is_sound_playing", is_sound_playing)?;

        // stop_all_sounds()
        let audio = Rc::clone(audio_engine);
        let stop_all_sounds = self.lua.create_function(move |_, ()| {
            audio.borrow_mut().stop_all();
            Ok(())
        })?;
        globals.set("stop_all_sounds", stop_all_sounds)?;

        Ok(())
    }
}
