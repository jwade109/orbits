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
    pub fn from_pv(r: Vec2, v: Vec2, body: Body) -> Self {
        let r3 = r.extend(0.0);
        let v3 = v.extend(0.0);
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

    pub fn from_apses(per: Vec2, apo: Vec2, ta: f32) -> Self {
        todo!()
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
        (self.body.mu() / self.semi_major_axis.powi(3)).sqrt()
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

pub const EARTH: (Body, Propagator) = (
    Body {
        radius: 63.0,
        mass: 1000.0,
        soi: 15000.0,
    },
    Propagator::Fixed(Vec2::ZERO, None),
);

pub const LUNA: (Body, Propagator) = (
    Body {
        radius: 22.0,
        mass: 10.0,
        soi: 800.0,
    },
    Propagator::NBody(NBodyPropagator {
        epoch: Duration::new(0, 0),
        pos: Vec2::new(-3800.0, 0.0),
        vel: Vec2::new(0.0, -58.0),
    }),
);

#[derive(Debug, Copy, Clone)]
pub struct NBodyPropagator {
    pub epoch: Duration,
    pub pos: Vec2,
    pub vel: Vec2,
}

impl NBodyPropagator {
    pub fn propagate_to(&mut self, bodies: &[(Vec2, Body)], epoch: Duration) {
        let delta_time = epoch - self.epoch;
        let dt = delta_time.as_secs_f32();

        let steps = delta_time.as_millis() / 10;

        let others = bodies
            .iter()
            .filter(|(c, _)| *c != self.pos)
            .collect::<Vec<_>>();

        (0..steps).for_each(|_| {
            let a: Vec2 = others
                .iter()
                .map(|(c, b)| -> Vec2 { gravity_accel(*b, *c, self.pos) })
                .sum();

            self.vel += a * dt / steps as f32;
            self.pos += self.vel * dt / steps as f32;
        });

        self.epoch = epoch;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeplerPropagator {
    pub epoch: Duration,
    pub primary: ObjectId,
    pub orbit: Orbit,
}

impl KeplerPropagator {
    pub fn from_pv(epoch: Duration, pos: Vec2, vel: Vec2, body: Body, parent: ObjectId) -> Self {
        let orbit = Orbit::from_pv(pos, vel, body);
        KeplerPropagator {
            epoch,
            primary: parent,
            orbit,
        }
    }

    pub fn propagate_to(&mut self, epoch: Duration) {
        let delta = epoch - self.epoch;

        if delta == Duration::default() {
            return;
        }

        let n = self.orbit.mean_motion();
        let m = self.orbit.mean_anomaly();
        let m2 = m + delta.as_secs_f32() * n;
        self.orbit.true_anomaly = anomaly_m2t(self.orbit.eccentricity, m2).unwrap();
        self.epoch = epoch;
    }
}

pub trait Propagate {
    fn epoch(&self) -> Duration;

    fn pos(&self) -> Vec2;

    fn vel(&self) -> Vec2;

    fn relative_to(&self) -> Option<ObjectId>;

    fn propagate_to(&mut self, epoch: Duration, state: &OrbitalSystem);
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2, Option<ObjectId>),
    NBody(NBodyPropagator),
    Kepler(KeplerPropagator),
}

impl Propagator {
    pub fn fixed_at(pos: Vec2) -> Self {
        Propagator::Fixed(pos, None)
    }

    pub fn kepler(epoch: Duration, orbit: Orbit, primary: ObjectId) -> Self {
        Propagator::Kepler(KeplerPropagator {
            epoch,
            primary,
            orbit,
        })
    }

    pub fn nbody(epoch: Duration, pos: Vec2, vel: Vec2) -> Self {
        Propagator::NBody(NBodyPropagator { epoch, pos, vel })
    }
}

impl Propagate for Propagator {
    fn propagate_to(&mut self, epoch: Duration, state: &OrbitalSystem) {
        let bodies = state.bodies();
        match self {
            Propagator::NBody(nb) => nb.propagate_to(&bodies, epoch),
            Propagator::Kepler(k) => k.propagate_to(epoch),
            Propagator::Fixed(_, _) => (),
        };
    }

    fn relative_to(&self) -> Option<ObjectId> {
        match self {
            Propagator::NBody(_) => None,
            Propagator::Kepler(k) => Some(k.primary),
            Propagator::Fixed(_, o) => *o,
        }
    }

    fn epoch(&self) -> Duration {
        match self {
            Propagator::NBody(nb) => nb.epoch,
            Propagator::Kepler(k) => k.epoch,
            Propagator::Fixed(_, _) => Duration::default(),
        }
    }

    fn pos(&self) -> Vec2 {
        match self {
            Propagator::NBody(nb) => nb.pos,
            Propagator::Kepler(k) => k.orbit.pos(),
            Propagator::Fixed(p, _) => *p,
        }
    }

    fn vel(&self) -> Vec2 {
        match self {
            Propagator::NBody(nb) => nb.vel,
            Propagator::Kepler(k) => k.orbit.vel(),
            Propagator::Fixed(_, _) => Vec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjectId(pub i64);

#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub id: ObjectId,
    pub prop: Propagator,
    pub body: Option<Body>,
}

#[derive(Debug, Clone, Default)]
pub struct OrbitalSystem {
    pub epoch: Duration,
    pub objects: Vec<Object>,
    next_id: i64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PV {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl OrbitalSystem {
    pub fn add_object(&mut self, prop: Propagator, body: Option<Body>) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.objects.push(Object { id, prop, body });
        id
    }

    pub fn lookup(&self, o: ObjectId) -> Option<Object> {
        self.objects.iter().find(|m| m.id == o).map(|m| *m)
    }

    pub fn global_transform(&self, prop: impl Propagate) -> Option<PV> {
        if let Some(rel) = prop.relative_to() {
            let obj = self.lookup(rel)?;
            let rel = self.global_transform(obj.prop)?;
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

    pub fn global_pos(&self, prop: impl Propagate) -> Option<Vec2> {
        if let Some(rel) = prop.relative_to() {
            let obj = self.lookup(rel)?;
            Some(prop.pos() + self.global_pos(obj.prop)?)
        } else {
            Some(prop.pos())
        }
    }

    pub fn global_vel(&self, prop: impl Propagate) -> Option<Vec2> {
        if let Some(rel) = prop.relative_to() {
            let obj = self.lookup(rel)?;
            Some(prop.vel() + self.global_vel(obj.prop)?)
        } else {
            Some(prop.vel())
        }
    }

    pub fn propagate_to(&mut self, epoch: Duration) {
        let copy = self.clone();
        for m in self.objects.iter_mut() {
            m.prop.propagate_to(epoch, &copy);
        }
    }

    fn bodies(&self) -> Vec<(Vec2, Body)> {
        self.objects
            .iter()
            .filter(|o| o.body.is_some())
            .map(|o| (o.prop.pos(), o.body.unwrap()))
            .collect()
    }

    pub fn gravity_at(&self, pos: Vec2) -> Vec2 {
        self.bodies()
            .iter()
            .map(|(c, b)| gravity_accel(*b, *c, pos))
            .sum()
    }

    pub fn potential_at(&self, pos: Vec2) -> f32 {
        self.bodies()
            .iter()
            .map(|(c, b)| {
                let r = (c - pos).length();
                if r < b.radius {
                    return 0.0;
                }
                -b.mu() / r
            })
            .sum()
    }

    pub fn primary_body_at(&self, pos: Vec2, exclude: Option<ObjectId>) -> Option<Object> {
        let mut ret = None;
        let mut max_grav = f32::MIN;
        for obj in self.objects.iter() {
            if Some(obj.id) == exclude
            {
                continue;
            }
            if let (Some(body), Some(c)) = (obj.body, self.global_pos(obj.prop)) {
                let g = gravity_accel(body, c, pos).length_squared();
                if max_grav < g {
                    max_grav = g;
                    ret = Some(*obj);
                }
            }
        }
        ret
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
