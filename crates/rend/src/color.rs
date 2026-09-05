#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

const fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

const fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if (t < 0.0) {
        t += 1.0
    };
    if (t > 1.0) {
        t -= 1.0
    };
    if (t < 1.0 / 6.0) {
        return p + (q - p) * 6.0 * t;
    }
    if (t < 1.0 / 2.0) {
        return q;
    }
    if (t < 2.0 / 3.0) {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }

    return p;
}

impl Color {
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const GRAY: Self = Self::new(0.3, 0.3, 0.3, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BROWN: Self = Self::new(0.4, 0.2, 0.0, 1.0);

    pub const LIGHT_BLUE: Self = Self::BLUE.mix(Self::WHITE, 0.5);

    // "rgb(23, 86, 22)"
    pub const FOREST_GREEN: Self = Self::rgb(23, 86, 22, 1.0);

    // "rgb(255, 90, 0)"
    pub const ORANGE: Self = Self::rgb(255, 90, 0, 1.0);

    // "rgb(137, 255, 251)"
    pub const SKY: Self = Self::rgb(137, 255, 251, 1.0);

    // "rgb(162, 0, 141)"
    pub const PURPLE: Self = Self::rgb(160, 0, 140, 1.0);

    pub const SLATE_GRAY: Self = Self::rgb(112, 128, 144, 1.0);

    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, a)
    }

    pub const fn hsl(h: f64, s: f64, l: f64, a: f64) -> Self {
        if s == 0.0 {
            return Self::new(0.0, 0.0, 0.0, a);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        Self::new(r, g, b, a)
    }

    pub const fn gray(val: f64, a: f64) -> Self {
        Self::new(val, val, val, a)
    }

    pub const fn alpha(self, a: f64) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    pub const fn mix(&self, other: Self, t: f64) -> Self {
        Self::new(
            lerp(self.r, other.r, t),
            lerp(self.g, other.g, t),
            lerp(self.b, other.b, t),
            lerp(self.a, other.a, t),
        )
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
