use std::cell::RefCell;
use std::rc::Rc;

use super::{LuaScripting, parse_uuid};
use crate::audio::AudioEngine;
use crate::logger::LOGGER;
use crate::project::ProjectManager;

impl LuaScripting {
    pub(crate) fn register_audio_bindings(
        &mut self,
        audio_engine: &Rc<RefCell<AudioEngine>>,
    ) -> Result<(), mlua::Error> {
        let globals = self.lua.globals();

        // play_sound(relative_path) -> play id string, or nil when playback
        // isn't possible (missing file, no audio device on this machine)
        let audio = Rc::clone(audio_engine);
        let play_sound = self.lua.create_function(move |_, path: String| {
            let full_path = ProjectManager::resolve_asset_path(&path);
            // Failures (missing file, no audio device) are reported by the
            // audio engine, once per file
            Ok(audio
                .borrow_mut()
                .play_sound(&full_path)
                .ok()
                .map(|play_id| play_id.to_string()))
        })?;
        globals.set("play_sound", play_sound)?;

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
