use uom::si::f32::{Length, Mass, Time, Velocity};
use uom::si::{length::kilometer, mass::yottagram, time::minute};

#[derive(Debug, Clone, Copy)]
pub struct CanonicalUnits {
    pub du: Length,
    pub tu: Time,
    pub mu: Mass,
}

impl CanonicalUnits {
    pub fn vu(&self) -> Velocity {
        self.du / self.tu
    }
}

fn from_yottagrams(yottagrams: f32) -> Mass {
    Mass::new::<yottagram>(yottagrams)
}

fn from_minutes(minutes: f32) -> Time {
    Time::new::<minute>(minutes)
}

fn from_kilometers(kilometers: f32) -> Length {
    Length::new::<kilometer>(kilometers)
}

pub fn earth_moon_canonical_units() -> CanonicalUnits {
    CanonicalUnits {
        du: from_kilometers(6378.145),
        tu: from_minutes(13.44686457),
        mu: from_yottagrams(5972.2),
    }
}
