use crate::parts::parts::TankProto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Tank {
    pub proto: TankProto,
    pub fuel_mass: f32,
}
