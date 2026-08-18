use bary_core::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub isometry: Isometry2d,
    /// camera zoom in raylib units, or whatever
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            isometry: Isometry2d::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn world_to_screen(&self, p: Vec2, screen_width: Vec2) -> Vec2 {
        let mut scaled = (p - self.isometry.translation) * self.zoom;
        scaled.y *= -1.0;
        scaled + screen_width / 2.0
    }

    pub fn screen_to_world(&self, p: Vec2, screen_width: Vec2) -> Vec2 {
        let p = p - screen_width / 2.0;
        let p = p.with_y(-p.y);
        p / self.zoom + self.isometry.translation
    }
}
