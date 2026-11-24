#[cfg(test)]
mod tests {
    use rust_2d_game_engine::audio::AudioEngine;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::Duration;

    /// Write a two-second 440 Hz tone (16-bit mono WAV) to a temp file.
    fn test_tone() -> PathBuf {
        let rate: u32 = 22050;
        let samples: Vec<i16> = (0..rate * 2)
            .map(|i| {
                let t = i as f32 / rate as f32;
                ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 8000.0) as i16
            })
            .collect();
        let data_len = (samples.len() * 2) as u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }
        let path = std::env::temp_dir().join(format!("rust2d_tone_{}.wav", uuid::Uuid::new_v4()));
        std::fs::write(&path, wav).unwrap();
        path
    }

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
        let result = audio_engine.play_sound(&test_tone());
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
        let tone = test_tone();
        let path = tone.as_path();

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
        let tone = test_tone();
        let path = tone.as_path();

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
        let tone = test_tone();
        let path = tone.as_path();

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
        let tone = test_tone();
        let path = tone.as_path();

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

    #[test]
    fn test_audio_duration() {
        // Reads the file header only; works without an audio device
        let audio_engine = AudioEngine::new();
        let duration = audio_engine.get_audio_duration(&test_tone()).unwrap();
        assert!((duration - 2.0).abs() < 0.01, "duration {}", duration);
        assert!(
            audio_engine
                .get_audio_duration(Path::new("missing.wav"))
                .is_err()
        );
    }
}
