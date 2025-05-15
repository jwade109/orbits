/// dimensions in meters
#[derive(Debug, Clone, Copy)]
pub struct PartProto {
    pub width: u32,
    pub height: u32,
    pub path: &'static str,
}

impl PartProto {
    pub const fn new(width: u32, height: u32, path: &'static str) -> Self {
        Self {
            width,
            height,
            path,
        }
    }
}

pub const TANK11: PartProto = PartProto::new(10, 10, "tank11");
pub const TANK21: PartProto = PartProto::new(10, 20, "tank21");
pub const TANK22: PartProto = PartProto::new(18, 18, "tank22");
pub const FRAME: PartProto = PartProto::new(10, 10, "frame");
pub const FRAME2: PartProto = PartProto::new(10, 10, "frame2");
pub const MOTOR: PartProto = PartProto::new(16, 25, "motor");
pub const ANTENNA: PartProto = PartProto::new(50, 27, "antenna");
pub const CARGO: PartProto = PartProto::new(30, 30, "cargo");

pub fn part_sprite_path(short_path: &str) -> String {
    format!("embedded://game/../assets/parts/{}.png", short_path)
}

pub const ALL_PARTS: [&PartProto; 8] = [
    &TANK11, &TANK21, &TANK22, &FRAME, &FRAME2, &MOTOR, &ANTENNA, &CARGO,
];
