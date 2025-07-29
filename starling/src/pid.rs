use crate::math::rand;

#[derive(Debug, Clone, Copy)]
pub struct PDCtrl {
    kp: f32,
    kd: f32,
}

impl PDCtrl {
    pub const fn new(kp: f32, kd: f32) -> Self {
        Self { kp, kd }
    }

    pub fn apply(&self, error: f32, error_rate: f32) -> f32 {
        error * self.kp - error_rate * self.kd
    }

    pub fn jitter(&self) -> Self {
        PDCtrl {
            kp: self.kp * rand(0.8, 1.2),
            kd: self.kd * rand(0.8, 1.2),
        }
    }
}
