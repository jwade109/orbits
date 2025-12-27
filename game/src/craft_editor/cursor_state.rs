use crate::starling::prelude::*;

#[derive(Debug, Default)]
pub enum CursorState {
    #[default]
    None,
    Part(PartPrototype),
    Blueprint(Blueprint),
}

impl CursorState {
    pub fn current_part(&self) -> Option<PartPrototype> {
        match self {
            Self::Part(proto) => Some(proto.clone()),
            _ => None,
        }
    }

    pub fn blueprint(&self) -> Option<&Blueprint> {
        match self {
            Self::Blueprint(bp) => Some(bp),
            _ => None,
        }
    }
}
