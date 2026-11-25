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
