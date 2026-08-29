use std::collections::BTreeMap;

use bary_core::prelude::{Ent, chance, rand, randint};

use crate::{track::Terminus, world::World};

pub struct RailCar {
    pub segment: Ent,
    pub pos: f64,
    pub origin: Terminus,
    pub vel: f64,
}

impl RailCar {
    pub const WIDTH_METERS: f64 = 4.0;
    pub const LENGTH_METERS: f64 = 14.0;

    pub fn new(segment: Ent, pos: f64, vel: f64, origin: Terminus) -> Self {
        Self {
            segment,
            pos,
            vel,
            origin,
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.pos += self.vel * dt;
    }
}

pub fn get_next_track(world: &World, at_node: Ent, source_track: Ent) -> Option<(Ent, Terminus)> {
    let node = world.nodes.get(at_node)?;

    let select = |c: &BTreeMap<Ent, Terminus>| {
        if c.is_empty() {
            return None;
        }
        let n = c.len();
        let v: Vec<_> = c.iter().collect();
        let i = randint(0, n as i32);
        v.get(i as usize).map(|e| (*e.0, *e.1))
    };

    if node.forward_connections.contains_key(&source_track) {
        return select(&node.backward_connections);
    }

    if node.backward_connections.contains_key(&source_track) {
        return select(&node.forward_connections);
    }

    None
}

pub fn spawn_new_car(world: &mut World, segment_id: Ent) -> Option<()> {
    let track = world.segments.get(segment_id)?;
    let n = 1;
    let s = track.length * bary_core::prelude::rand(0.1, 0.9) as f64;
    let vel = 1400.0;
    let d = RailCar::LENGTH_METERS + 3.0;

    for i in 0..n {
        let l = s + d * i as f64;
        if l > track.length {
            break;
        }

        let origin = if chance(0.5) {
            Terminus::Start
        } else {
            Terminus::End
        };

        let car = RailCar::new(segment_id, l, vel, origin);
        let id = world.spawner.spawn();
        world.cars.spawn(id, car);
    }

    Some(())
}
