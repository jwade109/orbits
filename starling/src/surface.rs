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
            gravity: rand(2.0, 7.0),
            wind: rand(-3.0, 3.0),
            radius: 2000.0,
            atmo_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
            land_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
        }
    }
}
