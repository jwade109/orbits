use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Notification {
    pub parent: ObjectId,
    pub offset: Vec2,
    pub jitter: Vec2,
    pub sim_time: Nanotime,
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
            NotificationType::NotControllable => self.extra_time + Nanotime::secs(1),
            NotificationType::OrbitChanged(_) => self.extra_time + Nanotime::secs(1),
            NotificationType::Following(_) => Nanotime::millis(500),
        }
    }

    pub fn jitter(&mut self) {
        if self.jitter == Vec2::ZERO && rand(0.0, 1.0) < 0.004 {
            self.jitter = randvec(0.1, 6.0);
        } else if self.jitter.length() > 0.0 && rand(0.0, 1.0) < 0.04 {
            self.jitter = Vec2::ZERO;
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    OrbiterCrashed(OrbiterId),
    OrbiterDeleted(OrbiterId),
    ManeuverStarted(OrbiterId),
    ManeuverComplete(OrbiterId),
    ManeuverFailed(OrbiterId),
    Following(ObjectId),
    OrbitChanged(OrbiterId),
    NotControllable,
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] ({}) {:?}", self.sim_time, self.parent, self.kind)
    }
}
