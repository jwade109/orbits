use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, Hash,
)]
pub struct EntityId(pub i64);

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectId {
    Orbiter(EntityId),
    Planet(EntityId),
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ObjectId {
    pub fn as_orbiter(&self) -> Option<EntityId> {
        match self {
            Self::Orbiter(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_planet(&self) -> Option<EntityId> {
        match self {
            Self::Planet(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_eid(&self) -> EntityId {
        match self {
            ObjectId::Orbiter(e) => *e,
            ObjectId::Planet(e) => *e,
        }
    }
}
