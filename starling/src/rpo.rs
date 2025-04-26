use std::collections::HashMap;

use crate::math::randvec;
use crate::nanotime::Nanotime;
use crate::orbiter::OrbiterId;
use crate::pv::PV;
use crate::vehicle::Vehicle;

pub struct RPO {
    pub stamp: Nanotime,
    pub vehicles: HashMap<OrbiterId, (PV, Vehicle)>,
}

impl RPO {
    pub fn example() -> Self {
        let stamp = Nanotime::zero();
        let vehicles = (0..12)
            .map(|i| {
                let p = randvec(10.0, 100.0);
                let v = randvec(2.0, 7.0);
                (OrbiterId(i), (PV::new(p, v), Vehicle::random(stamp)))
            })
            .collect();

        Self { stamp, vehicles }
    }

    pub fn step(&mut self, stamp: Nanotime) {
        let dt = (stamp - self.stamp).to_secs();
        for (pv, vehicle) in self.vehicles.values_mut() {
            vehicle.step(stamp);
            pv.pos += pv.vel * dt;
        }
        self.stamp = stamp;
    }
}
