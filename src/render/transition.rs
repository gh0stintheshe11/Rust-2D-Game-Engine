//! Scene transitions: a color covers the camera frame while the game goes
//! from one scene to the next, fading in or sweeping across.

use egui::{Color32, Painter, Rect};

/// How a transition covers the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    /// The color fades in, then out
    Fade,
    /// The color sweeps across the frame leftward: in from the right edge,
    /// then out past the left one
    WipeLeft,
    WipeRight,
    WipeUp,
    WipeDown,
}

/// Every transition, with the name scripts use for it.
pub const TRANSITIONS: [(TransitionKind, &str); 5] = [
    (TransitionKind::Fade, "fade"),
    (TransitionKind::WipeLeft, "wipe_left"),
    (TransitionKind::WipeRight, "wipe_right"),
    (TransitionKind::WipeUp, "wipe_up"),
    (TransitionKind::WipeDown, "wipe_down"),
];

impl TransitionKind {
    /// The transition named `name`, as listed in [`TRANSITIONS`].
    pub fn parse(name: &str) -> Option<Self> {
        TRANSITIONS
            .iter()
            .find(|(_, candidate)| *candidate == name)
            .map(|(kind, _)| *kind)
    }
}

/// A scene transition: the color covers the frame over the first half of
/// `seconds`, the scene changes behind it, and the new scene shows over
/// the second half.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub kind: TransitionKind,
    pub seconds: f32,
    pub color: Color32,
}

impl Transition {
    /// A fade through black.
    pub fn fade(seconds: f32) -> Self {
        Self {
            kind: TransitionKind::Fade,
            seconds,
            color: Color32::BLACK,
        }
    }

    /// When the scene changes: halfway, with the frame covered.
    pub fn midpoint(&self) -> f32 {
        self.seconds / 2.0
    }

    /// How much of the frame is covered `elapsed` seconds in, from 0 to 1:
    /// rising over the first half, falling over the second.
    pub fn coverage(&self, elapsed: f32) -> f32 {
        let half = self.midpoint();
        if half.is_nan() || half <= 0.0 {
            return 0.0;
        }
        let t = elapsed / half;
        if t <= 1.0 {
            t.max(0.0)
        } else {
            (2.0 - t).max(0.0)
        }
    }

    /// When a transition reaching the same coverage while covering is
    /// `coverage` in: for starting one from where another left off.
    pub fn elapsed_at(&self, coverage: f32) -> f32 {
        coverage.clamp(0.0, 1.0) * self.midpoint()
    }

    /// Paint the transition `elapsed` seconds in over `frame`, the camera
    /// frame on screen.
    pub fn paint(&self, painter: &Painter, frame: Rect, elapsed: f32) {
        let coverage = self.coverage(elapsed);
        if coverage <= 0.0 {
            return;
        }
        let covering = elapsed <= self.midpoint();
        let (width, height) = (frame.width() * coverage, frame.height() * coverage);
        // The covered part: growing from the side it comes in from, then
        // shrinking toward the side it leaves by
        let from_left = Rect::from_min_max(frame.min, egui::pos2(frame.min.x + width, frame.max.y));
        let from_right =
            Rect::from_min_max(egui::pos2(frame.max.x - width, frame.min.y), frame.max);
        let from_top = Rect::from_min_max(frame.min, egui::pos2(frame.max.x, frame.min.y + height));
        let from_bottom =
            Rect::from_min_max(egui::pos2(frame.min.x, frame.max.y - height), frame.max);
        let covered = match (self.kind, covering) {
            (TransitionKind::Fade, _) => {
                painter.rect_filled(frame, 0.0, self.color.gamma_multiply(coverage));
                return;
            }
            (TransitionKind::WipeLeft, true) | (TransitionKind::WipeRight, false) => from_right,
            (TransitionKind::WipeLeft, false) | (TransitionKind::WipeRight, true) => from_left,
            (TransitionKind::WipeUp, true) | (TransitionKind::WipeDown, false) => from_bottom,
            (TransitionKind::WipeUp, false) | (TransitionKind::WipeDown, true) => from_top,
        };
        painter.rect_filled(covered, 0.0, self.color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_cover_then_uncover() {
        let transition = Transition::fade(1.0);
        assert_eq!(transition.coverage(0.0), 0.0);
        assert_eq!(transition.coverage(0.25), 0.5);
        assert_eq!(transition.coverage(0.5), 1.0);
        assert_eq!(transition.coverage(0.75), 0.5);
        assert_eq!(transition.coverage(1.0), 0.0);
        assert_eq!(transition.coverage(2.0), 0.0);
        assert_eq!(Transition::fade(0.0).coverage(0.0), 0.0);
        assert_eq!(transition.elapsed_at(0.5), 0.25);
        for (kind, name) in TRANSITIONS {
            assert_eq!(TransitionKind::parse(name), Some(kind));
        }
        assert_eq!(TransitionKind::parse("spin"), None);
    }
}
