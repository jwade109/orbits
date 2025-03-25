use crate::aabb::OBB;
use crate::math::{rand, rotate, PI};
use crate::orbits::{Body, GlobalOrbit};
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
    pub fn new(
        parent: ObjectId,
        argp: f32,
        rp: f32,
        ra: f32,
        w: f32,
        body: Body,
        retrograde: bool,
    ) -> Option<Self> {
        let rp1 = rp - w / 2.0;
        let ra1 = ra - w / 2.0;
        let rp2 = rp + w / 2.0;
        let ra2 = ra + w / 2.0;
        let inner = SparseOrbit::new(ra1, rp1, argp, body, Nanotime::zero(), retrograde)?;
        let outer = SparseOrbit::new(ra2, rp2, argp, body, Nanotime::zero(), retrograde)?;
        Some(Self::from_orbits(parent, inner, outer))
    }

    pub fn from_orbit(orbit: GlobalOrbit, w: f32) -> Option<Self> {
        let ra = orbit.1.apoapsis_r();
        let rp = orbit.1.periapsis_r();
        let rp1 = rp - w / 2.0;
        let ra1 = ra - w / 2.0;
        let rp2 = rp + w / 2.0;
        let ra2 = ra + w / 2.0;
        let inner = SparseOrbit::new(
            ra1,
            rp1,
            orbit.1.arg_periapsis,
            orbit.1.body(),
            Nanotime::zero(),
            orbit.1.is_retrograde(),
        )?;
        let outer = SparseOrbit::new(
            ra2,
            rp2,
            orbit.1.arg_periapsis,
            orbit.1.body(),
            Nanotime::zero(),
            orbit.1.is_retrograde(),
        )?;
        Some(Self::from_orbits(orbit.0, inner, outer))
    }

    pub fn circular(
        parent: ObjectId,
        inner: f32,
        outer: f32,
        body: Body,
        retrograde: bool,
    ) -> Self {
        let inner = SparseOrbit::circular(inner, body, Nanotime::zero(), retrograde);
        let outer = SparseOrbit::circular(outer, body, Nanotime::zero(), retrograde);
        Self {
            parent,
            inner,
            outer,
        }
    }

    pub fn from_orbits(parent: ObjectId, inner: SparseOrbit, outer: SparseOrbit) -> Self {
        assert!(!inner.is_hyperbolic());
        assert!(!outer.is_hyperbolic());
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

    pub fn position(&self, angle: f32) -> (Vec2, Vec2) {
        let (rmin, rmax) = self.radius(angle);
        let u = rotate(Vec2::X, angle);
        (u * rmin, u * rmax)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let angle = Vec2::X.angle_to(p);
        let (rmin, rmax) = self.radius(angle);
        let r = p.length();
        r >= rmin && r <= rmax
    }

    pub fn contains_orbit(&self, other: &SparseOrbit) -> bool {
        self.contains(other.periapsis())
            && self.contains(other.apoapsis())
            && self.contains(other.position_at(0.5 * PI))
            && self.contains(other.position_at(1.5 * PI))
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

    pub fn apoapsis(&self, s: f32) -> (f32, f32) {
        let a1 = self.inner.apoapsis();
        let a2 = self.outer.apoapsis();
        let p = a1.lerp(a2, s);
        let angle = Vec2::X.angle_to(p);
        (p.length(), angle)
    }

    pub fn random_orbit(&self, epoch: Nanotime) -> Option<SparseOrbit> {
        let (r1, argp) = self.apoapsis(rand(0.0, 1.0));
        let r2 = self.random_radius(argp + PI);
        let (argp, rp, ra) = if r1 < r2 {
            (argp, r1, r2)
        } else {
            (argp + PI, r2, r1)
        };
        SparseOrbit::new(
            ra,
            rp,
            argp,
            self.inner.body(),
            epoch,
            self.inner.is_retrograde(),
        )
    }

    pub fn random_global(&self, epoch: Nanotime) -> Option<GlobalOrbit> {
        Some(GlobalOrbit(self.parent, self.random_orbit(epoch)?))
    }

    pub fn obb(&self) -> OBB {
        self.outer.obb().unwrap()
    }
}
