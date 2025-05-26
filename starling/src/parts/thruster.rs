use crate::math::{rotate, Vec2};
use crate::parts::parts::ThrusterProto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Thruster {
    pub proto: ThrusterProto,
    pub pos: Vec2,
    pub angle: f32,
    pub is_active: bool,
}

impl Thruster {
    pub fn main(pos: Vec2, angle: f32, length: f32) -> Self {
        let proto = ThrusterProto {
            thrust: 400.0,
            isp: 500.0,
            length,
            is_rcs: false,
        };
        Self {
            proto,
            pos,
            angle,
            is_active: false,
        }
    }

    pub fn rcs(pos: Vec2, angle: f32) -> Self {
        let proto = ThrusterProto {
            thrust: 30.0,
            isp: 300.0,
            length: 0.1,
            is_rcs: false,
        };
        Self {
            proto,
            pos,
            angle,
            is_active: false,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }
}
