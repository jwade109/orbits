use bary_factory::{Machine, MachineStatus, RecipeListing};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MachineData {
    pub recipe_categories: Vec<String>,
    pub input_slots: Vec<usize>,
    pub output_slots: Vec<usize>,
}

impl MachineData {
    pub fn into_machine(self) -> Machine {
        Machine {
            enabled: true,
            recipe: RecipeListing::DoNothing,
            steps: 0,
            required_steps: 1000,
            products_finished: 0,
            status: MachineStatus::Off,
            input_slots: self.input_slots,
            output_slots: self.output_slots,
        }
    }
}
