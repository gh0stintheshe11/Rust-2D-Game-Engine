use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::{parse_uuid, LuaScripting};
use crate::audio_engine::AudioEngine;
use crate::logger::LOGGER;
use crate::project_manager::ProjectManager;

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
            let full_path = match ProjectManager::get_project_path() {
                Some(project) => PathBuf::from(project).join(&path),
                None => PathBuf::from(&path),
            };
            match audio.borrow_mut().play_sound(&full_path) {
                Ok(play_id) => Ok(Some(play_id.to_string())),
                Err(e) => {
                    LOGGER.debug(format!("play_sound(\"{}\") failed: {}", path, e));
                    Ok(None)
                }
            }
        })?;
        globals.set("play_sound", play_sound)?;

        // stop_sound(play_id): stop a sound started by play_sound
        let audio = Rc::clone(audio_engine);
        let stop_sound = self.lua.create_function(move |_, play_id: String| {
            let uuid = parse_uuid(&play_id, "sound")?;
            if let Err(e) = audio.borrow_mut().stop(uuid) {
                LOGGER.debug(format!("stop_sound(\"{}\") failed: {}", play_id, e));
            }
            Ok(())
        })?;
        globals.set("stop_sound", stop_sound)?;

        // is_sound_playing(play_id) -> bool
        let audio = Rc::clone(audio_engine);
        let is_sound_playing = self.lua.create_function(move |_, play_id: String| {
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
