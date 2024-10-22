#[cfg(test)]
mod tests {
    use rust_2d_game_engine::audio_engine::AudioEngine;
    use std::time::Duration;
    use std::thread;

    const TEST_AUDIO_FILE: &str = "/Users/lang/Rust-2D-Game-Engine/tests/level-up-22268.mp3";

    #[test]
    fn test_audio_engine_initialization() {
        // Initialize the audio engine
        let audio_engine = AudioEngine::new();
        
        // Check if the sink is initialized and not playing anything initially
        assert!(audio_engine.sink.empty(), "The sink should be empty after initialization.");
    }

    #[test]
    fn test_play_sound() {
        // Initialize the audio engine
        let audio_engine = AudioEngine::new();

        // Test audio playback
        let result = audio_engine.play_sound(TEST_AUDIO_FILE);
        assert!(result.is_ok(), "Failed to play sound: {:?}", result.err());

        // Wait a bit to ensure the sound starts playing
        thread::sleep(Duration::from_millis(500));

        // Check if the audio is playing
        assert!(audio_engine.is_playing(), "The audio should be playing after calling play_sound.");

        // Stop the audio
        audio_engine.sink.stop();

        // Wait for the audio to stop
        thread::sleep(Duration::from_millis(500));

        // Check if the audio is stopped
        assert!(!audio_engine.is_playing(), "The audio should have stopped.");
    }

    #[test]
    fn test_is_playing() {
        let audio_engine = AudioEngine::new();

        // Initially, nothing should be playing
        assert!(!audio_engine.is_playing(), "No audio should be playing initially.");

        // Play a sound
        let result = audio_engine.play_sound(TEST_AUDIO_FILE);
        assert!(result.is_ok(), "Failed to play sound: {:?}", result.err());

        // Wait a bit to ensure the sound starts playing
        thread::sleep(Duration::from_millis(500));

        // Check if the sound is playing
        assert!(audio_engine.is_playing(), "Audio should be playing after starting a sound.");

        // Pause the audio
        audio_engine.pause();

        // Check if the sound stopped playing
        assert!(!audio_engine.is_playing(), "Audio should not be considered playing when paused.");

        // Resume the audio
        audio_engine.resume();

        // Check if the sound is playing again
        assert!(audio_engine.is_playing(), "Audio should be playing after resuming.");

        // Stop the audio
        audio_engine.sink.stop();

        // Wait a bit to ensure the sound stops
        thread::sleep(Duration::from_millis(500));

        // Check if the sound stopped playing
        assert!(!audio_engine.is_playing(), "Audio should not be playing after stopping.");
    }
}
