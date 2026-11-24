//! Audio engine tests. Playback runs on an offline engine (mixed into a
//! sample stream instead of a sound card), so the tests measure what is
//! actually heard on every machine, including CI.

mod common;

use common::{TONE_PEAK, assert_level, level, offline_audio, render, write_tone};
use rust_2d_game_engine::audio::{AudioEngine, PlayOptions};
use std::path::Path;

fn looping() -> PlayOptions {
    PlayOptions {
        looping: true,
        ..PlayOptions::default()
    }
}

#[test]
fn test_engine_without_device_does_not_panic() {
    let mut engine = AudioEngine::new();
    assert!(engine.list_playing_sounds().is_empty());
    if !engine.has_output() {
        assert!(
            engine.play_sound(&write_tone(0.1)).is_err(),
            "no device: an error, not a panic"
        );
    }
}

#[test]
fn test_device_playback_smoke() {
    let mut engine = AudioEngine::new();
    if !engine.has_output() {
        eprintln!("skipping: no audio output device available");
        return;
    }
    let id = engine.play_sound(&write_tone(0.5)).unwrap();
    assert!(engine.is_playing(id));
    engine.stop(id).unwrap();
    assert!(engine.is_stopped(id));
}

#[test]
fn test_playback_lifecycle() {
    let (mut engine, mut output) = offline_audio();
    let id = engine.play_sound(&write_tone(0.2)).unwrap();
    assert!(engine.is_playing(id));
    assert_level(level(&mut output), TONE_PEAK);

    engine.pause(id).unwrap();
    assert!(engine.is_paused(id) && !engine.is_playing(id));
    assert!(level(&mut output) < 1e-6, "silent while paused");

    // Nothing advances while paused: the end of the tone is still to come
    render(&mut output, 0.5);
    engine.update();
    assert!(engine.is_paused(id));

    engine.resume(id).unwrap();
    assert!(engine.is_playing(id));
    render(&mut output, 0.3);
    engine.update();
    assert!(engine.is_stopped(id), "finished sounds are dropped");
}

#[test]
fn test_stop_and_cleanup() {
    let (mut engine, mut output) = offline_audio();
    let tone = write_tone(1.0);
    let first = engine.play_sound(&tone).unwrap();
    let second = engine.play_sound(&tone).unwrap();
    assert_eq!(
        engine.get_memory_usage() as u64,
        std::fs::metadata(&tone).unwrap().len(),
        "both playbacks share one cached copy of the file"
    );

    engine.stop(first).unwrap();
    assert!(engine.is_stopped(first) && engine.is_playing(second));
    assert!(engine.stop(first).is_err(), "already stopped");

    engine.set_master_volume(0.5).unwrap();
    engine.cleanup();
    assert!(engine.is_stopped(second));
    assert_eq!(engine.get_memory_usage(), 0);
    assert_eq!(
        engine.master_volume(),
        1.0,
        "cleanup restores the master volume"
    );
    assert!(level(&mut output) < 1e-6, "silent after cleanup");
}

#[test]
fn test_volume_scales_the_output() {
    let (mut engine, mut output) = offline_audio();
    let half = PlayOptions {
        volume: 0.5,
        ..PlayOptions::default()
    };
    let id = engine.play_sound_with(&write_tone(2.0), half).unwrap();
    assert_eq!(engine.volume(id), Some(0.5));
    assert_level(level(&mut output), 0.5 * TONE_PEAK);

    engine.set_volume(id, 0.25).unwrap();
    assert_level(level(&mut output), 0.25 * TONE_PEAK);

    // The master volume scales every sound, including ones started later
    engine.set_master_volume(0.5).unwrap();
    assert_level(level(&mut output), 0.125 * TONE_PEAK);
    engine.stop(id).unwrap();
    engine.play_sound(&write_tone(2.0)).unwrap();
    assert_level(level(&mut output), 0.5 * TONE_PEAK);
}

#[test]
fn test_looping_sounds_play_until_stopped() {
    let (mut engine, mut output) = offline_audio();
    let tone = write_tone(0.1);
    let looped = engine.play_sound_with(&tone, looping()).unwrap();
    let once = engine.play_sound(&tone).unwrap();

    render(&mut output, 0.5);
    engine.update();
    assert!(engine.is_stopped(once), "a normal sound ends with its file");
    assert!(engine.is_playing(looped), "a looping sound starts over");
    assert_level(level(&mut output), TONE_PEAK);

    engine.stop(looped).unwrap();
    assert!(level(&mut output) < 1e-6);
}

#[test]
fn test_invalid_volumes_are_rejected() {
    let (mut engine, _output) = offline_audio();
    let tone = write_tone(0.1);
    for volume in [-0.5, f32::NAN, f32::INFINITY] {
        let options = PlayOptions {
            volume,
            ..PlayOptions::default()
        };
        assert!(
            engine.play_sound_with(&tone, options).is_err(),
            "volume {}",
            volume
        );
        assert!(
            engine.set_master_volume(volume).is_err(),
            "volume {}",
            volume
        );
    }
    assert_eq!(engine.master_volume(), 1.0);

    let id = engine.play_sound(&tone).unwrap();
    assert!(engine.set_volume(id, -1.0).is_err());
    assert!(
        engine.set_volume(uuid::Uuid::new_v4(), 0.5).is_err(),
        "unknown play id"
    );
}

#[test]
fn test_preview_slot_plays_one_sound_at_a_time() {
    let (mut engine, mut output) = offline_audio();
    let tone = write_tone(1.0);
    engine.play_sound_immediate(&tone).unwrap();
    // Replaces the first preview; both together would be twice as loud
    engine.play_sound_immediate(&tone).unwrap();
    assert_level(level(&mut output), TONE_PEAK);

    engine.stop_immediate();
    assert!(level(&mut output) < 1e-6);
}

#[test]
fn test_audio_duration() {
    // Reads the file header only; works without an audio device
    let engine = AudioEngine::new();
    let duration = engine.get_audio_duration(&write_tone(2.0)).unwrap();
    assert!((duration - 2.0).abs() < 0.01, "duration {}", duration);
    assert!(engine.get_audio_duration(Path::new("missing.wav")).is_err());
}
