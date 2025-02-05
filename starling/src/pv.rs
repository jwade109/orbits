use bevy::math::Vec2;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PV {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl PV {
    pub fn zero() -> Self {
        PV {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
        }
    }

    pub fn new(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        PV {
            pos: pos.into(),
            vel: vel.into(),
        }
    }

    pub fn pos(pos: impl Into<Vec2>) -> Self {
        PV::new(pos, Vec2::ZERO)
    }

    pub fn vel(vel: impl Into<Vec2>) -> Self {
        PV::new(Vec2::ZERO, vel)
    }
}

impl std::fmt::Display for PV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "P({:0.3}, {:0.3}) V({:0.3}, {:0.3})",
            self.pos.x, self.pos.y, self.vel.x, self.vel.y
        )
    }
}

impl std::ops::Add for PV {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        PV::new(self.pos + other.pos, self.vel + other.vel)
    }
}

impl std::ops::Sub for PV {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        PV::new(self.pos - other.pos, self.vel - other.vel)
    }
}

impl Into<PV> for ((f32, f32), (f32, f32)) {
    fn into(self) -> PV {
        let r: Vec2 = self.0.into();
        let v: Vec2 = self.1.into();
        PV::new(r, v)
    }
}

impl Into<PV> for (Vec2, Vec2) {
    fn into(self) -> PV {
        PV::new(self.0, self.1)
    }
}
