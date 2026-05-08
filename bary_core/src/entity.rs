use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
pub struct Ent(pub u64);

impl std::fmt::Display for Ent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e{}", self.0)
    }
}

impl std::fmt::Debug for Ent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e{}", self.0)
    }
}

impl std::ops::Add<u64> for Ent {
    type Output = Self;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Sub<u64> for Ent {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        if rhs > self.0 {
            Self(0)
        } else {
            Self(self.0 - rhs)
        }
    }
}
