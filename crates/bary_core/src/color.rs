pub struct BColor([u8; 4]);

impl BColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    pub fn to_u8(&self) -> [u8; 4] {
        self.0
    }

    pub fn to_f32(&self) -> [f32; 4] {
        self.0.map(|e| e as f32 / 255.0)
    }
}

pub const BARY_WHITE: BColor = BColor::new(255, 255, 255, 255);
pub const BARY_RED: BColor = BColor::new(255, 0, 0, 255);
pub const BARY_GREEN: BColor = BColor::new(0, 255, 0, 255);
pub const BARY_BLUE: BColor = BColor::new(0, 0, 255, 255);
