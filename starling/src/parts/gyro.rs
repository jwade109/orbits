use crate::prelude::PHYSICS_CONSTANT_DELTA_TIME;

#[derive(Debug, Clone, Copy)]
pub struct Gyro {
    pub target_velocity: f64,
    pub angular_velocity: f64,
    pub max_angular_velocity: f64,
    pub angular_acceleration: f64,
    pub moment_of_inertia: f64,
}

impl Gyro {
    pub fn new() -> Self {
        Self {
            target_velocity: 0.0,
            angular_velocity: 0.0,
            max_angular_velocity: 500.0,
            angular_acceleration: 0.0,
            moment_of_inertia: 30.0,
        }
    }

    pub fn increase_speed_by(&mut self, sp: f64) {
        self.target_velocity += sp;
        self.target_velocity = self
            .target_velocity
            .clamp(-self.max_angular_velocity, self.max_angular_velocity);
    }

    pub fn step(&mut self) {
        let delta = (self.target_velocity - self.angular_velocity).clamp(-2.0, 2.0);
        let old = self.angular_velocity;
        self.angular_velocity += delta;
        self.angular_velocity = self
            .angular_velocity
            .clamp(-self.max_angular_velocity, self.max_angular_velocity);
        self.target_velocity = self.angular_velocity;
        let da = self.angular_velocity - old;
        self.angular_acceleration = da / PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
    }

    pub fn saturation(&self) -> f64 {
        self.angular_velocity.abs() / self.max_angular_velocity
    }

    pub fn current_torque(&self) -> f64 {
        self.angular_acceleration * self.moment_of_inertia
    }
}
