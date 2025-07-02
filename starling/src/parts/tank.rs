use crate::factory::Mass;
use crate::math::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct Tank {
    pub pos: Vec2,
    pub width: f32,
    pub height: f32,
    pub dry_mass: Mass,
    pub current_fuel_mass: Mass,
    pub maximum_fuel_mass: Mass,
}

impl Tank {
    pub fn percent_filled(&self) -> f32 {
        self.current_fuel_mass.to_grams() as f32 / self.maximum_fuel_mass.to_grams() as f32
    }
}
