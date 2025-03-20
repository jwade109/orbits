use starling::prelude::*;

pub struct Notification {
    pub parent: ObjectId,
    pub offset: Vec2,
    pub jitter: Vec2,
    pub wall_time: Nanotime,
    pub duration: Nanotime,
    pub kind: NotificationType,
}

pub enum NotificationType {
    OrbiterCrashed,
    OrbiterDeleted,
    ManeuverStarted,
    ManeuverComplete,
    ManeuverFailed,
    Following,
    OrbitChanged,
}
