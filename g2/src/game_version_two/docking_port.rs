use crate::game_version_two::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct DockingPort {
    attached_to: Option<Entity>,
}

impl DockingPort {
    pub fn detached() -> Self {
        Self { attached_to: None }
    }

    pub fn attached(e: Entity) -> Self {
        Self {
            attached_to: Some(e),
        }
    }
}
