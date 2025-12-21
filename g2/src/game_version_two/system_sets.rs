use crate::game_version_two::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Sets {
    Input,
    PrePhysics,
    Physics,
    PostPhysics,
    Draw,
    Misc,
}
