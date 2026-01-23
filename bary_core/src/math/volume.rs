use std::iter::Sum;
use std::ops::*;

// strong type representing volume.
// the underlying integer is the volume in microliters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Volume(u64);

impl Volume {
    pub const ZERO: Volume = Volume(0);
    pub const MICROLITERS_PER_MILLILITER: u64 = 1_000;
    pub const MICROLITERS_PER_LITER: u64 = 1_000_000;
    pub const LITERS_PER_CUBIC_METER: u64 = 1_000;
    pub const MICROLITERS_PER_CUBIC_METER: u64 = 1_000_000_000;

    pub fn microliters(ul: u64) -> Self {
        Self(ul)
    }

    pub fn milliliters(ml: u64) -> Self {
        Self(ml * Self::MICROLITERS_PER_MILLILITER)
    }

    pub fn milliliters_f64(ml: f64) -> Self {
        Self((ml * Self::MICROLITERS_PER_MILLILITER as f64).round() as u64)
    }

    pub fn liters(l: u64) -> Self {
        Self(l * Self::MICROLITERS_PER_LITER)
    }

    pub fn liters_f32(l: f32) -> Self {
        let milliliters = (l.max(0.0) * 1000.0).round() as u64;
        Self(milliliters * Self::MICROLITERS_PER_MILLILITER)
    }

    pub fn to_microliters(&self) -> u64 {
        self.0
    }

    pub fn clamp(&self, lower: Self, upper: Self) -> Self {
        Volume(self.0.clamp(lower.0, upper.0))
    }
}

impl std::fmt::Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1000 {
            write!(f, "{} uL", self.0)
        } else if self.0 < 1000000 {
            write!(f, "{:0.1} mL", self.0 as f64 / 1000.0)
        } else if self.0 < 1000000000{
            write!(f, "{:0.1} L", (self.0 / 1000) as f64 / 1000.0)
        } else {
            write!(f, "{:0.1} kL", (self.0 / 1000000) as f64 / 1000.0)
        }
    }
}

impl Add for Volume {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Volume(self.0 + rhs.0)
    }
}

impl AddAssign for Volume {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0;
    }
}

impl Sum for Volume {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let sum = iter.map(|e| e.to_microliters()).sum();
        Self(sum)
    }
}

impl Sub for Volume {
    type Output = Volume;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Volume {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0 - rhs.0;
    }
}

impl PartialOrd for Volume {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Mul<u64> for Volume {
    type Output = Self;
    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<Volume> for u64 {
    type Output = Volume;
    fn mul(self, rhs: Volume) -> Self::Output {
        Volume(rhs.0 * self)
    }
}

impl Div<Volume> for Volume {
    type Output = f64;
    fn div(self, rhs: Volume) -> Self::Output {
        self.0 as f64 / rhs.0 as f64
    }
}
