use crate::math::randvec;
use crate::math::Vec2;
use crate::nanotime::Nanotime;
use crate::pv::PV;
use crate::vehicle::Vehicle;

#[derive(Debug)]
pub struct RPO {
    pub stamp: Nanotime,
    pub vehicles: Vec<(PV, Vehicle)>,
}

impl RPO {
    pub fn example(stamp: Nanotime) -> Self {
        let vehicles = (0..12)
            .map(|_| {
                let p = randvec(10.0, 100.0);
                let v = randvec(2.0, 7.0);
                (PV::new(p, v), Vehicle::random(stamp))
            })
            .collect();

        Self { stamp, vehicles }
    }

    pub fn nearest(&self, p: Vec2) -> Option<usize> {
        let mut ret = None;
        let mut d = f32::MAX;
        for (i, (pv, _)) in self.vehicles.iter().enumerate() {
            let di = p.distance(pv.pos);
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
            let d = pv.pos.length() + vehicle.bounding_radius();
            r = r.max(d);
        }
        r
    }

    pub fn step(&mut self, stamp: Nanotime) {
        let dt = (stamp - self.stamp).to_secs();
        for (pv, vehicle) in self.vehicles.iter_mut() {
            vehicle.step(stamp);
            pv.pos += pv.vel * dt;
            pv.vel *= 0.999;
            pv.vel += randvec(0.1, 12.0) * dt;
        }
        self.stamp = stamp;
    }
}
