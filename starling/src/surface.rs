use crate::math::*;

#[derive(Debug)]
pub struct Surface {
    pub gravity: f32,
    pub wind: f32,
    pub radius: f32,
    pub atmo_color: [f32; 3],
    pub land_color: [f32; 3],
}

impl Surface {
    pub fn random() -> Self {
        Surface {
            gravity: 2.0,
            wind: rand(-2.0, 2.0),
            radius: 2000.0,
            atmo_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
            land_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
        }
    }

    pub fn on_sim_tick(&mut self) {
        if self.wind.abs() > 0.0 && self.gravity.abs() > 0.0 && rand(0.0, 1.0) < 0.0002 {
            self.wind = 0.0;
            self.gravity = 0.0;
        }

        if self.wind == 0.0 && self.gravity == 0.0 && rand(0.0, 1.0) < 0.0002 {
            self.wind = rand(-2.0, 2.0);
            self.gravity = rand(1.0, 3.0);
        }

        if self.wind.abs() > 0.0 || self.gravity.abs() > 0.0 {
            self.wind += rand(-0.02, 0.02);
            self.wind = self.wind.clamp(-2.0, 2.0);
            self.gravity += rand(-0.02, 0.02);
            self.gravity = self.gravity.clamp(1.0, 4.0);
        }
    }

    fn gravity_vector(&self) -> Vec2 {
        Vec2::new(0.0, -self.gravity.abs())
    }

    fn wind_vector(&self) -> Vec2 {
        Vec2::new(self.wind, 0.0)
    }

    pub fn external_acceleration(&self) -> Vec2 {
        self.gravity_vector() + self.wind_vector()
    }

    pub fn increase_gravity(&mut self) {
        self.gravity += 0.1;
    }

    pub fn decrease_gravity(&mut self) {
        self.gravity -= 0.1;
        self.gravity = self.gravity.max(0.0);
    }
}
