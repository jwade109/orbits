#![allow(unused)]

use glam::f64::DVec2;
use glam::i64::I64Vec2;
use glam::u64::U64Vec2;
use std::ops::Sub;

#[derive(Debug, Default, Clone, Copy)]
pub struct GlobalPosition {
    /// Amount of kilometers from the origin.
    kilometers: I64Vec2,

    /// Amount of millimeters from `kilometers`.
    /// This field is always positive.
    millimeters: U64Vec2,
}

impl GlobalPosition {
    fn from_meters_f64(x: f64, y: f64) -> Self {
        let mx = (x * 1000.0) as i64;
        let my = (y * 1000.0) as i64;
        Self::from_millimeters(mx, my)
    }

    fn from_millimeters(x: i64, y: i64) -> Self {
        if x < 0 || y < 0 {
            todo!();
        }

        let mx = x % 1000000;
        let my = y % 1000000;
        let kx = x / 1000000;
        let ky = y / 1000000;
        Self {
            kilometers: I64Vec2::new(kx, ky),
            millimeters: U64Vec2::new(mx as u64, my as u64),
        }
    }

    fn km(&self) -> I64Vec2 {
        self.kilometers
    }

    fn mm(&self) -> U64Vec2 {
        self.millimeters
    }

    fn to_meters_f64(&self) -> DVec2 {
        let kmx = self.kilometers.x;
        let kmy = self.kilometers.y;
        let mmx = self.millimeters.x;
        let mmy = self.millimeters.y;
        DVec2::new(
            kmx as f64 * 1000.0 + mmx as f64 / 1000.0,
            kmy as f64 * 1000.0 + mmy as f64 / 1000.0,
        )
    }
}

impl Sub for GlobalPosition {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            kilometers: self.kilometers - other.kilometers,
            millimeters: self.millimeters.saturating_sub(other.millimeters),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic() {
        let gp = GlobalPosition::from_meters_f64(48923.5, 344.2);
        println!("{:?}", gp);
        println!("{:?}", gp.to_meters_f64());

        let g2 = GlobalPosition::from_millimeters(89322342, 23849234);
        println!("{:?}", g2);
        println!("{:?}", g2.to_meters_f64());

        println!("{:?}", gp - g2);
        println!("{:?}", g2 - gp);
    }
}
