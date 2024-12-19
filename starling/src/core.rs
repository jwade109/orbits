use crate::propagator::*;
use crate::canonical::*;
use bevy::math::Vec2;
use rand::Rng;
use std::time::Duration;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
}

pub fn anomaly_e2m(ecc: f32, eccentric_anomaly: f32) -> f32 {
    eccentric_anomaly - ecc * f32::sin(eccentric_anomaly)
}

pub fn anomaly_m2e(ecc: f32, mean_anomaly: f32) -> Option<f32> {
    let max_error = 1E-6;
    let max_iters = 1000;

    let mut e = mean_anomaly;

    for _ in 0..max_iters {
        e = e - (mean_anomaly - e + ecc * e.sin()) / (ecc * e.cos() - 1.0);
        if (mean_anomaly - e + ecc * e.sin()).abs() < max_error {
            return Some(e);
        }
    }

    None
}

pub fn anomaly_t2e(ecc: f32, true_anomaly: f32) -> f32 {
    f32::atan2(
        f32::sin(true_anomaly) * (1.0 - ecc.powi(2)).sqrt(),
        f32::cos(true_anomaly) + ecc,
    )
}

pub fn anomaly_e2t(ecc: f32, eccentric_enomaly: f32) -> f32 {
    f32::atan2(
        f32::sin(eccentric_enomaly) * (1.0 - ecc.powi(2)).sqrt(),
        f32::cos(eccentric_enomaly) - ecc,
    )
}

pub fn anomaly_t2m(ecc: f32, true_anomaly: f32) -> f32 {
    anomaly_e2m(ecc, anomaly_t2e(ecc, true_anomaly))
}

pub fn anomaly_m2t(ecc: f32, mean_anomaly: f32) -> Option<f32> {
    anomaly_m2e(ecc, mean_anomaly).map(|e| anomaly_e2t(ecc, e))
}

pub const GRAVITATIONAL_CONSTANT: f32 = 12000.0;

#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub radius: f32,
    pub mass: f32,
    pub soi: f32,
}

impl Body {
    pub fn new(radius: f32, mass: f32, soi: f32) -> Self {
        Body { radius, mass, soi }
    }

    pub fn mu(&self) -> f32 {
        self.mass * GRAVITATIONAL_CONSTANT
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub eccentricity: f32,
    pub semi_major_axis: f32,
    pub arg_periapsis: f32,
    pub true_anomaly: f32,
    pub retrograde: bool,
    pub body: Body,
}

impl Orbit {
    pub fn is_nan(&self) -> bool {
        self.eccentricity.is_nan()
            || self.semi_major_axis.is_nan()
            || self.arg_periapsis.is_nan()
            || self.true_anomaly.is_nan()
    }

    pub fn from_pv(r: impl Into<Vec2>, v: impl Into<Vec2>, body: Body) -> Self {
        let r3 = r.into().extend(0.0);
        let v3 = v.into().extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / body.mu() - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (body.mu() * (1.0 - e.length_squared()));
        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = 2.0 * std::f32::consts::PI - true_anomaly;
        }

        Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            true_anomaly,
            retrograde: h.z < 0.0,
            body,
        }
    }

    pub fn circular(radius: f32, ta: f32, body: Body) -> Self {
        Orbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            true_anomaly: ta,
            retrograde: false,
            body,
        }
    }

    pub fn prograde(&self) -> Vec2 {
        self.prograde_at(self.true_anomaly)
    }

    pub fn prograde_at(&self, true_anomaly: f32) -> Vec2 {
        let fpa = self.flight_path_angle_at(true_anomaly);
        Vec2::from_angle(fpa).rotate(self.tangent_at(true_anomaly))
    }

    pub fn flight_path_angle(&self) -> f32 {
        self.flight_path_angle_at(self.true_anomaly)
    }

    pub fn flight_path_angle_at(&self, true_anomaly: f32) -> f32 {
        -(self.eccentricity * true_anomaly.sin())
            .atan2(1.0 + self.eccentricity * true_anomaly.cos())
    }

    pub fn tangent(&self) -> Vec2 {
        self.tangent_at(self.true_anomaly)
    }

    pub fn tangent_at(&self, true_anomaly: f32) -> Vec2 {
        let n = self.normal_at(true_anomaly);
        let angle = match self.retrograde {
            true => -std::f32::consts::PI / 2.0,
            false => std::f32::consts::PI / 2.0,
        };
        Vec2::from_angle(angle).rotate(n)
    }

    pub fn normal(&self) -> Vec2 {
        self.normal_at(self.true_anomaly)
    }

    pub fn normal_at(&self, true_anomaly: f32) -> Vec2 {
        self.position_at(true_anomaly).normalize()
    }

    pub fn semi_latus_rectum(&self) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
    }

    pub fn angular_momentum(&self) -> f32 {
        (self.body.mu() * self.semi_latus_rectum()).sqrt()
    }

    pub fn radius_at(&self, true_anomaly: f32) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f32::cos(true_anomaly))
    }

    pub fn period(&self) -> Duration {
        let t =
            2.0 * std::f32::consts::PI * (self.semi_major_axis.powi(3) / (self.body.mu())).sqrt();
        Duration::from_secs_f32(t)
    }

    pub fn pos(&self) -> Vec2 {
        self.position_at(self.true_anomaly)
    }

    pub fn vel(&self) -> Vec2 {
        self.velocity_at(self.true_anomaly)
    }

    pub fn position_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        let angle = match self.retrograde {
            false => true_anomaly,
            true => -true_anomaly,
        };
        Vec2::from_angle(angle + self.arg_periapsis) * r
    }

    pub fn velocity_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        let v = (self.body.mu() * (2.0 / r - 1.0 / self.semi_major_axis)).sqrt();
        let h = self.angular_momentum();
        let cosfpa = h / (r * v);
        let sinfpa = cosfpa * self.eccentricity * true_anomaly.sin()
            / (1.0 + self.eccentricity * true_anomaly.cos());
        let n = self.normal_at(true_anomaly);
        let t = self.tangent_at(true_anomaly);
        v * (t * cosfpa + n * sinfpa)
    }

    pub fn periapsis(&self) -> Vec2 {
        self.position_at(0.0)
    }

    pub fn apoapsis(&self) -> Vec2 {
        self.position_at(std::f32::consts::PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.body.mu() / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn mean_anomaly(&self) -> f32 {
        anomaly_t2m(self.eccentricity, self.true_anomaly)
    }
}

pub fn gravity_accel(body: Body, body_center: Vec2, sample: Vec2) -> Vec2 {
    let r: Vec2 = body_center - sample;
    let rsq = r.length_squared().clamp(body.radius.powi(2), std::f32::MAX);
    let a = GRAVITATIONAL_CONSTANT * body.mass / rsq;
    a * r.normalize()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    pub prop: Propagator,
    pub body: Option<Body>,
}

#[derive(Debug, Clone)]
pub struct OrbitalSystem {
    pub iter: usize,
    pub epoch: Duration,
    pub objects: Vec<Object>,
    next_id: i64,
    pub stepsize: Duration,
    pub units: CanonicalUnits,
}

impl Default for OrbitalSystem {
    fn default() -> Self {
        OrbitalSystem {
            iter: 0,
            epoch: Duration::default(),
            objects: Vec::default(),
            next_id: 0,
            stepsize: Duration::from_millis(100),
            units: earth_moon_canonical_units(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PV {
    pub pos: Vec2,
    pub vel: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub enum OrbitalEvent {
    LookupFailure(ObjectId),
    NumericalError(ObjectId),
    Collision(Vec2, ObjectId, Option<ObjectId>),
    Escaped(Vec2, ObjectId),
}

impl OrbitalSystem {
    pub fn add_object(&mut self, prop: impl Into<Propagator>, body: Option<Body>) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.objects.push(Object {
            id,
            prop: prop.into(),
            body,
        });
        id
    }

    pub fn has_object(&self, id: ObjectId) -> bool {
        self.objects.iter().find(|o| o.id == id).is_some()
    }

    pub fn min_id(&self) -> Option<ObjectId> {
        self.objects.iter().map(|o| o.id).min()
    }

    pub fn max_id(&self) -> Option<ObjectId> {
        self.objects.iter().map(|o| o.id).max()
    }

    pub fn lookup(&self, o: ObjectId) -> Option<Object> {
        self.objects.iter().find(|m| m.id == o).map(|m| m.clone())
    }

    pub fn lookup_ref(&self, o: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|m| m.id == o)
    }

    pub fn lookup_mut(&mut self, o: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|m| m.id == o)
    }

    pub fn transform_from_id(&self, id: Option<ObjectId>) -> Option<PV> {
        if let Some(i) = id {
            let obj = self.lookup(i)?;
            self.global_transform(&obj.prop)
        } else {
            Some(PV::default())
        }
    }

    pub fn global_transform(&self, prop: &impl Propagate) -> Option<PV> {
        if let Some(rel) = prop.relative_to() {
            let obj = self.lookup(rel)?;
            let rel = self.global_transform(&obj.prop)?;
            Some(PV {
                pos: prop.pos() + rel.pos,
                vel: prop.vel() + rel.vel,
            })
        } else {
            Some(PV {
                pos: prop.pos(),
                vel: prop.vel(),
            })
        }
    }

    pub fn step(&mut self) -> Vec<(Object, OrbitalEvent)> {
        self.iter += 1;
        self.propagate_to(self.epoch + self.stepsize)
    }

    fn propagate_to(&mut self, epoch: Duration) -> Vec<(Object, OrbitalEvent)> {
        let copy = self.clone();
        for m in self.objects.iter_mut() {
            m.prop.propagate(epoch - self.epoch, &copy);
        }

        self.epoch = epoch;

        self.reparent_patched_conics();

        let bodies = self.bodies();

        let remove_with_reason = |o: &Object| -> Option<OrbitalEvent> {
            if let Propagator::Kepler(k) = o.prop {
                if k.orbit.is_nan() {
                    return Some(OrbitalEvent::NumericalError(o.id));
                }
            }

            let gp = match self.global_transform(&o.prop) {
                Some(p) => p,
                None => return Some(OrbitalEvent::LookupFailure(o.id)),
            };

            if gp.pos.length_squared() > 20000.0 * 20000.0 {
                return Some(OrbitalEvent::Escaped(gp.pos, o.id));
            }

            let mut collided = bodies.iter().filter(|b| {
                let d = b.1.distance_squared(gp.pos);
                d != 0.0 && d < b.2.radius.powi(2)
            });

            if let Some(c) = collided.next() {
                let delta: Vec2 = gp.pos - c.1;
                return Some(OrbitalEvent::Collision(delta, o.id, Some(c.0)));
            }

            None
        };

        let to_remove = self
            .objects
            .iter()
            .filter_map(|o| match remove_with_reason(o) {
                Some(r) => Some((o.clone(), r)),
                None => None,
            })
            .collect::<Vec<_>>();

        let ids_to_remove = to_remove.iter().map(|(o, _)| o.id).collect::<Vec<_>>();

        self.objects.retain(|o| !ids_to_remove.contains(&o.id));

        to_remove
    }

    pub fn bodies(&self) -> Vec<(ObjectId, Vec2, Body)> {
        self.objects
            .iter()
            .filter_map(|o| Some((o.id, self.global_transform(&o.prop)?.pos, o.body?)))
            .collect()
    }

    pub fn gravity_at(&self, pos: Vec2) -> Vec2 {
        self.bodies()
            .iter()
            .map(|(_, c, b)| gravity_accel(*b, *c, pos))
            .sum()
    }

    pub fn potential_at(&self, pos: Vec2) -> f32 {
        self.bodies()
            .iter()
            .map(|(_, c, b)| {
                let r = (c - pos).length();
                if r < b.radius {
                    return 0.0;
                }
                -b.mu() / r
            })
            .sum()
    }

    pub fn primary_body_at(&self, pos: Vec2, exclude: Option<ObjectId>) -> Option<Object> {
        let mut ret = self
            .objects
            .iter()
            .filter_map(|o| {
                if Some(o.id) == exclude {
                    return None;
                }
                let soi = o.body?.soi;
                let bpos = self.global_transform(&o.prop)?;
                let d = bpos.pos.distance(pos);
                if d > soi {
                    return None;
                }
                Some((o.clone(), soi))
            })
            .collect::<Vec<_>>();

        ret.sort_by(|(_, l), (_, r)| l.partial_cmp(r).unwrap());
        ret.first().map(|(o, _)| o.clone())
    }

    pub fn barycenter(&self) -> Vec2 {
        let bodies = self.bodies();
        let total_mass: f32 = bodies.iter().map(|(_, _, b)| b.mass).sum();
        bodies.iter().map(|(_, p, b)| p * b.mass).sum::<Vec2>() / total_mass
    }

    pub fn reparent_patched_conics(&mut self) {
        let new_kepler: Vec<_> = self.objects.iter().filter_map(|obj| {
            match &obj.prop {
                Propagator::Kepler(k) => {
                    let child_pv = self.global_transform(&obj.prop)?;
                    let primary = self.primary_body_at(child_pv.pos, Some(obj.id))?;
                    if primary.id == k.primary {
                        return None;
                    }
                    let primary_pv = self.global_transform(&primary.prop)?;
                    // TODO math operators for PV?
                    let ds = child_pv.pos - primary_pv.pos;
                    let dv = child_pv.vel - primary_pv.vel;
                    let orbit = Orbit::from_pv(ds, dv, primary.body?);
                    let mut new_prop = *k;
                    new_prop.orbit = orbit;
                    new_prop.primary = primary.id;
                    Some((obj.id, new_prop))
                }
                _ => None,
            }
        }).collect();

        for (id, prop) in new_kepler.iter() {
            if let Some(obj) = self.lookup_mut(*id) {
                obj.prop = (*prop).into();
            }
        }
    }
}

pub fn generate_square_lattice(center: Vec2, w: i32, step: usize) -> Vec<Vec2> {
    let mut ret = vec![];
    for x in (-w..w).step_by(step) {
        for y in (-w..w).step_by(step) {
            ret.push(center + Vec2::new(x as f32, y as f32));
        }
    }
    ret
}

pub fn generate_circular_log_lattice(center: Vec2, rmin: f32, rmax: f32) -> Vec<Vec2> {
    // this isn't actually log, but I'm lazy
    let mut ret = vec![];

    let mut r = rmin;
    let mut dr = 30.0;

    while r < rmax {
        let circ = 2.0 * std::f32::consts::PI * r;
        let mut pts = (circ / dr).ceil() as u32;
        while pts % 8 > 0 {
            pts += 1; // yeah this is stupid
        }
        for i in 0..pts {
            let a = 2.0 * std::f32::consts::PI * i as f32 / pts as f32;
            let x = a.cos();
            let y = a.sin();
            ret.push(center + Vec2::new(x, y) * r);
        }

        r += dr;
        dr *= 1.1;
    }

    ret
}
