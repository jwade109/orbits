use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleFileStorage {
    pub parts: Vec<VehiclePartFileStorage>,
    pub pipes: Option<Vec<PipeFileStorage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehiclePartFileStorage {
    pub partname: String,
    pub pos: PartCoord,
    pub rot: Rotation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeFileStorage {
    pub geometry: PipeGeometry,
}

#[derive(Debug)]
pub struct NoPartError(String);

impl std::fmt::Display for NoPartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no part found with the name \"{}\"", self.0)
    }
}

impl std::error::Error for NoPartError {}

pub fn load_blueprint(
    id: &BlueprintId,
    assets_dir: impl AsRef<Path>,
    parts: &BTreeMap<String, PartPrototype>,
) -> Result<Blueprint, Box<dyn std::error::Error>> {
    let old_path = assets_dir
        .as_ref()
        .join("vehicles")
        .join(format!("{}.vehicle", id.0));

    let new_path = assets_dir
        .as_ref()
        .join("vehicles")
        .join(format!("{}/v{}.bp", id.0, id.1));

    if old_path.exists() {
        load_blueprint_file(old_path, parts)
    } else {
        load_blueprint_file(new_path, parts)
    }
}

pub fn load_blueprint_file(
    path: impl AsRef<Path>,
    parts: &BTreeMap<String, PartPrototype>,
) -> Result<Blueprint, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(path)?;
    let storage: VehicleFileStorage = serde_yaml::from_str(&s)?;
    let mut prototypes = Vec::new();
    for part in &storage.parts {
        let proto = parts
            .get(&part.partname)
            .ok_or(Box::new(NoPartError(part.partname.clone())))?;
        prototypes.push((part.pos, part.rot, proto.clone()));
    }
    let pipes = storage
        .pipes
        .unwrap_or(vec![])
        .iter()
        .map(|p| p.geometry)
        .collect();
    Ok(Blueprint::from_parts(prototypes, pipes))
}

fn part_from_path(path: &Path) -> Result<PartPrototype, String> {
    let data_path = path.join("metadata.yaml");
    let s = std::fs::read_to_string(&data_path).map_err(|_| "Failed to load metadata file")?;
    serde_yaml::from_str(&s)
        .map_err(|e| format!("Failed to parse metadata file at {}: {}", path.display(), e))
}

pub fn load_parts_from_dir(
    path: impl AsRef<Path>,
) -> Result<BTreeMap<String, PartPrototype>, String> {
    let mut ret = BTreeMap::new();
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

pub fn save_blueprint(path: impl AsRef<Path>, blueprint: &Blueprint) -> Result<(), BaryError> {
    let parts = blueprint
        .parts()
        .map(|(_, instance)| VehiclePartFileStorage {
            partname: instance.name.clone(),
            pos: instance.origin(),
            rot: instance.rotation(),
        })
        .collect();

    let pipes = blueprint
        .pipes()
        .map(|(_, pipe)| PipeFileStorage { geometry: *pipe })
        .collect();

    let storage = VehicleFileStorage {
        parts,
        pipes: Some(pipes),
    };

    let s = serde_yaml::to_string(&storage)?;
    std::fs::write(path.as_ref(), s)?;
    Ok(())
}
