use crate::math::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct Tank {
    pub pos: Vec2,
    pub width: f32,
    pub height: f32,
    pub dry_mass: f32,
    pub current_fuel_mass: f32,
    pub maximum_fuel_mass: f32,
}

impl Tank {
    pub fn percent_filled(&self) -> f32 {
        self.current_fuel_mass / self.maximum_fuel_mass
    }
}
