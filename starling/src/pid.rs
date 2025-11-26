use crate::math::rand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
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

    pub fn jitter(&self, magnitude: f32) -> Self {
        PDCtrl {
            kp: self.kp * rand(1.0 / magnitude, magnitude) as f64,
            kd: self.kd * rand(1.0 / magnitude, magnitude) as f64,
        }
    }
}


#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct PIDCtrl {
    kp: f64,
    ki: f64,
    kd: f64,
    last_error: Option<f64>,
}

impl PIDCtrl {
    pub const fn new(kp: f64, ki:f64, kd: f64) -> Self {
        Self { kp, ki, kd, last_error: None }
    }

    pub fn apply(&mut self, error_integrated: f64, error: f64) -> f64 {
        let last_error = match self.last_error {
            Some(x) => x,
            None => error,
        };
        const GAME_UPDATE_TIMESTEP: f64 = 1.0 / 60.0;
        let error_rate = (error - last_error) * GAME_UPDATE_TIMESTEP;
        self.last_error = Some(error);
        -error * self.kp + error_integrated * self.ki - error_rate * self.kd
    }
}
