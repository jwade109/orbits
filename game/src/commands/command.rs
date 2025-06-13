use crate::game::GameState;

pub trait Command: Send + Sync {
    fn apply(&self, state: &GameState) -> Option<()>;
}
