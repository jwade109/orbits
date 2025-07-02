use serde::{Deserialize, Serialize};
use std::iter::Sum;
use std::ops::{Add, AddAssign, SubAssign};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Mass(u64);

impl Mass {
    pub const ZERO: Mass = Mass(0);
    pub const GRAMS_PER_KILOGRAM: u64 = 1_000;
    pub const GRAMS_PER_TON: u64 = 1_000_000;

    pub fn grams(g: u64) -> Self {
        Mass(g)
    }

    pub fn kilograms(kg: u64) -> Self {
        Mass(kg * 1_000)
    }

    pub fn tons(t: u64) -> Self {
        Mass(t * 1_000_000)
    }

    pub fn to_grams(&self) -> u64 {
        self.0
    }

    pub fn to_kg_f32(&self) -> f32 {
        self.0 as f32 / Self::GRAMS_PER_KILOGRAM as f32
    }

    pub fn from_kg_f32(kg: f32) -> Self {
        let grams = (kg.abs() * Self::GRAMS_PER_KILOGRAM as f32).round() as u64;
        Self(grams)
    }

    pub fn clamp(&self, lower: Self, upper: Self) -> Self {
        Mass(self.0.clamp(lower.0, upper.0))
    }
}

impl std::fmt::Display for Mass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1000 {
            write!(f, "{} g", self.0)
        } else if self.0 < 1000000 {
            write!(f, "{:0.1} kg", self.0 as f32 / 1000.0)
        } else {
            write!(f, "{:0.1} t", (self.0 / 1000) as f32 / 1000.0)
        }
    }
}

impl Add for Mass {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Mass(self.0 + rhs.0)
    }
}

impl AddAssign for Mass {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0;
    }
}

impl Sum for Mass {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let sum = iter.map(|e| e.to_grams()).sum();
        Self(sum)
    }
}

impl SubAssign for Mass {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0 - rhs.0;
    }
}
