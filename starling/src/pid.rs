use crate::math::rand;

#[derive(Debug, Clone, Copy)]
pub struct PDCtrl {
    kp: f64,
    kd: f64,
}

impl PDCtrl {
    pub const fn new(kp: f64, kd: f64) -> Self {
        Self { kp, kd }
    }

    pub fn apply(&self, error: f64, error_rate: f64) -> f64 {
        error * self.kp - error_rate * self.kd
    }

    pub fn jitter(&self) -> Self {
        PDCtrl {
            kp: self.kp * rand(0.8, 1.2) as f64,
            kd: self.kd * rand(0.8, 1.2) as f64,
        }
    }
}
