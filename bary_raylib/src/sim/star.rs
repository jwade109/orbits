use crate::components::*;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Star {
    pub pos: Vec3,
    pub alpha: f32,
}

pub fn spawn_stars(spawner: &mut EntitySpawner) -> Components<Star> {
    let n_stars = 4000;
    let mut stars = Components::default();
    for _ in 0..n_stars {
        let pos = randvec(0.0, 10000.0);
        let star = Star {
            pos: pos.extend(rand(0.3, 0.9)),
            alpha: rand(0.5, 1.0),
        };
        let id = spawner.spawn();
        stars.spawn(id, star);
    }
    stars
}
