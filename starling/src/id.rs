use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Hash)]
pub struct OrbiterId(pub i64);

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Hash)]
pub struct VehicleId(pub i64);

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Hash)]
pub struct PlanetId(pub i64);

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Hash)]
pub struct GroupId(pub String);

impl std::fmt::Display for OrbiterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::Debug for OrbiterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::Display for VehicleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Debug for VehicleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Display for PlanetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Debug for PlanetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{}", self.0)
    }
}

impl std::fmt::Debug for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ":{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectId {
    Planet(PlanetId),
    Orbiter(OrbiterId),
}

impl ObjectId {
    pub fn orbiter(&self) -> Option<OrbiterId> {
        match self {
            Self::Orbiter(id) => Some(*id),
            _ => None,
        }
    }

    pub fn planet(&self) -> Option<PlanetId> {
        match self {
            Self::Planet(id) => Some(*id),
            _ => None,
        }
    }
}

impl From<OrbiterId> for ObjectId {
    fn from(value: OrbiterId) -> Self {
        ObjectId::Orbiter(value)
    }
}

impl From<PlanetId> for ObjectId {
    fn from(value: PlanetId) -> Self {
        ObjectId::Planet(value)
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
