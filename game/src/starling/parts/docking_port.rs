use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct DockingPortData {
    pub radius: f32,
    pub distance: f32,
}
