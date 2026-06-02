use bary_parts::ExcavatorPrototype;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Excavator {
    pub radius: f32,
}

impl Excavator {
    pub fn from_proto(ex: &ExcavatorPrototype) -> Self {
        Self { radius: ex.radius }
    }
}
