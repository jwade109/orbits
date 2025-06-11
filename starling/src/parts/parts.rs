use enum_iterator::Sequence;
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const PIXELS_PER_METER: f32 = 20.0;

fn part_from_path(path: &Path) -> Result<PartProto, String> {
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
    let data: PartMetaData =
        serde_yaml::from_str(&s).map_err(|e| format!("Failed to parse metadata file: {}", e))?;
    Ok(PartProto::new(img.width(), img.height(), name, data))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Hash, Deserialize, Serialize)]
pub enum PartLayer {
    Internal,
    Structural,
    Exterior,
}

/// dimensions in meters
#[derive(Debug, Clone)]
pub struct PartProto {
    pub width: u32,
    pub height: u32,
    pub path: String,
    pub data: PartMetaData,
}

impl PartProto {
    pub const fn new(width: u32, height: u32, path: String, data: PartMetaData) -> Self {
        Self {
            width,
            height,
            path,
            data,
        }
    }

    pub fn width_meters(&self) -> f32 {
        self.width as f32 / PIXELS_PER_METER
    }

    pub fn height_meters(&self) -> f32 {
        self.height as f32 / PIXELS_PER_METER
    }

    pub fn to_z_index(&self) -> f32 {
        match self.data.layer {
            PartLayer::Internal => 10.0,
            PartLayer::Structural => 11.0,
            PartLayer::Exterior => 12.0,
        }
    }
}

pub fn load_parts_from_dir(path: &Path) -> HashMap<String, PartProto> {
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
pub struct PartMetaData {
    pub mass: f32,
    pub layer: PartLayer,
    pub class: PartClass,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThrusterProto {
    pub model: String,
    pub thrust: f32,
    pub exhaust_velocity: f32,
    pub length: f32,
    pub is_rcs: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TankProto {
    pub wet_mass: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PartClass {
    Thruster(ThrusterProto),
    Tank(TankProto),
    Radar,
    Cargo,
    Undefined,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;

    #[test]
    fn write_thruster_metadata_to_file() {
        let data = PartMetaData {
            mass: 12.0,
            layer: PartLayer::Exterior,
            class: PartClass::Thruster(ThrusterProto {
                model: "RJ1200".into(),
                thrust: 1200.0,
                exhaust_velocity: 3500.0,
                length: 3.4,
                is_rcs: false,
            }),
        };

        let s = serde_yaml::to_string(&data).unwrap();
        std::fs::write("thruster.yaml", s).unwrap();
    }

    #[test]
    fn write_tank_metadata_to_file() {
        let data = PartMetaData {
            mass: 12.0,
            layer: PartLayer::Exterior,
            class: PartClass::Tank(TankProto { wet_mass: 120.0 }),
        };

        let s = serde_yaml::to_string(&data).unwrap();
        std::fs::write("tank.yaml", s).unwrap();
    }
}
