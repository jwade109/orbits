use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Construction {
    steps_required: u32,
    steps_completed: u32,
}
