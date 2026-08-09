#[cfg(test)]
mod tests {
    use rust_2d_game_engine::audio_engine::AudioEngine;
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    const TEST_AUDIO_FILE: &str = "tests/level-up-22268.mp3";

    /// Returns an engine if an audio output device is available,
    /// otherwise logs a skip notice (headless machines, CI, WSL).
    fn engine_or_skip() -> Option<AudioEngine> {
        let audio_engine = AudioEngine::new();
        if audio_engine.has_output() {
            Some(audio_engine)
        } else {
            eprintln!("skipping: no audio output device available");
            None
        }
    }

    #[test]
    fn test_audio_engine_initialization() {
        // Must not panic even without an audio device.
        let audio_engine = AudioEngine::new();
        assert!(
            audio_engine.list_playing_sounds().is_empty(),
            "No sounds should be playing initially"
        );
    }

    #[test]
    fn test_play_sound_fails_gracefully_without_device() {
        let mut audio_engine = AudioEngine::new();
        if audio_engine.has_output() {
            return; // Only meaningful on devices without audio output.
        }
        let result = audio_engine.play_sound(Path::new(TEST_AUDIO_FILE));
        assert!(
            result.is_err(),
            "play_sound should return Err, not panic, without a device"
        );
    }

    #[test]
    fn test_play_sound() {
        let Some(mut audio_engine) = engine_or_skip() else {
            return;
        };
        let path = Path::new(TEST_AUDIO_FILE);

        // Test audio playback
        let sound_id = audio_engine.play_sound(path).expect("Failed to play sound");

        // Wait for playback to start
        thread::sleep(Duration::from_millis(500));

        // Verify sound is playing
        assert!(
            audio_engine.is_playing(sound_id),
            "The audio should be playing"
        );

        // Stop the sound
        audio_engine.stop(sound_id).expect("Failed to stop sound");

        // Verify sound stopped
        assert!(
            audio_engine.is_stopped(sound_id),
            "The audio should be stopped"
        );
    }

    #[test]
    fn test_playback_controls() {
        let Some(mut audio_engine) = engine_or_skip() else {
            return;
        };
        let path = Path::new(TEST_AUDIO_FILE);

        // Play sound and get ID
        let sound_id = audio_engine.play_sound(path).expect("Failed to play sound");
        thread::sleep(Duration::from_millis(500));

        // Test pause
        audio_engine.pause(sound_id).expect("Failed to pause");
        assert!(audio_engine.is_paused(sound_id), "Audio should be paused");
        assert!(
            !audio_engine.is_playing(sound_id),
            "Audio should not be playing while paused"
        );

        // Test resume
        audio_engine.resume(sound_id).expect("Failed to resume");
        assert!(
            audio_engine.is_playing(sound_id),
            "Audio should be playing after resume"
        );
        assert!(
            !audio_engine.is_paused(sound_id),
            "Audio should not be paused"
        );

        // Test stop
        audio_engine.stop(sound_id).expect("Failed to stop");
        assert!(audio_engine.is_stopped(sound_id), "Audio should be stopped");
    }

    #[test]
    fn test_immediate_playback() {
        let Some(mut audio_engine) = engine_or_skip() else {
            return;
        };
        let path = Path::new(TEST_AUDIO_FILE);

        // Play immediate sound
        audio_engine
            .play_sound_immediate(path)
            .expect("Failed to play immediate sound");
        thread::sleep(Duration::from_millis(500));

        // Stop immediate sound
        audio_engine.stop_immediate();
    }

    #[test]
    fn test_cleanup() {
        let Some(mut audio_engine) = engine_or_skip() else {
            return;
        };
        let path = Path::new(TEST_AUDIO_FILE);

        // Load and play sound
        let sound_id = audio_engine.play_sound(path).expect("Failed to play sound");

        // Cleanup should stop all sounds and clear caches
        audio_engine.cleanup();

        assert!(
            audio_engine.is_stopped(sound_id),
            "Sound should be stopped after cleanup"
        );
        assert_eq!(
            audio_engine.get_memory_usage(),
            0,
            "Cache should be empty after cleanup"
        );
    }
}
