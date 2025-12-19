use crate::factory::Mass;
use crate::math::*;
use crate::parts::PartLayer;
use crate::parts::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Generic {
    name: String,
    dims: UVec2,
    layer: PartLayer,
    mass: Mass,
    sprites: Option<usize>,
    docking_port_data: Option<DockingPortData>,
    excavator_data: Option<ExcavatorData>,
    computer_data: Option<ComputerData>,
    inventory_data: Option<InventoryData>,
    thruster_data: Option<ThrusterModel>,
}

impl Generic {
    pub fn new(name: String, dims: UVec2, layer: PartLayer, mass: Mass) -> Self {
        Self {
            name,
            dims,
            layer,
            mass,
            sprites: None,
            docking_port_data: None,
            excavator_data: None,
            computer_data: None,
            inventory_data: None,
            thruster_data: None,
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

    pub fn docking_port_data(&self) -> Option<&DockingPortData> {
        self.docking_port_data.as_ref()
    }

    pub fn excavator_data(&self) -> Option<&ExcavatorData> {
        self.excavator_data.as_ref()
    }

    pub fn inventory_data(&self) -> Option<&InventoryData> {
        self.inventory_data.as_ref()
    }

    pub fn thruster_data(&self) -> Option<&ThrusterModel> {
        self.thruster_data.as_ref()
    }
}
