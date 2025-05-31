use crate::math::{rotate, Vec2};
use crate::parts::parts::ThrusterProto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thruster {
    pub proto: ThrusterProto,
    pub pos: Vec2,
    pub angle: f32,
    pub is_active: bool,
    pub force_active: bool,
}

impl Thruster {
    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }
}
