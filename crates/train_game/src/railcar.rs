use bary_core::prelude::Ent;

pub struct RailCar {
    segment: Ent,
    position: f64,
}

impl RailCar {
    pub const WIDTH_METERS: f64 = 4.0;
    pub const LENGTH_METERS: f64 = 14.0;

    pub fn new(segment: Ent, position: f64) -> Self {
        Self { segment, position }
    }
}
