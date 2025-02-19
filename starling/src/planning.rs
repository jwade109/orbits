use crate::core::*;
use crate::orbiter::*;
use crate::orbits::sparse_orbit::{OrbitClass, SparseOrbit, PI};
use crate::orbits::universal::tspace;
use crate::pv::PV;
use glam::f32::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum ConvergeError<T> {
    Initial((T, bool), (T, bool)),
    Final((T, bool), (T, bool)),
    MaxIter,
}

pub trait BinarySearchKey
where
    Self: Copy + std::ops::Sub<Output = Self> + std::ops::Add<Output = Self>,
    Self: std::cmp::PartialOrd,
    Self: std::ops::Mul<f32, Output = Self>,
{
}

impl<T> BinarySearchKey for T
where
    T: Copy + std::ops::Sub<Output = T> + std::ops::Add<Output = T>,
    T: std::cmp::PartialOrd,
    T: std::ops::Mul<f32, Output = T>,
{
}

// determines timestamp where condition goes from true to false
pub fn binary_search_recurse<T: BinarySearchKey>(
    start: (T, bool),
    end: (T, bool),
    tol: T,
    cond: impl Fn(T) -> bool,
    remaining: usize,
) -> Result<T, ConvergeError<T>> {
    if remaining == 0 {
        return Err(ConvergeError::MaxIter);
    }

    if end.0 - start.0 < tol {
        return Ok(end.0);
    }

    let midpoint = start.0 + (end.0 - start.0) * 0.5;
    let a = start.1;
    let b = cond(midpoint);
    let c = end.1;

    if !a {
        Err(ConvergeError::Initial(start, end))
    } else if a && !b {
        binary_search_recurse(start, (midpoint, b), tol, cond, remaining - 1)
    } else if b && !c {
        binary_search_recurse((midpoint, b), end, tol, cond, remaining - 1)
    } else {
        Err(ConvergeError::Final(start, end))
    }
}

pub fn binary_search<T: BinarySearchKey>(
    start: T,
    end: T,
    tol: T,
    max_iters: usize,
    cond: &impl Fn(T) -> bool,
) -> Result<T, ConvergeError<T>> {
    let a = cond(start);
    let c = cond(end);
    binary_search_recurse((start, a), (end, c), tol, cond, max_iters)
}

#[derive(Debug, Clone, Copy)]
pub enum PredictError<T: BinarySearchKey> {
    BadType,
    Lookup,
    BadTimeDelta,
    TooManyIterations,
    Collision(ConvergeError<T>),
    Escape(ConvergeError<T>),
    Encounter(ConvergeError<T>),
}

fn mutual_separation(o1: &SparseOrbit, o2: &SparseOrbit, t: Nanotime) -> f32 {
    let p1 = o1.pv_at_time(t).pos;
    let p2 = o2.pv_at_time(t).pos;
    p1.distance(p2)
}

pub fn search_condition<T: BinarySearchKey>(
    t1: T,
    t2: T,
    tol: T,
    cond: &impl Fn(T) -> bool,
) -> Result<Option<T>, ConvergeError<T>> {
    let a = cond(t1);
    if !a {
        return Ok(None);
    }
    let b = cond(t2);
    if b {
        return Ok(None);
    }
    binary_search::<T>(t1, t2, tol, 100, cond).map(|t| Some(t))
}

#[derive(Debug, Clone)]
pub struct Propagator {
    pub parent: ObjectId,
    pub orbit: SparseOrbit,
    pub start: Nanotime,
    pub end: Nanotime,
    pub dt: Nanotime,
    pub finished: bool,
    pub event: Option<EventType>,
}

impl Propagator {
    pub fn new(parent: ObjectId, orbit: SparseOrbit, stamp: Nanotime) -> Self {
        Propagator {
            parent,
            orbit,
            start: stamp,
            end: stamp,
            dt: Nanotime(0),
            finished: false,
            event: None,
        }
    }

    pub fn is_active(&self, stamp: Nanotime) -> bool {
        self.start <= stamp && stamp <= self.end
    }

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        self.is_active(stamp).then(|| self.orbit.pv_at_time(stamp))
    }

    pub fn stamp(&self) -> Nanotime {
        self.end
    }

    pub fn calculated_to(&self, stamp: Nanotime) -> bool {
        return self.finished || self.end >= stamp;
    }

    pub fn reset(&mut self, stamp: Nanotime) {
        self.finished = false;
        self.end = stamp;
    }

    pub fn freeze(&mut self, stamp: Nanotime) {
        self.finished = true;
        self.end = stamp;
    }

    pub fn next_prop(
        &self,
        planets: &PlanetarySystem,
    ) -> Result<Option<Propagator>, BadObjectNextState> {
        let e = match self.event {
            Some(e) => e,
            None => return Ok(None),
        };

        match e {
            EventType::Collide(_) => Ok(None),
            EventType::NumericalError => Ok(None),
            EventType::Escape(_) => {
                let cur = planets
                    .lookup(self.parent, self.end)
                    .ok_or(BadObjectNextState::Lookup)?;
                let reparent = match cur.2 {
                    Some(id) => id,
                    None => return Ok(None),
                };
                let new = planets
                    .lookup(reparent, self.end)
                    .ok_or(BadObjectNextState::Lookup)?;

                let pv = self.orbit.pv_at_time(self.end);
                let dv = cur.1 - new.1;
                let orbit = SparseOrbit::from_pv(pv + dv, new.0, self.end)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(reparent, orbit, self.end)))
            }
            EventType::Encounter(id) => {
                let cur = planets
                    .lookup(self.parent, self.end)
                    .ok_or(BadObjectNextState::Lookup)?;
                let new = planets
                    .lookup(id, self.end)
                    .ok_or(BadObjectNextState::Lookup)?;

                let pv = self.orbit.pv_at_time(self.end);
                let dv = cur.1 - new.1;
                let orbit = SparseOrbit::from_pv(pv + dv, new.0, self.end)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(id, orbit, self.end)))
            }
            EventType::Maneuver(man) => {
                let pv = self.orbit.pv_at_time(self.end);
                let dv = match man {
                    Maneuver::AxisAligned(dv) => dv,
                };
                let orbit = SparseOrbit::from_pv(pv + PV::vel(dv), self.orbit.body, self.end)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(self.parent, orbit, self.end)))
            }
        }
    }

    pub fn next(
        &mut self,
        bodies: &[(ObjectId, &SparseOrbit, f32)],
    ) -> Result<(), PredictError<Nanotime>> {
        if self.finished {
            return Ok(());
        }

        let tol = Nanotime(5);

        let alt = self.orbit.pv_at_time(self.end).pos.length();

        let might_hit_planet = self.orbit.is_suborbital();
        let can_escape =
            self.orbit.eccentricity >= 1.0 || self.orbit.apoapsis_r() >= self.orbit.body.soi;
        let near_body = bodies
            .iter()
            .any(|(_, orb, soi)| mutual_separation(&self.orbit, orb, self.stamp()) < soi * 3.0);

        let p_self = self.orbit.periapsis_r();
        let a_self = self.orbit.apoapsis_r();
        let will_never_hit_anything = !might_hit_planet
            && bodies.iter().all(|(_, orbit, soi)| {
                let p_other = orbit.periapsis_r();
                let a_other = orbit.apoapsis_r();

                p_self > a_other + soi || a_self < p_other - soi
            });

        self.dt = if will_never_hit_anything {
            Nanotime::secs(500)
        } else if might_hit_planet {
            Nanotime::millis(20)
        } else if can_escape {
            Nanotime::secs(2)
        } else if near_body {
            Nanotime::millis(500)
        } else {
            Nanotime::secs(5)
        };

        let t1 = self.end;
        let t2 = self.end + self.dt;

        self.end = t2;

        match self
            .orbit
            .pv_at_time_fallible(t1)
            .zip(self.orbit.pv_at_time_fallible(t2))
        {
            None => {
                self.end = t1;
                self.finished = true;
                self.event = Some(EventType::NumericalError);
                return Ok(());
            }
            _ => (),
        };

        let above_planet = |t: Nanotime| {
            let pos = self.orbit.pv_at_time(t).pos;
            pos.length() > self.orbit.body.radius
        };

        let escape_soi = |t: Nanotime| {
            let pos = self.orbit.pv_at_time(t).pos;
            pos.length() < self.orbit.body.soi
        };

        if might_hit_planet {
            if !above_planet(t1) {
                self.end = t1;
                self.finished = true;
                self.event = Some(EventType::Collide(self.parent));
                return Ok(());
            }

            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &above_planet)
                .map_err(|e| PredictError::Collision(e))?
            {
                if t - self.start < Nanotime::millis(10) {
                    self.end = t2;
                    return Ok(());
                }
                self.end = t;
                self.finished = true;
                self.event = Some(EventType::Collide(self.parent));
                return Ok(());
            }
        }

        if can_escape {
            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &escape_soi)
                .map_err(|e| PredictError::Escape(e))?
            {
                if t - self.start < Nanotime::millis(10) {
                    self.end = t2;
                    return Ok(());
                }
                self.end = t;
                self.finished = true;
                self.event = Some(EventType::Escape(self.parent));
                return Ok(());
            }
        }

        if near_body {
            for i in 0..bodies.len() {
                let (_, orbit, soi) = bodies[i];
                let cond = separation_with(&self.orbit, orbit, soi);
                let id = bodies[i].0;
                if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &cond)
                    .map_err(|e| PredictError::Encounter(e))?
                {
                    if t - self.start < Nanotime::millis(10) {
                        self.end = t2;
                        return Ok(());
                    }
                    self.end = t;
                    self.finished = true;
                    self.event = Some(EventType::Encounter(id));
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}

pub fn separation_with<'a>(
    ego: &'a SparseOrbit,
    planet: &'a SparseOrbit,
    soi: f32,
) -> impl Fn(Nanotime) -> bool + use<'a> {
    move |t: Nanotime| mutual_separation(ego, planet, t) > soi
}

pub fn get_next_intersection(
    stamp: Nanotime,
    eval: &SparseOrbit,
    target: &SparseOrbit,
) -> Result<Option<(Nanotime, PV)>, ConvergeError<Nanotime>> {
    let n = 100;
    let period = eval.period_or(Nanotime::secs(500));
    let teval = tspace(stamp, stamp + period, n);

    let signed_distance_at = |t: Nanotime| {
        let pcurr = eval.pv_at_time(t);
        target.nearest_along_track(pcurr.pos)
    };

    let initial_sign = signed_distance_at(stamp).1 > 0.0;

    let condition = |t: Nanotime| {
        let (_, d) = signed_distance_at(t);
        let cur = d > 0.0;
        cur == initial_sign
    };

    for ts in teval.windows(2) {
        let t1 = ts[0];
        let t2 = ts[1];
        let t = search_condition(t1, t2, Nanotime(500), &condition)?;
        if let Some(t) = t {
            let (pv, _) = signed_distance_at(t);
            return Ok(Some((t, pv)));
        }
    }

    return Ok(None);
}

#[derive(Debug, Clone)]
pub struct ManeuverPlan {
    pub nodes: Vec<ManeuverNode>,
}

impl std::fmt::Display for ManeuverPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Maneuver Plan\n")?;
        if self.nodes.is_empty() {
            return write!(f, " (empty)");
        }
        for (i, node) in self.nodes.iter().enumerate() {
            let endline = if i + 1 < self.nodes.len() { "\n" } else { "" };
            write!(
                f,
                "{}. {:?} dV {:0.1} ({:0.1}) to {:?} orbit{}",
                i + 1,
                node.stamp,
                node.dv(),
                node.dv().length(),
                node.orbit.class(),
                endline
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ManeuverNode {
    pub stamp: Nanotime,
    pub before: PV,
    pub after: PV,
    pub orbit: SparseOrbit,
}

impl ManeuverNode {
    pub fn dv(&self) -> Vec2 {
        self.after.vel - self.before.vel
    }
}

pub fn generate_maneuver_plan(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    if let Some(n) = get_next_intersection(now, current, destination)
        .ok()?
        .map(|(t, pvf)| {
            let pvi = current.pv_at_time(t);
            Some(ManeuverNode {
                stamp: t,
                before: pvi,
                after: pvf,
                orbit: SparseOrbit::from_pv(pvf, current.body, t)?,
            })
        })
        .flatten()
    {
        return Some(ManeuverPlan { nodes: vec![n] });
    }

    match current.class() {
        OrbitClass::Parabolic | OrbitClass::Hyperbolic | OrbitClass::VeryThin => return None,
        _ => (),
    }

    if current.retrograde != destination.retrograde {
        return None;
    }

    let r1 = current.periapsis_r();
    let r2 = destination.radius_at_angle(current.arg_periapsis + PI);
    let a_transfer = (r1 + r2) / 2.0;
    let mu = current.body.mu();
    let v1 = (mu * (2.0 / r1 - 1.0 / a_transfer)).sqrt();

    let t1 = current.t_next_p(now)?;
    let before = current.pv_at_time(t1);
    let prograde = before.vel.normalize_or_zero();
    let after = PV::new(before.pos, prograde * v1);

    let transfer_orbit = SparseOrbit::from_pv(after, current.body, t1)?;

    let n1 = ManeuverNode {
        stamp: t1,
        before,
        after,
        orbit: transfer_orbit,
    };

    let t2 = t1 + transfer_orbit.period()? / 2;
    let before = transfer_orbit.pv_at_time(t2);
    let (after, _) = destination.nearest(before.pos);
    let after = PV::new(before.pos, after.vel);

    let final_orbit = SparseOrbit::from_pv(after, current.body, t2)?;

    let n2 = ManeuverNode {
        stamp: t2,
        before,
        after,
        orbit: final_orbit,
    };

    Some(ManeuverPlan {
        nodes: vec![n1, n2],
    })
}
