//! Volumes that fade smoothly: the game sets where a playback's volume is
//! going and how fast, and the audio thread moves it there sample by sample,
//! so fades are free of steps and clicks whatever the frame rate.

use rodio::source::SeekError;
use rodio::{ChannelCount, Sample, SampleRate, Source};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::time::Duration;

/// Seconds a volume set without a fade takes to get there anyway: a jump
/// in volume mid-wave would click.
pub(super) const DECLICK_SECONDS: f32 = 0.005;

/// A playback's own volume, shared by the game and the audio thread.
#[derive(Debug)]
pub(super) struct Gain {
    /// Where the volume is going (f32 bits)
    target: AtomicU32,
    /// How fast it gets there, in volume per second (f32 bits)
    rate: AtomicU32,
    /// Where it is, as the audio thread last left it (f32 bits)
    current: AtomicU32,
    /// The playback ends once its volume reached 0 and stays there: a fade
    /// out that stops it
    stop_when_silent: AtomicBool,
}

impl Gain {
    pub(super) fn new(volume: f32) -> Arc<Self> {
        Arc::new(Self {
            target: AtomicU32::new(volume.to_bits()),
            rate: AtomicU32::new(f32::INFINITY.to_bits()),
            current: AtomicU32::new(volume.to_bits()),
            stop_when_silent: AtomicBool::new(false),
        })
    }

    /// Go to `volume` over `seconds`, linearly from where the volume is now
    /// (at least [`DECLICK_SECONDS`]).
    pub(super) fn fade_to(&self, volume: f32, seconds: f32) {
        let distance = (volume - self.current()).abs();
        let rate = distance / seconds.max(DECLICK_SECONDS);
        // The rate first: the audio thread may pick the target up at once
        self.rate.store(rate.to_bits(), Relaxed);
        self.target.store(volume.to_bits(), Relaxed);
    }

    pub(super) fn target(&self) -> f32 {
        f32::from_bits(self.target.load(Relaxed))
    }

    pub(super) fn current(&self) -> f32 {
        f32::from_bits(self.current.load(Relaxed))
    }

    pub(super) fn set_stop_when_silent(&self, stop: bool) {
        self.stop_when_silent.store(stop, Relaxed);
    }

    pub(super) fn stops_when_silent(&self) -> bool {
        self.stop_when_silent.load(Relaxed)
    }
}

/// A source played at the volume its [`Gain`] says.
pub(super) struct Faded<S> {
    source: S,
    gain: Arc<Gain>,
    /// The volume now
    volume: f32,
}

impl<S: Source> Faded<S> {
    pub(super) fn new(source: S, gain: Arc<Gain>) -> Self {
        Self {
            volume: gain.current(),
            source,
            gain,
        }
    }

    /// Move the volume one sample's worth toward `target`. Returns false
    /// when the playback is to end: silent, at the end of a fade out that
    /// stops it.
    #[cold]
    fn ramp(&mut self, target: f32) -> bool {
        let rate = f32::from_bits(self.gain.rate.load(Relaxed));
        // Every channel's sample moves it: channels drift apart by a step
        // at most, far too little to hear
        let samples_per_second =
            self.source.sample_rate().get() as f32 * f32::from(self.source.channels().get());
        let step = rate / samples_per_second;
        let distance = target - self.volume;
        self.volume = if distance.abs() <= step {
            target
        } else {
            self.volume + step.copysign(distance)
        };
        self.gain.current.store(self.volume.to_bits(), Relaxed);
        !(self.volume == 0.0 && target == 0.0 && self.gain.stops_when_silent())
    }
}

impl<S: Source> Iterator for Faded<S> {
    type Item = Sample;

    #[inline]
    fn next(&mut self) -> Option<Sample> {
        // A load and a comparison per sample, unless the volume moves
        let target = self.gain.target();
        if self.volume != target && !self.ramp(target) {
            return None;
        }
        let sample = self.source.next()?;
        Some(sample * self.volume)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}

impl<S: Source> Source for Faded<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.source.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.source.try_seek(position)
    }
}
