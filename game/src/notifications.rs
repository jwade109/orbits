use starling::prelude::*;

#[derive(Debug, Clone)]
pub struct Notification {
    pub parent: Option<ObjectId>,
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
            NotificationType::OrbiterEscaped(_) => self.extra_time + Nanotime::secs(10),
            NotificationType::NumericalError(_) => self.extra_time + Nanotime::secs(30),
            NotificationType::OrbiterDeleted(_) => self.extra_time + Nanotime::secs(5),
            NotificationType::ManeuverStarted(_) => self.extra_time + Nanotime::secs(2),
            NotificationType::ManeuverComplete(_) => self.extra_time + Nanotime::secs(7),
            NotificationType::ManeuverFailed(_) => self.extra_time + Nanotime::secs(3),
            NotificationType::NotControllable(_) => self.extra_time + Nanotime::secs(5),
            NotificationType::OrbitChanged(_) => self.extra_time + Nanotime::secs(2),
            NotificationType::Notice(_) => Nanotime::secs(7),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    OrbiterCrashed(OrbiterId),
    OrbiterEscaped(OrbiterId),
    NumericalError(OrbiterId),
    OrbiterDeleted(OrbiterId),
    ManeuverStarted(OrbiterId),
    ManeuverComplete(OrbiterId),
    ManeuverFailed(OrbiterId),
    OrbitChanged(OrbiterId),
    NotControllable(OrbiterId),
    Notice(String),
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OrbiterCrashed(id) => {
                write!(f, "Orbiter {id} crashed")
            }
            Self::OrbiterEscaped(id) => {
                write!(f, "Orbiter {id} escaped the system")
            }
            Self::NumericalError(id) => {
                write!(f, "Orbiter {id} encountered a numerical error")
            }
            Self::OrbiterDeleted(id) => {
                write!(f, "Orbiter {id} was removed from system tracking")
            }
            Self::ManeuverStarted(id) => {
                write!(f, "Orbiter {id} has initiated a mission")
            }
            Self::ManeuverComplete(id) => {
                write!(f, "Orbiter {id} has completed a mission")
            }
            Self::ManeuverFailed(id) => {
                write!(f, "Orbiter {id} failed to execute a maneuver")
            }
            Self::OrbitChanged(id) => {
                write!(f, "Orbiter {id}'s orbit has changed")
            }
            Self::NotControllable(id) => {
                write!(f, "Orbiter {id} is not controllable")
            }
            Self::Notice(str) => {
                write!(f, "Notice: {str}")
            }
        }
    }
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.sim_time, self.kind)
    }
}
