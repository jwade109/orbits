use crate::scenario::*;
use crate::math::{tspace, PI};
use crate::nanotime::Nanotime;
use crate::orbiter::*;
use crate::orbits::{OrbitClass, SparseOrbit};
use crate::pv::PV;
use glam::f32::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Collide(ObjectId),
    Escape(ObjectId),
    Encounter(ObjectId),
    Impulse(Vec2),
    NumericalError,
}

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
pub(crate) fn binary_search_recurse<T: BinarySearchKey>(
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

pub(crate) fn binary_search<T: BinarySearchKey>(
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
    Lookup,
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

pub(crate) fn search_condition<T: BinarySearchKey>(
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
pub(crate) enum BadObjectNextState {
    Lookup,
    BadOrbit,
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

impl std::fmt::Display for Propagator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}, {:?}, {}, {:?}, {:?}, {:?}",
            self.start,
            self.end,
            self.finished,
            self.event,
            self.dt,
            self.orbit.class(),
        )
    }
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

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        self.is_active(stamp).then(|| self.orbit.pv_at_time(stamp))
    }

    pub(crate) fn is_active(&self, stamp: Nanotime) -> bool {
        self.start <= stamp && stamp <= self.end
    }

    pub(crate) fn calculated_to(&self, stamp: Nanotime) -> bool {
        return self.finished || self.end >= stamp;
    }

    pub(crate) fn is_err(&self) -> bool {
        match self.event {
            Some(EventType::NumericalError) => true,
            _ => false,
        }
    }

    pub(crate) fn next_prop(
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
            EventType::Impulse(dv) => {
                let pv = self.orbit.pv_at_time(self.end);
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

        if !self.orbit.is_suborbital() && !self.orbit.will_escape() && bodies.is_empty() {
            // nothing will ever happen to this orbit
            self.end += Nanotime::secs(500);
            return Ok(());
        }

        let tol = Nanotime(5);

        let alt = self.orbit.pv_at_time(self.end).pos.length();

        let might_hit_planet = self.orbit.is_suborbital() && alt < self.orbit.body.radius * 20.0;
        let can_escape = self.orbit.will_escape();
        let near_body = bodies
            .iter()
            .any(|(_, orb, soi)| mutual_separation(&self.orbit, orb, self.end) < soi * 3.0);

        self.dt = if might_hit_planet {
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
            .ok()
            .zip(self.orbit.pv_at_time_fallible(t2).ok())
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

                if t1 != self.start && !cond(t1) {
                    self.end = t1;
                    self.finished = true;
                    self.event = Some(EventType::Encounter(id));
                    return Ok(());
                }

                if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &cond)
                    .map_err(|e| PredictError::Encounter(e))?
                {
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

pub(crate) fn separation_with<'a>(
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
        let t = search_condition(t1, t2, Nanotime(10), &condition)?;
        if let Some(t) = t {
            let (pv, _) = signed_distance_at(t);
            return Ok(Some((t, pv)));
        }
    }

    return Ok(None);
}

#[derive(Debug, Clone)]
pub struct ManeuverPlan {
    pub kind: ManeuverType,
    pub nodes: Vec<ManeuverNode>,
}

impl ManeuverPlan {
    pub fn new(
        kind: ManeuverType,
        initial: &SparseOrbit,
        dvs: &[(Nanotime, Vec2)],
    ) -> Option<Self> {
        let mut current = *initial;
        let mut nodes = vec![];
        for (time, dv) in dvs {
            let before = current.pv_at_time_fallible(*time).ok()?;
            let after = before + PV::vel(*dv);
            let next = SparseOrbit::from_pv(after, initial.body, *time)?;
            let node = ManeuverNode {
                stamp: *time,
                impulse: PV::new(before.pos, after.vel - before.vel),
                orbit: next,
            };
            nodes.push(node);
            current = next;
        }
        Some(ManeuverPlan { kind, nodes })
    }

    pub fn dv(&self) -> f32 {
        self.nodes.iter().map(|n| n.impulse.vel.length()).sum()
    }
}

impl std::fmt::Display for ManeuverPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Maneuver Plan ({:?}) ({:0.1})\n", self.kind, self.dv())?;
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
                node.impulse.vel,
                node.impulse.vel.length(),
                node.orbit.class(),
                endline
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ManeuverType {
    Direct,
    Hohmann,
    Bielliptic,
}

#[derive(Debug, Clone, Copy)]
pub struct ManeuverNode {
    pub stamp: Nanotime,
    pub impulse: PV,
    pub orbit: SparseOrbit,
}

// https://en.wikipedia.org/wiki/Vis-viva_equation
fn vis_viva_equation(mu: f32, r: f32, a: f32) -> f32 {
    (mu * (2.0 / r - 1.0 / a)).sqrt()
}

fn hohmann_transfer(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    match current.class() {
        OrbitClass::Parabolic | OrbitClass::Hyperbolic | OrbitClass::VeryThin => return None,
        _ => (),
    }

    let mu = current.body.mu();

    if current.retrograde != destination.retrograde {
        return None;
    }

    let r1 = current.periapsis_r();
    let r2 = destination.radius_at_angle(current.arg_periapsis + PI);
    let a_transfer = (r1 + r2) / 2.0;
    let v1 = vis_viva_equation(mu, r1, a_transfer);

    let t1 = current.t_next_p(now)?;
    let before = current.pv_at_time_fallible(t1).ok()?;
    let prograde = before.vel.normalize_or_zero();
    let after = PV::new(before.pos, prograde * v1);

    let dv1 = after.vel - before.vel;

    let transfer_orbit = SparseOrbit::from_pv(after, current.body, t1)?;

    let t2 = t1 + transfer_orbit.period()? / 2;
    let before = transfer_orbit.pv_at_time_fallible(t2).ok()?;
    let (after, _) = destination.nearest(before.pos);
    let after = PV::new(before.pos, after.vel);

    let dv2 = after.vel - before.vel;

    ManeuverPlan::new(ManeuverType::Hohmann, current, &[(t1, dv1), (t2, dv2)])
}

fn bielliptic_transfer(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    match current.class() {
        OrbitClass::Parabolic | OrbitClass::Hyperbolic | OrbitClass::VeryThin => return None,
        _ => (),
    }

    let mu = current.body.mu();

    let t1 = current.t_next_p(now)?;

    let r1 = current.periapsis_r();
    let r2 = destination.apoapsis_r();

    let rb = current.apoapsis_r().max(destination.apoapsis_r());

    let a1 = (r1 + rb) / 2.0;
    let a2 = (r2 + rb) / 2.0;

    // first maneuver; transfer from initial orbit to transfer one
    let (dv1, transfer_one) = {
        let v1 = vis_viva_equation(mu, r1, a1);
        let cv = current.pv_at_time_fallible(t1).ok()?;
        let prograde = cv.vel.try_normalize()?;
        let dv = v1 * prograde - cv.vel;
        let pv = PV::new(cv.pos, v1 * prograde);
        let transfer = SparseOrbit::from_pv(pv, current.body, t1)?;
        (dv, transfer)
    };

    // second maneuver; change from T1 to T2
    let t2 = t1 + transfer_one.period()? / 2;
    let (dv2, _transfer_two) = {
        let v2 = vis_viva_equation(mu, rb, a2);
        let cv = transfer_one.pv_at_time_fallible(t2).ok()?;
        let prograde = cv.vel.try_normalize()?;
        let dv = v2 * prograde - cv.vel;
        let pv = PV::new(cv.pos, v2 * prograde);
        let transfer = SparseOrbit::from_pv(pv, current.body, t1)?;
        (dv, transfer)
    };

    // let t3 = t2 + transfer_two.period()? / 2;

    // let dv3 = {
    //     let p_final = transfer_two.pv_at_time_fallible(t3).ok()?;
    //     let (pv_near, _) = destination.nearest(p_final.pos);
    //     let prograde = pv_near.vel.try_normalize()?;
    //     prograde * 20.0
    // };

    ManeuverPlan::new(ManeuverType::Bielliptic, current, &[(t1, dv1), (t2, dv2)])
}

pub fn generate_maneuver_plans(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Vec<ManeuverPlan> {
    let direct = get_next_intersection(now, current, destination)
        .ok()
        .flatten()
        .map(|(t, pvf)| {
            let pvi = current.pv_at_time(t);
            ManeuverPlan::new(ManeuverType::Direct, current, &[(t, pvf.vel - pvi.vel)])
        })
        .flatten();

    let hohmann = hohmann_transfer(current, destination, now);
    let bielliptic = bielliptic_transfer(current, destination, now);

    [direct, hohmann, bielliptic]
        .into_iter()
        .filter_map(|e| if let Some(e) = e { Some(e) } else { None })
        .collect()
}
