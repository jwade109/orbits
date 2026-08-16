#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const GRAY: Self = Self::new(0.3, 0.3, 0.3, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BROWN: Self = Self::new(0.4, 0.2, 0.0, 1.0);
    pub const ORANGE: Self = Self::new(1.0, 0.3, 0.0, 1.0);

    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, a)
    }

    pub const fn gray(val: f64, a: f64) -> Self {
        Self::new(val, val, val, a)
    }

    pub const fn to_vec(&self) -> glm::Vec4 {
        glm::Vec4 {
            x: self.r as f32,
            y: self.g as f32,
            z: self.b as f32,
            w: self.a as f32,
        }
    }

    pub const fn to_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}
