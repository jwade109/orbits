use crate::factory::Mass;
use crate::math::*;
use crate::parts::computer::ComputerData;
use crate::parts::excavator::ExcavatorData;
use crate::parts::PartLayer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Generic {
    name: String,
    dims: UVec2,
    layer: PartLayer,
    mass: Mass,
    sprites: Option<usize>,
    is_docking_port: Option<bool>,
    excavator_data: Option<ExcavatorData>,
    computer_data: Option<ComputerData>,
}

impl Generic {
    pub fn new(name: String, dims: UVec2, layer: PartLayer, mass: Mass) -> Self {
        Self {
            name,
            dims,
            layer,
            mass,
            sprites: None,
            is_docking_port: None,
            excavator_data: None,
            computer_data: None,
        }
    }

    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn layer(&self) -> PartLayer {
        self.layer
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn sprites(&self) -> usize {
        self.sprites.unwrap_or(1)
    }

    pub fn computer_data(&self) -> Option<&ComputerData> {
        self.computer_data.as_ref()
    }

    pub fn is_docking_port(&self) -> bool {
        self.is_docking_port.unwrap_or(false)
    }

    pub fn excavator_data(&self) -> Option<&ExcavatorData> {
        self.excavator_data.as_ref()
    }
}
