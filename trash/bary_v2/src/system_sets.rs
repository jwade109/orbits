use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct KeybindsSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SimulationSet {
    Thruster,
    Misc,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct DrawSet;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CameraSet;
