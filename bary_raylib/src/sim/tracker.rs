use crate::sim::*;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Tracker {
    origin: VecDeque<Vec2>,
    center_of_mass: VecDeque<Vec2>,
    centroid: VecDeque<Vec2>,
}

impl Tracker {
    pub const MAX_LEN: usize = 500;

    pub fn series(&self) -> [&VecDeque<Vec2>; 3] {
        [&self.origin, &self.center_of_mass, &self.centroid]
    }

    pub fn center_of_mass(&self) -> &VecDeque<Vec2> {
        &self.center_of_mass
    }

    fn enqueue(hist: &mut VecDeque<Vec2>, pose: Isometry2d) {
        hist.push_back(pose.translation);
        if hist.len() > Self::MAX_LEN {
            hist.pop_front();
        }
    }

    pub fn add(&mut self, grid: &VehicleGrid) {
        Self::enqueue(&mut self.origin, grid.origin());
        Self::enqueue(&mut self.center_of_mass, grid.particle_location);
        Self::enqueue(&mut self.centroid, grid.centroid_isometry());
    }
}
