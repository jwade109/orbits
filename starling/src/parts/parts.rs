use crate::factory::Mass;
use crate::math::*;
use crate::parts::*;
use enum_iterator::Sequence;
// use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// TODO reduce scope of this constant
pub const PIXELS_PER_METER: f32 = 20.0;

fn part_from_path(path: &Path) -> Result<Part, String> {
    // let image_path = path.join("skin.png");
    let data_path = path.join("metadata.yaml");
    // let img = ImageReader::open(&image_path)
    //     .map_err(|_| "Failed to load image")?
    //     .decode()
    //     .map_err(|_| "Failed to decode image")?;
    // let name = path
    //     .file_stem()
    //     .ok_or("Failed to get file stem")?
    //     .to_string_lossy()
    //     .to_string();
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
                        ret.insert(part.part_name().to_string(), part);
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
    Machine(Machine),
    Generic(Generic),
}

impl Part {
    pub fn dims(&self) -> UVec2 {
        match self {
            Self::Thruster(p) => p.dims(),
            Self::Tank(p) => p.dims(),
            Self::Radar(p) => p.dims(),
            Self::Cargo(p) => p.dims(),
            Self::Magnetorquer(p) => p.dims(),
            Self::Generic(p) => p.dims(),
            Self::Machine(p) => p.dims(),
        }
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.dims().as_vec2() / PIXELS_PER_METER
    }

    pub fn part_name(&self) -> &str {
        match self {
            Self::Thruster(p) => p.part_name(),
            Self::Tank(p) => p.part_name(),
            Self::Radar(p) => p.part_name(),
            Self::Cargo(p) => p.part_name(),
            Self::Magnetorquer(p) => p.part_name(),
            Self::Generic(p) => p.part_name(),
            Self::Machine(p) => p.part_name(),
        }
    }

    pub fn current_mass(&self) -> Mass {
        match self {
            Self::Thruster(p) => p.current_mass(),
            Self::Tank(p) => p.current_mass(),
            Self::Radar(p) => p.current_mass(),
            Self::Cargo(p) => p.current_mass(),
            Self::Magnetorquer(p) => p.current_mass(),
            Self::Generic(p) => p.current_mass(),
            Self::Machine(p) => p.current_mass(),
        }
    }

    pub fn dry_mass(&self) -> Mass {
        match self {
            Self::Thruster(p) => p.current_mass(),
            Self::Tank(p) => p.dry_mass(),
            Self::Radar(p) => p.current_mass(),
            Self::Cargo(p) => p.dry_mass(),
            Self::Magnetorquer(p) => p.current_mass(),
            Self::Generic(p) => p.current_mass(),
            Self::Machine(p) => p.current_mass(),
        }
    }

    pub fn layer(&self) -> PartLayer {
        match self {
            Self::Thruster(..) => PartLayer::Internal,
            Self::Tank(..) => PartLayer::Internal,
            Self::Radar(..) => PartLayer::Internal,
            Self::Cargo(..) => PartLayer::Internal,
            Self::Magnetorquer(..) => PartLayer::Internal,
            Self::Generic(p) => p.layer(),
            Self::Machine(..) => PartLayer::Internal,
        }
    }

    pub fn sprite_path(&self) -> &str {
        self.part_name()
    }
}
