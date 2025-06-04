use crate::math::randvec;
use crate::math::Vec2;
use crate::nanotime::Nanotime;
use crate::pv::PV;
use crate::vehicle::{PhysicsMode, Vehicle};

#[derive(Debug)]
pub struct RPO {
    pub stamp: Nanotime,
    pub vehicles: Vec<(PV, Vehicle)>,
}

impl RPO {
    pub fn example(stamp: Nanotime, vehicles: Vec<Vehicle>) -> Self {
        let vehicles = vehicles
            .into_iter()
            .map(|veh| {
                let p = randvec(10.0, 100.0);
                let v = randvec(2.0, 7.0);
                (PV::from_f64(p, v), veh)
            })
            .collect();

        Self { stamp, vehicles }
    }

    pub fn nearest(&self, p: Vec2) -> Option<usize> {
        let mut ret = None;
        let mut d = f32::MAX;
        for (i, (pv, _)) in self.vehicles.iter().enumerate() {
            let di = p.distance(pv.pos_f32());
            if di < d {
                d = di;
                ret = Some(i);
            }
        }
        ret
    }

    pub fn bounding_radius(&self) -> f32 {
        let mut r: f32 = 0.0;
        for (pv, vehicle) in &self.vehicles {
            let d = pv.pos_f32().length() + vehicle.bounding_radius();
            r = r.max(d);
        }
        r
    }

    pub fn step(&mut self, stamp: Nanotime, mode: PhysicsMode) {
        let dt = (stamp - self.stamp).to_secs().clamp(0.0, 0.03);
        for (pv, vehicle) in self.vehicles.iter_mut() {
            vehicle.step(stamp, Vec2::ZERO, mode);

            let perturb = PV::from_f64(pv.vel_f32() * dt, randvec(0.1, 12.0) * dt);
            *pv += perturb;
        }
        self.stamp = stamp;
    }
}
