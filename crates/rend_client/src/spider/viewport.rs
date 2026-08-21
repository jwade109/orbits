use bary_sim::Camera;
use glam::DVec2;

pub struct Viewport {
    camera: Camera,
    screen_width: DVec2,
}

impl Viewport {
    pub fn new(camera: Camera, screen_width: DVec2) -> Self {
        Self {
            camera,
            screen_width,
        }
    }

    pub fn world_to_screen(&self, p: impl Into<DVec2>) -> DVec2 {
        self.camera.world_to_screen(p.into(), self.screen_width)
    }

    pub fn screen_to_world(&self, p: impl Into<DVec2>) -> DVec2 {
        self.camera.screen_to_world(p.into(), self.screen_width)
    }

    pub fn zoom(&self) -> f64 {
        self.camera.zoom as f64
    }

    pub fn meters(&self, x: f64) -> f64 {
        self.zoom() * x
    }
}
