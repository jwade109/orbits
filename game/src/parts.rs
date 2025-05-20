use enum_iterator::Sequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Sequence, Hash)]
pub enum PartLayer {
    Internal,
    Structural,
    Exterior,
}

/// dimensions in meters
#[derive(Debug, Clone, Copy)]
pub struct PartProto {
    pub width: u32,
    pub height: u32,
    pub layer: PartLayer,
    pub path: &'static str,
}

impl PartProto {
    pub const fn new(width: u32, height: u32, layer: PartLayer, path: &'static str) -> Self {
        Self {
            width,
            height,
            layer,
            path,
        }
    }

    pub fn to_z_index(&self) -> f32 {
        match self.layer {
            PartLayer::Internal => 10.0,
            PartLayer::Structural => 11.0,
            PartLayer::Exterior => 12.0,
        }
    }
}

pub const TANK11: PartProto = PartProto::new(10, 10, PartLayer::Internal, "tank11");
pub const TANK21: PartProto = PartProto::new(10, 20, PartLayer::Internal, "tank21");
pub const TANK22: PartProto = PartProto::new(20, 20, PartLayer::Internal, "tank22");
pub const FRAME: PartProto = PartProto::new(10, 10, PartLayer::Structural, "frame");
pub const FRAME2: PartProto = PartProto::new(10, 10, PartLayer::Structural, "frame2");
pub const FRAME22: PartProto = PartProto::new(20, 20, PartLayer::Structural, "frame22");
pub const FRAME3: PartProto = PartProto::new(40, 10, PartLayer::Structural, "frame3");
pub const MOTOR: PartProto = PartProto::new(16, 25, PartLayer::Internal, "motor");
pub const ANTENNA: PartProto = PartProto::new(50, 27, PartLayer::Internal, "antenna");
pub const SMALL_ANTENNA: PartProto = PartProto::new(6, 20, PartLayer::Internal, "small-antenna");
pub const CARGO: PartProto = PartProto::new(30, 30, PartLayer::Internal, "cargo");
pub const BATTERY: PartProto = PartProto::new(9, 9, PartLayer::Internal, "battery");
pub const CPU: PartProto = PartProto::new(8, 9, PartLayer::Internal, "cpu");
pub const SOLARPANEL: PartProto = PartProto::new(65, 16, PartLayer::Internal, "solarpanel");
pub const GOLD: PartProto = PartProto::new(10, 10, PartLayer::Exterior, "gold");
pub const PLATE: PartProto = PartProto::new(10, 10, PartLayer::Exterior, "plate");

pub fn part_sprite_path(short_path: &str) -> String {
    format!("embedded://game/../assets/parts/{}.png", short_path)
}

pub fn find_part(short_path: &str) -> Option<&PartProto> {
    ALL_PARTS.iter().cloned().find(|p| p.path == short_path)
}

pub const ALL_PARTS: [&PartProto; 16] = [
    &TANK11,
    &TANK21,
    &TANK22,
    &FRAME,
    &FRAME2,
    &FRAME22,
    &FRAME3,
    &MOTOR,
    &ANTENNA,
    &SMALL_ANTENNA,
    &CARGO,
    &BATTERY,
    &CPU,
    &SOLARPANEL,
    &GOLD,
    &PLATE,
];
