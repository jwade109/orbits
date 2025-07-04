use crate::factory::Mass;
use crate::parts::{TankModel, ThrusterModel};
use enum_iterator::Sequence;
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// TODO reduce scope of this constant
pub const PIXELS_PER_METER: f32 = 20.0;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PartFileStorage {
    pub mass: Mass,
    pub layer: PartLayer,
    pub class: PartDefinitionVariant,
}

fn part_from_path(path: &Path) -> Result<PartDefinition, String> {
    let image_path = path.join("skin.png");
    let data_path = path.join("metadata.yaml");
    let img = ImageReader::open(&image_path)
        .map_err(|_| "Failed to load image")?
        .decode()
        .map_err(|_| "Failed to decode image")?;
    let name = path
        .file_stem()
        .ok_or("Failed to get file stem")?
        .to_string_lossy()
        .to_string();
    let s = std::fs::read_to_string(&data_path).map_err(|_| "Failed to load metadata file")?;
    let data: PartFileStorage =
        serde_yaml::from_str(&s).map_err(|e| format!("Failed to parse metadata file: {}", e))?;

    let proto = PartDefinition {
        width: img.width(),
        height: img.height(),
        path: name,
        mass: data.mass,
        layer: data.layer,
        class: data.class,
    };

    Ok(proto)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Hash, Deserialize, Serialize)]
pub enum PartLayer {
    Internal,
    Structural,
    Exterior,
}

#[derive(Debug, Clone)]
pub struct PartDefinition {
    pub width: u32,
    pub height: u32,
    pub path: String,
    pub mass: Mass,
    pub layer: PartLayer,
    pub class: PartDefinitionVariant,
}

impl PartDefinition {
    pub fn width_meters(&self) -> f32 {
        self.width as f32 / PIXELS_PER_METER
    }

    pub fn height_meters(&self) -> f32 {
        self.height as f32 / PIXELS_PER_METER
    }

    pub fn to_z_index(&self) -> f32 {
        match self.layer {
            PartLayer::Internal => 10.0,
            PartLayer::Structural => 11.0,
            PartLayer::Exterior => 12.0,
        }
    }
}

pub fn load_parts_from_dir(path: &Path) -> HashMap<String, PartDefinition> {
    let mut ret = HashMap::new();
    if let Ok(paths) = std::fs::read_dir(path) {
        for path in paths {
            if let Ok(path) = path {
                let path = path.path();
                match part_from_path(&path) {
                    Ok(part) => {
                        ret.insert(part.path.clone(), part);
                    }
                    Err(e) => {
                        println!("Error loading part {}: {}", path.display(), e);
                        continue;
                    }
                }
            }
        }
    }

    ret
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PartDefinitionVariant {
    Thruster(ThrusterModel),
    Tank(TankModel),
    Radar,
    Cargo,
    Undefined,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::Item;
    use serde_yaml;

    #[test]
    fn write_thruster_metadata_to_file() {
        let data = PartFileStorage {
            mass: Mass::kilograms(12),
            layer: PartLayer::Exterior,
            class: PartDefinitionVariant::Thruster(ThrusterModel {
                model: "RJ1200".into(),
                thrust: 1200.0,
                exhaust_velocity: 3500.0,
                length: 3.4,
                width: 1.6,
                is_rcs: false,
                throttle_rate: 2.0,
                primary_color: [0.5, 0.6, 0.9, 0.8],
                secondary_color: [0.4, 0.7, 0.4, 1.0],
            }),
        };

        let s = serde_yaml::to_string(&data).unwrap();
        std::fs::write("thruster.yaml", s).unwrap();
    }

    #[test]
    fn write_tank_metadata_to_file() {
        let data = PartFileStorage {
            mass: Mass::kilograms(12),
            layer: PartLayer::Exterior,
            class: PartDefinitionVariant::Tank(TankModel {
                wet_mass: Mass::kilograms(120),
                item: Item::H2,
            }),
        };

        let s = serde_yaml::to_string(&data).unwrap();
        std::fs::write("tank.yaml", s).unwrap();
    }
}
