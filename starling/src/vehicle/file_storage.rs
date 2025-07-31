use crate::math::*;
use crate::parts::*;
use crate::vehicle::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleFileStorage {
    pub name: String,
    pub parts: Vec<VehiclePartFileStorage>,
    pub lines: HashSet<IVec2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehiclePartFileStorage {
    pub partname: String,
    pub pos: IVec2,
    pub rot: Rotation,
}

#[derive(Debug)]
pub struct NoPartError(String);

impl std::fmt::Display for NoPartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no part found with the name \"{}\"", self.0)
    }
}

impl std::error::Error for NoPartError {}

pub fn load_vehicle(
    path: &Path,
    name: String,
    parts: &HashMap<String, PartPrototype>,
) -> Result<Vehicle, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(path)?;
    let storage: VehicleFileStorage = serde_yaml::from_str(&s)?;
    let mut prototypes = Vec::new();
    for part in &storage.parts {
        let proto = parts
            .get(&part.partname)
            .ok_or(Box::new(NoPartError(part.partname.clone())))?;
        prototypes.push((part.pos, part.rot, proto.clone()));
    }
    Ok(Vehicle::from_parts(
        name,
        storage.name,
        prototypes,
        storage.lines,
    ))
}

fn part_from_path(path: &Path) -> Result<PartPrototype, String> {
    let data_path = path.join("metadata.yaml");
    let s = std::fs::read_to_string(&data_path).map_err(|_| "Failed to load metadata file")?;
    serde_yaml::from_str(&s).map_err(|e| format!("Failed to parse metadata file: {}", e))
}

pub fn load_parts_from_dir(path: &Path) -> Result<HashMap<String, PartPrototype>, String> {
    let mut ret = HashMap::new();
    if let Ok(paths) = std::fs::read_dir(path) {
        for path in paths {
            if let Ok(path) = path {
                let path = path.path();
                let part = part_from_path(&path)?;
                ret.insert(part.part_name().to_string(), part);
            }
        }
    }
    Ok(ret)
}
