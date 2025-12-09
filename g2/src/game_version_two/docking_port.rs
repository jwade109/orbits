use crate::game_version_two::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct DockingPort {
    pub attached: PortAttachment,
}

#[derive(Debug, Clone, Copy)]
pub enum PortAttachment {
    None,
    Seeking(Entity),
    AttachedTo(Entity),
}

impl DockingPort {
    pub fn detached() -> Self {
        Self {
            attached: PortAttachment::None,
        }
    }

    pub fn attached(e: Entity) -> Self {
        Self {
            attached: PortAttachment::AttachedTo(e),
        }
    }

    pub fn target(&self) -> Option<Entity> {
        match self.attached {
            PortAttachment::None => None,
            PortAttachment::Seeking(entity) => Some(entity),
            PortAttachment::AttachedTo(entity) => Some(entity),
        }
    }
}
