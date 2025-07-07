use crate::aabb::AABB;
use crate::math::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ConnectivityGroup {
    transport_lines: HashSet<IVec2>,
    connections: HashMap<usize, IVec2>,
    bounds: Option<AABB>,
}

impl ConnectivityGroup {
    pub fn new() -> Self {
        Self {
            transport_lines: HashSet::new(),
            connections: HashMap::new(),
            bounds: None,
        }
    }

    pub fn connect(&mut self, idx: usize, pos: IVec2) {
        self.connections.insert(idx, pos);
    }

    pub fn add_transport_line(&mut self, p: IVec2) {
        self.transport_lines.insert(p);
        if let Some(aabb) = &mut self.bounds {
            aabb.include(&p.as_vec2());
            aabb.include(&(p + IVec2::ONE).as_vec2());
        } else {
            self.bounds = Some(AABB::from_arbitrary(
                p.as_vec2(),
                (p + IVec2::ONE).as_vec2(),
            ));
        }
    }

    pub fn points(&self) -> impl Iterator<Item = IVec2> + use<'_> {
        self.connections.iter().map(|(_, p)| *p)
    }

    pub fn is_connected(&self, idx_a: usize, idx_b: usize) -> bool {
        idx_a != idx_b
            && self.connections.contains_key(&idx_a)
            && self.connections.contains_key(&idx_b)
    }

    pub fn bounds(&self) -> Option<AABB> {
        self.bounds
    }
}
