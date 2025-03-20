use crate::math::{rand, rotate, PI};
use crate::orbits::GlobalOrbit;
use crate::prelude::Nanotime;
use crate::region::Region;
use crate::{orbiter::ObjectId, orbits::SparseOrbit};
use glam::f32::Vec2;
use glam::FloatExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct AsteroidBelt {
    parent: ObjectId,
    inner: SparseOrbit,
    outer: SparseOrbit,
}

impl AsteroidBelt {
    pub fn new(parent: ObjectId, inner: SparseOrbit, outer: SparseOrbit) -> Self {
        AsteroidBelt {
            parent,
            inner,
            outer,
        }
    }

    pub fn parent(&self) -> ObjectId {
        self.parent
    }

    pub fn region(&self) -> Region {
        Region::OrbitRange(self.inner, self.outer)
    }

    pub fn radius(&self, angle: f32) -> (f32, f32) {
        (
            self.inner.radius_at_angle(angle),
            self.outer.radius_at_angle(angle),
        )
    }

    pub fn radius_at(&self, angle: f32, s: f32) -> f32 {
        let (rmin, rmax) = self.radius(angle);
        rmin.lerp(rmax, s)
    }

    pub fn random_radius(&self, angle: f32) -> f32 {
        let (rmin, rmax) = self.radius(angle);
        let s = rand(0.0, 1.0);
        rmin.lerp(rmax, s)
    }

    pub fn random_sample(&self) -> Vec2 {
        let angle = rand(0.0, 2.0 * PI);
        let r = self.random_radius(angle);
        rotate(Vec2::X, angle) * r
    }

    pub fn apoapsis(&self, s: f32) -> (f32, f32, f32) {
        let a1 = self.inner.apoapsis();
        let a2 = self.outer.apoapsis();
        let p = a1.lerp(a2, s);
        let angle = Vec2::X.angle_to(p);
        (p.length(), angle, s)
    }

    pub fn random_orbit(&self, epoch: Nanotime) -> Option<SparseOrbit> {
        let (r1, argp, s) = self.apoapsis(rand(0.0, 1.0));
        let r2 = self.radius_at(argp + PI, s);
        let (argp, rp, ra) = if r1 < r2 {
            (argp, r1, r2)
        } else {
            (argp + PI, r2, r1)
        };
        SparseOrbit::new(
            ra,
            rp,
            argp,
            self.inner.body,
            epoch,
            self.inner.is_retrograde(),
        )
    }

    pub fn random_global(&self, epoch: Nanotime) -> Option<GlobalOrbit> {
        Some(GlobalOrbit(self.parent, self.random_orbit(epoch)?))
    }
}
