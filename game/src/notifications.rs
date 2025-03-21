use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Notification {
    pub parent: ObjectId,
    pub offset: Vec2,
    pub jitter: Vec2,
    pub wall_time: Nanotime,
    pub extra_time: Nanotime,
    pub kind: NotificationType,
}

impl Notification {
    pub fn is_duplicate(&self, other: &Self) -> bool {
        self.parent == other.parent
            && self.kind == other.kind
            && ((self.wall_time - other.wall_time).abs() < Nanotime::secs(1))
    }

    pub fn duration(&self) -> Nanotime {
        match self.kind {
            NotificationType::OrbiterCrashed(_) => self.extra_time + Nanotime::secs(10),
            NotificationType::OrbiterDeleted(_) => self.extra_time + Nanotime::secs(5),
            NotificationType::ManeuverStarted(_) => self.extra_time + Nanotime::secs(2),
            NotificationType::ManeuverComplete(_) => self.extra_time + Nanotime::secs(7),
            NotificationType::ManeuverFailed(_) => self.extra_time + Nanotime::secs(3),
            NotificationType::OrbitChanged(_) => self.extra_time + Nanotime::secs(1),
            NotificationType::Following(_) => Nanotime::millis(500),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    OrbiterCrashed(ObjectId),
    OrbiterDeleted(ObjectId),
    ManeuverStarted(ObjectId),
    ManeuverComplete(ObjectId),
    ManeuverFailed(ObjectId),
    Following(ObjectId),
    OrbitChanged(ObjectId),
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] ({}) {:?}", self.wall_time, self.parent, self.kind)
    }
}
