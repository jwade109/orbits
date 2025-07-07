use crate::math::IVec2;
use std::collections::HashMap;

pub struct ConnectivityGroup {
    connections: HashMap<usize, IVec2>,
}

impl ConnectivityGroup {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub fn connect(&mut self, idx: usize, pos: IVec2) {
        self.connections.insert(idx, pos);
    }

    pub fn points(&self) -> impl Iterator<Item = IVec2> + use<'_> {
        self.connections.iter().map(|(_, p)| *p)
    }
}
