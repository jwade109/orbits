use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct ExcavatorPrototype {
    pub radius: f32,
}
