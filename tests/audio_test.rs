//! Audio engine tests. Playback runs on an offline engine (mixed into a
//! sample stream instead of a sound card), so the tests measure what is
//! actually heard on every machine, including CI.

mod common;

use common::{TONE_PEAK, assert_level, level, offline_audio, render, write_tone};
use rust_2d_game_engine::audio::{
    AudioEngine, DEFAULT_GROUP, MUSIC_GROUP, MusicOptions, PlayOptions,
};
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

/// Peak level of the next `seconds` of output.
fn peak(output: &mut rust_2d_game_engine::audio::OfflineOutput, seconds: f32) -> f32 {
    render(output, seconds)
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

#[test]
fn test_sounds_fade_in_and_out_smoothly() {
    let (mut engine, mut output) = offline_audio();
    let fading_in = PlayOptions {
        fade_in: 0.4,
        ..PlayOptions::default()
    };
    let id = engine.play_sound_with(&write_tone(3.0), fading_in).unwrap();
    assert_eq!(engine.volume(id), Some(1.0), "where it's going");
    // Rising steadily from silence: a quarter of the way after 0.1 s, half
    // after 0.2 s, all of it after 0.4 s
    let first = peak(&mut output, 0.05);
    assert!(first < 0.15 * TONE_PEAK, "{}", first);
    render(&mut output, 0.045);
    let quarter = peak(&mut output, 0.01);
    render(&mut output, 0.09);
    let half = peak(&mut output, 0.01);
    for (level, expected) in [(quarter, 0.25), (half, 0.5)] {
        assert!(
            (level / TONE_PEAK - expected).abs() < 0.06,
            "{} for {}",
            level / TONE_PEAK,
            expected
        );
    }
    render(&mut output, 0.2);
    assert_level(level(&mut output), TONE_PEAK);

    // Down to a quarter over 0.2 s, a little lower every 10 ms
    engine.fade(id, 0.25, 0.2).unwrap();
    assert_eq!(engine.volume(id), Some(0.25));
    let mut last = f32::MAX;
    for _ in 0..20 {
        let now = peak(&mut output, 0.01);
        assert!(now <= last + 1e-4, "{} after {}", now, last);
        last = now;
    }
    assert_level(level(&mut output), 0.25 * TONE_PEAK);

    // Faded out, then stopped: it plays until it's silent
    engine.stop_with_fade(id, 0.2).unwrap();
    render(&mut output, 0.1);
    engine.update();
    assert!(engine.is_playing(id), "still fading out");
    render(&mut output, 0.15);
    engine.update();
    assert!(engine.is_stopped(id), "stopped once silent");
    assert!(level(&mut output) < 1e-6);
    assert!(engine.fade(id, 0.5, 1.0).is_err(), "it's gone");

    let id = engine.play_sound(&write_tone(1.0)).unwrap();
    assert!(engine.fade(id, -1.0, 1.0).is_err());
    assert!(engine.fade(id, 0.5, f32::NAN).is_err());
    assert!(engine.stop_with_fade(id, -1.0).is_err());
    let backwards = PlayOptions {
        fade_in: -1.0,
        ..PlayOptions::default()
    };
    assert!(engine.play_sound_with(&write_tone(1.0), backwards).is_err());
    // No time: stopped at once
    engine.stop_with_fade(id, 0.0).unwrap();
    assert!(engine.is_stopped(id));

    // Faded to silence already: nothing is left to fade, so it stops at once
    // (a fade out from silence would never end it)
    let id = engine.play_sound_with(&write_tone(2.0), looping()).unwrap();
    engine.fade(id, 0.0, 0.05).unwrap();
    render(&mut output, 0.1);
    engine.stop_with_fade(id, 1.0).unwrap();
    assert!(engine.is_stopped(id));
}

#[test]
fn test_volume_groups_scale_their_sounds() {
    let (mut engine, mut output) = offline_audio();
    let music = PlayOptions {
        group: "music".to_string(),
        looping: true,
        ..PlayOptions::default()
    };
    let tone = write_tone(0.5);
    let id = engine.play_sound_with(&tone, music.clone()).unwrap();
    assert_level(level(&mut output), TONE_PEAK);
    engine.set_group_volume("music", 0.5).unwrap();
    assert_level(level(&mut output), 0.5 * TONE_PEAK);
    engine.set_group_volume(DEFAULT_GROUP, 0.1).unwrap();
    assert_level(level(&mut output), 0.5 * TONE_PEAK);
    engine.set_master_volume(0.5).unwrap();
    assert_level(level(&mut output), 0.25 * TONE_PEAK);

    // Sounds started later play at their group's volume too
    engine.stop(id).unwrap();
    engine.set_master_volume(1.0).unwrap();
    let id = engine.play_sound_with(&tone, music).unwrap();
    assert_level(level(&mut output), 0.5 * TONE_PEAK);
    engine.stop(id).unwrap();
    engine.play_sound_with(&tone, looping()).unwrap();
    assert_level(level(&mut output), 0.1 * TONE_PEAK);

    assert_eq!(engine.group_volume("music"), 0.5);
    assert_eq!(engine.group_volume("voices"), 1.0, "not set");
    assert!(engine.set_group_volume("", 0.5).is_err());
    assert!(engine.set_group_volume(&"x".repeat(65), 0.5).is_err());
    assert!(engine.set_group_volume("music", -1.0).is_err());
    let nameless = PlayOptions {
        group: String::new(),
        ..PlayOptions::default()
    };
    assert!(engine.play_sound_with(&tone, nameless).is_err());

    engine.cleanup();
    assert_eq!(engine.group_volume("music"), 1.0, "cleanup restores them");
}

#[test]
fn test_music_crossfades_from_one_track_to_the_next() {
    let (mut engine, mut output) = offline_audio();
    let (first, second) = (write_tone(0.5), write_tone(0.7));
    let a = engine.play_music(&first, MusicOptions::default()).unwrap();
    assert_eq!(engine.music(), Some(a));
    assert_level(level(&mut output), TONE_PEAK);
    render(&mut output, 1.0);
    engine.update();
    assert!(engine.is_playing(a), "music loops");

    // The same track plays on, at the new volume
    let quieter = MusicOptions {
        volume: 0.5,
        ..MusicOptions::default()
    };
    assert_eq!(engine.play_music(&first, quieter).unwrap(), a);
    assert_level(level(&mut output), 0.5 * TONE_PEAK);

    // Another one fades in while the first fades out, then stops
    let crossfade = MusicOptions {
        crossfade: 0.3,
        ..MusicOptions::default()
    };
    let b = engine.play_music(&second, crossfade).unwrap();
    assert_ne!(a, b);
    assert_eq!(engine.music(), Some(b));
    render(&mut output, 0.15);
    engine.update();
    assert!(
        engine.is_playing(a) && engine.is_playing(b),
        "both, halfway"
    );
    render(&mut output, 0.2);
    engine.update();
    assert!(engine.is_stopped(a), "the first stopped once silent");
    assert_level(level(&mut output), TONE_PEAK);

    // Music plays in the music group
    engine.set_group_volume(MUSIC_GROUP, 0.5).unwrap();
    assert_level(level(&mut output), 0.5 * TONE_PEAK);

    engine.stop_music(0.1).unwrap();
    assert_eq!(engine.music(), None);
    render(&mut output, 0.2);
    engine.update();
    assert!(engine.is_stopped(b));
    assert!(level(&mut output) < 1e-6);
    engine.stop_music(0.0).unwrap();
    assert!(engine.stop_music(-1.0).is_err());
}
