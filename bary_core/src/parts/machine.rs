use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MachineData {
    pub recipe_categories: Vec<String>,
    pub input_slots: Vec<usize>,
    pub output_slots: Vec<usize>,
}
