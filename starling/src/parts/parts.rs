use crate::factory::Mass;
use crate::math::*;
use crate::parts::*;
use enum_iterator::Sequence;
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// TODO reduce scope of this constant
pub const PIXELS_PER_METER: f32 = 20.0;

fn part_from_path(path: &Path) -> Result<Part, String> {
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
    serde_yaml::from_str(&s).map_err(|e| format!("Failed to parse metadata file: {}", e))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Hash, Deserialize, Serialize)]
pub enum PartLayer {
    Internal,
    Structural,
    Exterior,
}

pub fn load_parts_from_dir(path: &Path) -> HashMap<String, Part> {
    let mut ret = HashMap::new();
    if let Ok(paths) = std::fs::read_dir(path) {
        for path in paths {
            if let Ok(path) = path {
                let path = path.path();
                match part_from_path(&path) {
                    Ok(part) => {
                        ret.insert(part.name().to_string(), part);
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
pub enum Part {
    Thruster(Thruster),
    Tank(Tank),
    Radar(Radar),
    Cargo(Cargo),
    Magnetorquer(Magnetorquer),
    Undefined,
}

impl Part {
    pub fn dims(&self) -> UVec2 {
        todo!()
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.dims().as_vec2() / PIXELS_PER_METER
    }

    pub fn name(&self) -> &str {
        todo!()
    }

    pub fn mass(&self) -> Mass {
        todo!()
    }

    pub fn layer(&self) -> PartLayer {
        todo!()
    }

    pub fn sprite_path(&self) -> &str {
        todo!()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::factory::Item;
//     use serde_yaml;

//     #[test]
//     fn write_thruster_metadata_to_file() {
//         let data = Part::Thruster(Thruster {
//                 model: "RJ1200".into(),
//                 thrust: 1200.0,
//                 exhaust_velocity: 3500.0,
//                 length: 3.4,
//                 width: 1.6,
//                 is_rcs: false,
//                 throttle_rate: 2.0,
//                 primary_color: [0.5, 0.6, 0.9, 0.8],
//                 secondary_color: [0.4, 0.7, 0.4, 1.0],
//             }),
//         };

//         let s = serde_yaml::to_string(&data).unwrap();
//         std::fs::write("thruster.yaml", s).unwrap();
//     }

//     #[test]
//     fn write_tank_metadata_to_file() {
//         let data = PartFileStorage {
//             mass: Mass::kilograms(12),
//             layer: PartLayer::Exterior,
//             class: PartModel::Tank(TankModel {
//                 wet_mass: Mass::kilograms(120),
//                 item: Item::H2,
//             }),
//         };

//         let s = serde_yaml::to_string(&data).unwrap();
//         std::fs::write("tank.yaml", s).unwrap();
//     }
// }
