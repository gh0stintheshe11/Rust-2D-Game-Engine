//! Helpers shared by the integration tests.

// Each test file uses a different subset
#![allow(dead_code)]

use rust_2d_game_engine::audio::{AudioEngine, OfflineOutput};
use std::num::NonZero;
use tempfile::TempPath;

/// Sample rate of the tones written by [`write_tone`].
pub const TONE_RATE: u32 = 22_050;

/// Peak amplitude of those tones, as a fraction of full scale.
pub const TONE_PEAK: f32 = 8000.0 / 32768.0;

/// Write a 440 Hz sine tone lasting `seconds` (16-bit mono WAV at
/// [`TONE_RATE`], peaking at [`TONE_PEAK`]) to a new temp file, removed when
/// the returned path is dropped.
pub fn write_tone(seconds: f32) -> TempPath {
    let samples: Vec<i16> = (0..(seconds * TONE_RATE as f32) as u32)
        .map(|i| {
            let t = i as f32 / TONE_RATE as f32;
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
    wav.extend_from_slice(&TONE_RATE.to_le_bytes());
    wav.extend_from_slice(&(TONE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    let path = temp_file("rust2d_tone_", ".wav");
    std::fs::write(&path, wav).unwrap();
    path
}

/// A new empty temp file named `prefix…suffix`, removed when the returned
/// path is dropped.
pub fn temp_file(prefix: &str, suffix: &str) -> TempPath {
    tempfile::Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile()
        .unwrap()
        .into_temp_path()
}

/// A new temp folder named `prefix…`, removed with what it holds when the
/// returned guard is dropped.
pub fn temp_folder(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap()
}

/// An audio engine without a device, mixing into a mono stream at the tones'
/// sample rate.
pub fn offline_audio() -> (AudioEngine, OfflineOutput) {
    AudioEngine::offline(NonZero::new(TONE_RATE).unwrap(), NonZero::new(1).unwrap())
}

/// Read `seconds` of rendered audio; playback advances by that much.
pub fn render(output: &mut OfflineOutput, seconds: f32) -> Vec<f32> {
    output
        .by_ref()
        .take((seconds * TONE_RATE as f32) as usize)
        .collect()
}

/// Peak level of the next 50 ms of output. 10 ms are skipped first: players
/// apply volume, pause and stop requests every 5 ms.
pub fn level(output: &mut OfflineOutput) -> f32 {
    render(output, 0.01);
    render(output, 0.05)
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

/// Assert that a measured level is within 5% of `expected`.
pub fn assert_level(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= expected * 0.05 + 0.002,
        "level {} (expected {})",
        actual,
        expected
    );
}
