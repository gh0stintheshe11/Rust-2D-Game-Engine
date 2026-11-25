/// 2D view: `position` is the world point at the viewport's top-left corner,
/// `zoom` the number of screen pixels per world unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub position: (f32, f32),
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            zoom: 1.0,
        }
    }

    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.position.0 += dx;
        self.position.1 += dy;
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
    }

    pub fn world_to_screen(&self, world_pos: (f32, f32)) -> (f32, f32) {
        (
            (world_pos.0 - self.position.0) * self.zoom,
            (world_pos.1 - self.position.1) * self.zoom,
        )
    }

    /// The inverse of [`world_to_screen`](Self::world_to_screen): the world
    /// point at a viewport-relative screen position.
    pub fn screen_to_world(&self, screen_pos: (f32, f32)) -> (f32, f32) {
        let zoom = self.zoom.max(f32::MIN_POSITIVE);
        (
            screen_pos.0 / zoom + self.position.0,
            screen_pos.1 / zoom + self.position.1,
        )
    }

    pub fn reset(&mut self) {
        self.position = (0.0, 0.0);
        self.zoom = 1.0;
    }
}

/// How a [`CameraRig`] follows an entity. Distances are in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFollow {
    /// Added to the point followed: aims the camera ahead of or above it.
    pub offset: (f32, f32),
    /// How far behind the camera lags, in seconds: the time it takes to get
    /// about two thirds of the way to where it's going. 0 keeps the point
    /// followed in the middle of the frame.
    pub smoothing: f32,
    /// Width and height of the dead zone: a box in the middle of the frame
    /// the point followed moves in without the camera moving.
    pub deadzone: (f32, f32),
    /// An area the frame stays inside, as `(x, y, width, height)`: the
    /// level, say, so the camera doesn't show what's beyond its edges. A
    /// frame larger than the area is centered on it.
    pub bounds: Option<(f32, f32, f32, f32)>,
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self {
            offset: (0.0, 0.0),
            smoothing: 0.0,
            deadzone: (0.0, 0.0),
            bounds: None,
        }
    }
}

/// Most shakes a [`CameraRig`] keeps at once; a new one beyond replaces
/// the weakest.
pub const MAX_CAMERA_SHAKES: usize = 16;

/// A shake: `strength` world units at first, down to nothing after
/// `seconds`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shake {
    strength: f32,
    seconds: f32,
    elapsed: f32,
}

impl Shake {
    fn amplitude(&self) -> f32 {
        self.strength * (1.0 - self.elapsed / self.seconds).max(0.0)
    }
}

/// Moves a scene's camera every frame: following an entity (smoothly, with
/// a dead zone, inside bounds) and shaking. The shake is added to where the
/// camera would otherwise be, and taken off again the next frame, so it
/// never builds up; when something else moved the camera in between (a
/// script), the rig starts from there instead.
#[derive(Clone, Debug, Default)]
pub struct CameraRig {
    follow: Option<(uuid::Uuid, CameraFollow)>,
    /// Jump to the point followed at the next step, smoothing or not
    snap: bool,
    shakes: Vec<Shake>,
    /// Where the rig last put the camera, and where it was without the
    /// shake
    written: Option<((f32, f32), (f32, f32))>,
    /// Seconds stepped, which the shake's motion follows
    time: f64,
}

impl CameraRig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Follow an entity from the next step on. With `snap`, the camera jumps
    /// to it at once; otherwise it gets there as `smoothing` says.
    pub fn follow(&mut self, target: uuid::Uuid, options: CameraFollow, snap: bool) {
        self.follow = Some((target, options));
        self.snap = snap;
    }

    /// Stop following; the camera stays where it is.
    pub fn stop_following(&mut self) {
        self.follow = None;
    }

    /// The entity followed, if any.
    pub fn following(&self) -> Option<uuid::Uuid> {
        self.follow.map(|(target, _)| target)
    }

    /// Shake the camera by up to `strength` world units, fading out over
    /// `seconds`. Shakes at the same time don't add up: the strongest
    /// counts.
    pub fn shake(&mut self, strength: f32, seconds: f32) {
        if !(strength > 0.0 && seconds > 0.0) {
            return;
        }
        let shake = Shake {
            strength,
            seconds,
            elapsed: 0.0,
        };
        if self.shakes.len() < MAX_CAMERA_SHAKES {
            self.shakes.push(shake);
        } else if let Some(weakest) = self
            .shakes
            .iter_mut()
            .min_by(|a, b| a.amplitude().total_cmp(&b.amplitude()))
            .filter(|weakest| weakest.amplitude() < strength)
        {
            *weakest = shake;
        }
    }

    /// How strongly the camera shakes now, in world units.
    pub fn shake_amplitude(&self) -> f32 {
        self.shakes.iter().map(Shake::amplitude).fold(0.0, f32::max)
    }

    /// Whether a step would do anything: the rig follows something or
    /// shakes, or must take its last shake off the camera.
    pub fn is_active(&self) -> bool {
        self.follow.is_some()
            || !self.shakes.is_empty()
            || self
                .written
                .is_some_and(|(position, base)| position != base)
    }

    /// Forget everything: what's followed, the shakes, where the camera was
    /// put (a new scene, say).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One frame of `dt` seconds: where the camera's center goes, from
    /// `camera`, where it is now. `target` is where the entity followed is
    /// (`None` when it's gone: the rig stops following it), and
    /// `frame_size` the frame's size in world units, which `bounds` keep
    /// inside.
    pub fn step(
        &mut self,
        dt: f32,
        camera: (f32, f32),
        target: Option<(f32, f32)>,
        frame_size: (f32, f32),
    ) -> (f32, f32) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.time += f64::from(dt);
        // Without last frame's shake (exactly: no rounding builds up),
        // unless something else moved the camera since
        let mut base = match self.written {
            Some((written, base)) if written == camera => base,
            _ => camera,
        };

        if let Some((_, options)) = self.follow {
            match target {
                Some(target) => {
                    let aim = (target.0 + options.offset.0, target.1 + options.offset.1);
                    let mut goal = (
                        dead_zone(base.0, aim.0, options.deadzone.0 / 2.0),
                        dead_zone(base.1, aim.1, options.deadzone.1 / 2.0),
                    );
                    if let Some(bounds) = options.bounds {
                        goal = inside(goal, frame_size, bounds);
                    }
                    let share = if self.snap || options.smoothing <= 0.0 {
                        1.0
                    } else {
                        1.0 - (-dt / options.smoothing).exp()
                    };
                    base = (
                        base.0 + (goal.0 - base.0) * share,
                        base.1 + (goal.1 - base.1) * share,
                    );
                    if let Some(bounds) = options.bounds {
                        base = inside(base, frame_size, bounds);
                    }
                    self.snap = false;
                }
                None => self.follow = None,
            }
        }

        for shake in &mut self.shakes {
            shake.elapsed += dt;
        }
        self.shakes.retain(|shake| shake.elapsed < shake.seconds);
        let amplitude = self.shake_amplitude();
        let offset = if amplitude > 0.0 {
            let wobble = wobble(self.time);
            (wobble.0 * amplitude, wobble.1 * amplitude)
        } else {
            (0.0, 0.0)
        };
        let position = (base.0 + offset.0, base.1 + offset.1);
        self.written = Some((position, base));
        position
    }
}

/// Where the center goes on one axis so `aim` is at most `half` from it.
fn dead_zone(center: f32, aim: f32, half: f32) -> f32 {
    if aim > center + half {
        aim - half
    } else if aim < center - half {
        aim + half
    } else {
        center
    }
}

/// A frame center moved so the frame of `size` stays inside `bounds`
/// (`x, y, width, height`), or centered on them where it's larger.
fn inside(center: (f32, f32), size: (f32, f32), bounds: (f32, f32, f32, f32)) -> (f32, f32) {
    let axis = |center: f32, size: f32, start: f32, length: f32| {
        if size >= length {
            start + length / 2.0
        } else {
            center.clamp(start + size / 2.0, start + length - size / 2.0)
        }
    };
    (
        axis(center.0, size.0, bounds.0, bounds.2),
        axis(center.1, size.1, bounds.1, bounds.3),
    )
}

/// The shake's motion at `time`: a jitter from -1 to 1 on each axis, made
/// of waves at unrelated frequencies so it doesn't look regular.
fn wobble(time: f64) -> (f32, f32) {
    use std::f64::consts::TAU;
    let x = 0.6 * (TAU * 17.3 * time).sin() + 0.4 * (TAU * 27.1 * time + 1.3).sin();
    let y = 0.6 * (TAU * 19.7 * time + 0.7).sin() + 0.4 * (TAU * 23.9 * time + 2.1).sin();
    (x as f32, y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: (f32, f32) = (400.0, 300.0);

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn following_locks_on_or_lags_behind() {
        let target = uuid::Uuid::new_v4();
        let mut rig = CameraRig::new();
        rig.follow(target, CameraFollow::default(), false);
        let at = rig.step(1.0 / 60.0, (0.0, 0.0), Some((100.0, 50.0)), FRAME);
        assert!(close(at, (100.0, 50.0)), "{:?}", at);

        let smooth = CameraFollow {
            smoothing: 0.5,
            offset: (0.0, -20.0),
            ..Default::default()
        };
        rig.follow(target, smooth, false);
        let at = rig.step(0.5, (0.0, 0.0), Some((100.0, 70.0)), FRAME);
        // One time constant: about 63% of the way
        assert!(close(at, (63.212, 31.606)), "{:?}", at);
        // The same in many small steps (frame rate independent)
        rig.follow(target, smooth, false);
        let mut camera = (0.0, 0.0);
        for _ in 0..30 {
            camera = rig.step(0.5 / 30.0, camera, Some((100.0, 70.0)), FRAME);
        }
        assert!(close(camera, (63.212, 31.606)), "{:?}", camera);
        // Snapping jumps there whatever the smoothing
        rig.follow(target, smooth, true);
        let at = rig.step(1.0 / 60.0, (0.0, 0.0), Some((100.0, 70.0)), FRAME);
        assert!(close(at, (100.0, 50.0)), "{:?}", at);

        // The target is gone: the camera stays
        let at = rig.step(1.0 / 60.0, at, None, FRAME);
        assert!(close(at, (100.0, 50.0)));
        assert_eq!(rig.following(), None);
    }

    #[test]
    fn the_dead_zone_and_bounds_hold_the_camera() {
        let mut rig = CameraRig::new();
        let options = CameraFollow {
            deadzone: (100.0, 60.0),
            bounds: Some((0.0, 0.0, 1000.0, 300.0)),
            ..Default::default()
        };
        rig.follow(uuid::Uuid::new_v4(), options, false);
        // Within 50 / 30 of the center: no move
        let at = rig.step(0.1, (500.0, 150.0), Some((540.0, 170.0)), FRAME);
        assert!(close(at, (500.0, 150.0)), "{:?}", at);
        // Beyond: just enough to keep it at the dead zone's edge (the frame
        // is as high as the bounds, so it stays centered on them)
        let at = rig.step(0.1, at, Some((600.0, 400.0)), FRAME);
        assert!(close(at, (550.0, 150.0)), "{:?}", at);
        // Near the left end, the frame stops at the bounds
        let at = rig.step(0.1, at, Some((-500.0, 150.0)), FRAME);
        assert!(close(at, (200.0, 150.0)), "{:?}", at);
        // A frame wider than the bounds is centered on them
        let narrow = CameraFollow {
            bounds: Some((0.0, 0.0, 100.0, 1000.0)),
            ..Default::default()
        };
        rig.follow(uuid::Uuid::new_v4(), narrow, false);
        let at = rig.step(0.1, at, Some((900.0, 900.0)), FRAME);
        assert!(close(at, (50.0, 850.0)), "{:?}", at);
    }

    #[test]
    fn shakes_fade_and_never_build_up() {
        let mut rig = CameraRig::new();
        rig.shake(10.0, 0.5);
        rig.shake(4.0, 2.0);
        let mut camera = (100.0, 100.0);
        let mut moved = false;
        for frame in 0..60 {
            camera = rig.step(1.0 / 60.0, camera, None, FRAME);
            let off = ((camera.0 - 100.0).abs(), (camera.1 - 100.0).abs());
            let limit = if frame < 30 { 10.0 } else { 4.0 };
            assert!(
                off.0 <= limit && off.1 <= limit,
                "frame {}: {:?}",
                frame,
                off
            );
            moved |= off.0 > 1.0;
        }
        assert!(moved, "it shook");
        // Over: back where it was, exactly
        for _ in 0..90 {
            camera = rig.step(1.0 / 60.0, camera, None, FRAME);
        }
        assert_eq!(camera, (100.0, 100.0));
        assert!(rig.shakes.is_empty() && !rig.is_active());

        // A script moving the camera mid-shake moves where it shakes around
        rig.shake(5.0, 1.0);
        let shaken = rig.step(1.0 / 60.0, camera, None, FRAME);
        let at = rig.step(1.0 / 60.0, (300.0, 300.0), None, FRAME);
        assert!((at.0 - 300.0).abs() <= 5.0 && (at.1 - 300.0).abs() <= 5.0);
        assert_ne!(shaken, at);

        // Invalid shakes are ignored; there's a cap
        let mut rig = CameraRig::new();
        rig.shake(-1.0, 1.0);
        rig.shake(1.0, 0.0);
        rig.shake(f32::NAN, 1.0);
        assert!(!rig.is_active());
        for strength in 1..=(MAX_CAMERA_SHAKES as u32 + 4) {
            rig.shake(strength as f32, 1.0);
        }
        assert_eq!(rig.shakes.len(), MAX_CAMERA_SHAKES);
        assert_eq!(rig.shake_amplitude(), (MAX_CAMERA_SHAKES + 4) as f32);
    }
}
