// Transform component for positioning, scaling, and rotating
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: (f32, f32),
    pub rotation: f32, // In radians
    pub scale: (f32, f32),
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            rotation: 0.0,
            scale: (1.0, 1.0),
        }
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }

    pub fn with_rotation(mut self, angle: f32) -> Self {
        self.rotation = angle;
        self
    }

    pub fn with_scale(mut self, sx: f32, sy: f32) -> Self {
        self.scale = (sx, sy);
        self
    }

    // Helper for uniform scaling
    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = (scale, scale);
        self
    }
}
