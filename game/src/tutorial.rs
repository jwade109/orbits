use crate::goals::*;

pub struct Tutorial {
    goals: Vec<Goal>,
    current: usize,
}

impl Tutorial {
    pub fn new(goals: Vec<Goal>) -> Self {
        Self { goals, current: 0 }
    }

    pub fn current(&self) -> &Goal {
        self.goals
            .get(self.current)
            .expect("Expected non-empty goals list")
    }
}
