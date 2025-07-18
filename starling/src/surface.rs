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
            wind: rand(-3.0, 3.0),
            radius: 2000.0,
            atmo_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
            land_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
        }
    }

    pub fn gravity_vector(&self) -> Vec2 {
        Vec2::new(0.0, -self.gravity)
    }

    pub fn increase_gravity(&mut self) {
        self.gravity += 0.1;
    }

    pub fn decrease_gravity(&mut self) {
        self.gravity -= 0.1;
        self.gravity = self.gravity.max(0.0);
    }
}
