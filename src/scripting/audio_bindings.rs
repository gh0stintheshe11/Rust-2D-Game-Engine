use std::cell::RefCell;
use std::rc::Rc;

use mlua::{Table, Value};

use super::{LuaScripting, parse_uuid};
use crate::audio::{
    AudioEngine, MusicOptions, PlayOptions, check_group, check_seconds, check_volume,
};
use crate::logger::LOGGER;
use crate::project::ProjectManager;

/// Validate a volume argument of `function`.
fn volume_arg(function: &str, volume: f32) -> mlua::Result<f32> {
    check_volume(volume).map_err(|e| mlua::Error::external(format!("{}: {}", function, e)))
}

/// Validate a number of seconds given to `function`.
fn seconds_arg(function: &str, seconds: f32) -> mlua::Result<f32> {
    check_seconds(seconds).map_err(|e| mlua::Error::external(format!("{}: {}", function, e)))
}

fn option_error(function: &str, message: String) -> mlua::Error {
    mlua::Error::external(format!("{}: {}", function, message))
}

/// The pairs of an options table of `function`, by name. Unknown options are
/// errors (in the callers), so typos don't go unnoticed.
///
/// This runs before any engine state is borrowed: reading a table can run
/// Lua code through metamethods.
fn options(function: &str, table: Option<Table>) -> mlua::Result<Vec<(String, Value)>> {
    let mut pairs = Vec::new();
    let Some(table) = table else {
        return Ok(pairs);
    };
    for pair in table.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let Value::String(key) = key else {
            return Err(option_error(
                function,
                format!("option names must be strings, got a {}", key.type_name()),
            ));
        };
        pairs.push((key.to_str()?.to_string(), value));
    }
    Ok(pairs)
}

fn number_option(function: &str, key: &str, value: Value) -> mlua::Result<f32> {
    match value {
        Value::Number(number) => Ok(number as f32),
        Value::Integer(number) => Ok(number as f32),
        other => Err(option_error(
            function,
            format!("{} must be a number, got a {}", key, other.type_name()),
        )),
    }
}

fn bool_option(function: &str, key: &str, value: Value) -> mlua::Result<bool> {
    match value {
        Value::Boolean(value) => Ok(value),
        other => Err(option_error(
            function,
            format!("{} must be true or false, got a {}", key, other.type_name()),
        )),
    }
}

/// Read `play_sound`'s options table: `volume` (number >= 0), `loop`
/// (boolean), `group` (a volume group's name) and `fade_in` (seconds).
fn play_options(table: Option<Table>) -> mlua::Result<PlayOptions> {
    const F: &str = "play_sound";
    let mut parsed = PlayOptions::default();
    for (key, value) in options(F, table)? {
        match key.as_str() {
            "volume" => parsed.volume = volume_arg(F, number_option(F, &key, value)?)?,
            "loop" => parsed.looping = bool_option(F, &key, value)?,
            "fade_in" => parsed.fade_in = seconds_arg(F, number_option(F, &key, value)?)?,
            "group" => {
                let Value::String(group) = value else {
                    return Err(option_error(
                        F,
                        format!("group must be a string, got a {}", value.type_name()),
                    ));
                };
                parsed.group = group.to_str()?.to_string();
                check_group(&parsed.group).map_err(|e| option_error(F, e))?;
            }
            unknown => {
                return Err(option_error(
                    F,
                    format!(
                        "unknown option '{}' (expected volume, loop, group, fade_in)",
                        unknown
                    ),
                ));
            }
        }
    }
    Ok(parsed)
}

/// Read `play_music`'s options table: `volume`, `crossfade` (seconds) and
/// `loop` (true unless false).
fn music_options(table: Option<Table>) -> mlua::Result<MusicOptions> {
    const F: &str = "play_music";
    let mut parsed = MusicOptions::default();
    for (key, value) in options(F, table)? {
        match key.as_str() {
            "volume" => parsed.volume = volume_arg(F, number_option(F, &key, value)?)?,
            "crossfade" => parsed.crossfade = seconds_arg(F, number_option(F, &key, value)?)?,
            "loop" => parsed.looping = bool_option(F, &key, value)?,
            unknown => {
                return Err(option_error(
                    F,
                    format!(
                        "unknown option '{}' (expected volume, crossfade, loop)",
                        unknown
                    ),
                ));
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

        // stop_sound(play_id [, seconds]): stop a sound started by
        // play_sound, fading it out over `seconds` if given. Accepts the nil
        // play_sound returns when nothing could be played.
        let audio = Rc::clone(audio_engine);
        let stop_sound = self.lua.create_function(
            move |_, (play_id, seconds): (Option<String>, Option<f32>)| {
                let seconds = seconds_arg("stop_sound", seconds.unwrap_or(0.0))?;
                let Some(play_id) = play_id else {
                    return Ok(());
                };
                let uuid = parse_uuid(&play_id, "sound")?;
                if let Err(e) = audio.borrow_mut().stop_with_fade(uuid, seconds) {
                    LOGGER.debug(format!("stop_sound(\"{}\") failed: {}", play_id, e));
                }
                Ok(())
            },
        )?;
        globals.set("stop_sound", stop_sound)?;

        // fade_sound(play_id, volume, seconds): fade a playing sound's volume
        // to `volume` over `seconds`. Accepts nil like stop_sound.
        let audio = Rc::clone(audio_engine);
        let fade_sound = self.lua.create_function(
            move |_, (play_id, volume, seconds): (Option<String>, f32, f32)| {
                let volume = volume_arg("fade_sound", volume)?;
                let seconds = seconds_arg("fade_sound", seconds)?;
                let Some(play_id) = play_id else {
                    return Ok(());
                };
                let uuid = parse_uuid(&play_id, "sound")?;
                if let Err(e) = audio.borrow_mut().fade(uuid, volume, seconds) {
                    LOGGER.debug(format!("fade_sound(\"{}\") failed: {}", play_id, e));
                }
                Ok(())
            },
        )?;
        globals.set("fade_sound", fade_sound)?;

        // play_music(path [, {volume = 1.0, crossfade = 0, loop = true}]) ->
        // play id, or nil when it can't play: make a file the music, in the
        // "music" group. The music playing fades out over `crossfade` while
        // this one fades in; the same file plays on, at the new volume.
        let audio = Rc::clone(audio_engine);
        let play_music =
            self.lua
                .create_function(move |_, (path, options): (String, Option<Table>)| {
                    let options = music_options(options)?;
                    let full_path = ProjectManager::resolve_asset_path(&path);
                    Ok(audio
                        .borrow_mut()
                        .play_music(&full_path, options)
                        .ok()
                        .map(|play_id| play_id.to_string()))
                })?;
        globals.set("play_music", play_music)?;

        // stop_music([seconds]): fade the music out over `seconds` and stop it
        let audio = Rc::clone(audio_engine);
        let stop_music = self.lua.create_function(move |_, seconds: Option<f32>| {
            let seconds = seconds_arg("stop_music", seconds.unwrap_or(0.0))?;
            audio
                .borrow_mut()
                .stop_music(seconds)
                .map_err(mlua::Error::external)
        })?;
        globals.set("stop_music", stop_music)?;

        // set_group_volume(group, volume): scale every sound of a volume
        // group ("sfx" unless play_sound says otherwise; "music"), playing
        // or started later. Reset to 1.0 when the game stops.
        let audio = Rc::clone(audio_engine);
        let set_group_volume =
            self.lua
                .create_function(move |_, (group, volume): (String, f32)| {
                    let volume = volume_arg("set_group_volume", volume)?;
                    audio
                        .borrow_mut()
                        .set_group_volume(&group, volume)
                        .map_err(|e| option_error("set_group_volume", e))
                })?;
        globals.set("set_group_volume", set_group_volume)?;

        // get_group_volume(group) -> its volume (1.0 unless set)
        let audio = Rc::clone(audio_engine);
        let get_group_volume = self
            .lua
            .create_function(move |_, group: String| Ok(audio.borrow().group_volume(&group)))?;
        globals.set("get_group_volume", get_group_volume)?;

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
