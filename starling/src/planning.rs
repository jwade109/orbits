use crate::math::{tspace, PI};
use crate::nanotime::Nanotime;
use crate::orbiter::*;
use crate::orbits::{vis_viva_equation, DenseOrbit, OrbitClass, SparseOrbit};
use crate::pv::PV;
use crate::scenario::*;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
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
    BadPosition,
    Collision(ConvergeError<T>),
    Escape(ConvergeError<T>),
    Encounter(ConvergeError<T>),
}

fn mutual_separation(o1: &SparseOrbit, o2: &SparseOrbit, t: Nanotime) -> f32 {
    let p1 = o1.pv(t).unwrap().pos;
    let p2 = o2.pv(t).unwrap().pos;
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
    BadPosition,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum HorizonState {
    Continuing(Nanotime),
    Indefinite,
    Transition(Nanotime, EventType),
    Terminating(Nanotime, EventType),
}

impl HorizonState {
    pub fn is_change(&self) -> bool {
        match self {
            HorizonState::Continuing(_) | HorizonState::Indefinite => false,
            HorizonState::Terminating(_, _) | HorizonState::Transition(_, _) => true,
        }
    }

    pub fn end(&self) -> Option<Nanotime> {
        match self {
            HorizonState::Indefinite => None,
            HorizonState::Continuing(t)
            | HorizonState::Terminating(t, _)
            | HorizonState::Transition(t, _) => Some(*t),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Propagator {
    pub parent: ObjectId,
    pub orbit: SparseOrbit,
    pub start: Nanotime,
    pub dt: Nanotime,
    pub horizon: HorizonState,
}

impl std::fmt::Display for Propagator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}, {:?}, {:?}, {:?}",
            self.start,
            self.dt,
            self.horizon,
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
            dt: Nanotime::zero(),
            horizon: HorizonState::Continuing(stamp),
        }
    }

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        self.is_active(stamp)
            .then(|| self.orbit.pv(stamp).ok())
            .flatten()
    }

    pub(crate) fn is_active(&self, stamp: Nanotime) -> bool {
        self.start <= stamp && self.end().unwrap_or(stamp) >= stamp
    }

    pub(crate) fn calculated_to(&self, stamp: Nanotime) -> bool {
        match self.horizon {
            HorizonState::Terminating(_, _) => true,
            HorizonState::Indefinite => true,
            HorizonState::Transition(_, _) => true,
            HorizonState::Continuing(end) => end >= stamp,
        }
    }

    pub(crate) fn is_err(&self) -> bool {
        match self.horizon {
            HorizonState::Terminating(_, EventType::NumericalError) => true,
            _ => false,
        }
    }

    pub fn event(&self) -> Option<EventType> {
        match self.horizon {
            HorizonState::Continuing(_) | HorizonState::Indefinite => None,
            HorizonState::Terminating(_, e) | HorizonState::Transition(_, e) => Some(e),
        }
    }

    pub fn stamped_event(&self) -> Option<(Nanotime, EventType)> {
        match self.horizon {
            HorizonState::Continuing(_) | HorizonState::Indefinite => None,
            HorizonState::Terminating(t, e) | HorizonState::Transition(t, e) => Some((t, e)),
        }
    }

    pub fn end(&self) -> Option<Nanotime> {
        self.horizon.end()
    }

    pub fn finish_or_compute_until(
        &mut self,
        stamp: Nanotime,
        bodies: &[(ObjectId, &SparseOrbit, f32)],
    ) -> Result<(), PredictError<Nanotime>> {
        while !self.calculated_to(stamp) {
            let e = self.next(bodies);
            if e.is_err() {
                return e;
            }
        }
        return Ok(());
    }

    pub(crate) fn next_prop(
        &self,
        planets: &PlanetarySystem,
    ) -> Result<Option<Propagator>, BadObjectNextState> {
        let (stamp, e) = match self.horizon {
            HorizonState::Transition(stamp, e) => (stamp, e),
            _ => return Ok(None),
        };

        match e {
            EventType::Collide(_) => Ok(None),
            EventType::NumericalError => Ok(None),
            EventType::Escape(_) => {
                let cur = planets
                    .lookup(self.parent, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let reparent = match cur.2 {
                    Some(id) => id,
                    None => return Ok(None),
                };
                let new = planets
                    .lookup(reparent, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let pv = self
                    .orbit
                    .pv(stamp)
                    .map_err(|_| BadObjectNextState::BadPosition)?;
                let dv = cur.1 - new.1;
                let orbit = SparseOrbit::from_pv(pv + dv, new.0, stamp)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(reparent, orbit, stamp)))
            }
            EventType::Encounter(id) => {
                let cur = planets
                    .lookup(self.parent, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let new = planets
                    .lookup(id, stamp)
                    .ok_or(BadObjectNextState::Lookup)?;
                let pv = self
                    .orbit
                    .pv(stamp)
                    .map_err(|_| BadObjectNextState::BadPosition)?;
                let dv = cur.1 - new.1;
                let orbit = SparseOrbit::from_pv(pv + dv, new.0, stamp)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(id, orbit, stamp)))
            }
            EventType::Impulse(dv) => {
                let pv = self
                    .orbit
                    .pv(stamp)
                    .map_err(|_| BadObjectNextState::BadPosition)?;
                let orbit = SparseOrbit::from_pv(pv + PV::vel(dv), self.orbit.body, stamp)
                    .ok_or(BadObjectNextState::BadOrbit)?;
                Ok(Some(Propagator::new(self.parent, orbit, stamp)))
            }
        }
    }

    pub fn next(
        &mut self,
        bodies: &[(ObjectId, &SparseOrbit, f32)],
    ) -> Result<(), PredictError<Nanotime>> {
        let end = match self.horizon {
            HorizonState::Continuing(end) => end,
            _ => return Ok(()),
        };

        let will_never_encounter = bodies.iter().all(|(_, orbit, soi)| {
            let rmin = orbit.periapsis_r() - soi;
            let rmax = orbit.apoapsis_r() + soi;
            self.orbit.apoapsis_r() < rmin || self.orbit.periapsis_r() > rmax
        });

        if !self.orbit.is_suborbital()
            && !self.orbit.will_escape()
            && (will_never_encounter || bodies.is_empty())
        {
            // nothing will ever happen to this orbit
            self.horizon = HorizonState::Indefinite;
            return Ok(());
        }

        let tol = Nanotime::nanos(5);

        let alt = self
            .orbit
            .pv(end)
            .map_err(|_| PredictError::BadPosition)?
            .pos
            .length();

        let might_hit_planet = self.orbit.is_suborbital() && alt < self.orbit.body.radius * 20.0;
        let can_escape = self.orbit.will_escape();
        let near_body = bodies
            .iter()
            .any(|(_, orb, soi)| mutual_separation(&self.orbit, orb, end) < soi * 3.0);

        self.dt = if might_hit_planet {
            Nanotime::millis(20)
        } else if can_escape {
            Nanotime::secs(2)
        } else if near_body {
            Nanotime::millis(500)
        } else {
            Nanotime::secs(5)
        };

        let t1 = end;
        let t2 = end + self.dt;

        match self.orbit.pv(t1).ok().zip(self.orbit.pv(t2).ok()) {
            None => {
                self.horizon = HorizonState::Terminating(t1, EventType::NumericalError);
                return Ok(());
            }
            _ => (),
        };

        let above_planet = |t: Nanotime| {
            let pos = self.orbit.pv(t).unwrap_or(PV::inf()).pos;
            pos.length() > self.orbit.body.radius
        };

        let escape_soi = |t: Nanotime| {
            let pos = self.orbit.pv(t).unwrap_or(PV::inf()).pos;
            pos.length() < self.orbit.body.soi
        };

        if might_hit_planet {
            if !above_planet(t1) {
                self.horizon = HorizonState::Terminating(t1, EventType::Collide(self.parent));
                return Ok(());
            }

            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &above_planet)
                .map_err(|e| PredictError::Collision(e))?
            {
                if t - self.start < Nanotime::millis(10) {
                    self.horizon = HorizonState::Continuing(t2);
                    return Ok(());
                }
                self.horizon = HorizonState::Terminating(t, EventType::Collide(self.parent));
                return Ok(());
            }
        }

        if can_escape {
            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &escape_soi)
                .map_err(|e| PredictError::Escape(e))?
            {
                if t - self.start < Nanotime::millis(10) {
                    self.horizon = HorizonState::Continuing(t2);
                    return Ok(());
                }
                self.horizon = HorizonState::Transition(t, EventType::Escape(self.parent));
                return Ok(());
            }
        }

        if near_body {
            for i in 0..bodies.len() {
                let (_, orbit, soi) = bodies[i];
                let cond = separation_with(&self.orbit, orbit, soi);
                let id = bodies[i].0;

                if t1 != self.start && !cond(t1) {
                    self.horizon = HorizonState::Transition(t1, EventType::Encounter(id));
                    return Ok(());
                }

                if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, &cond)
                    .map_err(|e| PredictError::Encounter(e))?
                {
                    self.horizon = HorizonState::Transition(t, EventType::Encounter(id));
                    return Ok(());
                }
            }
        }

        self.horizon = HorizonState::Continuing(t2);
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
        let pcurr = eval.pv(t).unwrap_or(PV::inf());
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
        let t = search_condition(t1, t2, Nanotime::nanos(10), &condition)?;
        if let Some(t) = t {
            let (pv, _) = signed_distance_at(t);
            return Ok(Some((t, pv)));
        }
    }

    return Ok(None);
}

#[derive(Debug, Clone)]
pub struct ManeuverPlan {
    kind: ManeuverType,
    segments: Vec<ManeuverSegment>,
}

impl ManeuverPlan {
    pub fn new(
        now: Nanotime,
        kind: ManeuverType,
        initial: SparseOrbit,
        dvs: &[(Nanotime, Vec2)],
    ) -> Option<Self> {
        if dvs.is_empty() {
            return None;
        }

        let mut segments = vec![ManeuverSegment {
            range: TimeRange::before(dvs.first().unwrap().0),
            orbit: DenseOrbit::in_range(&initial, now, dvs.first().unwrap().0)?,
            impulse: None,
        }];

        for (i, (stamp, dv)) in dvs.iter().enumerate() {
            let end = dvs.get(i + 1).map(|(t, _)| *t);
            let next = segments.last()?.next(*dv, *stamp, end)?;
            segments.push(next);
        }

        Some(ManeuverPlan {
            kind,
            segments: segments,
        })
    }

    pub fn from_orbits(orbits: &[SparseOrbit], now: Nanotime) -> Vec<Self> {
        orbits
            .windows(2)
            .into_iter()
            .filter_map(|o| Some(best_maneuver_plan(&o[0], &o[1], now)?))
            .collect()
    }

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        for (range, _, orbit) in self.orbits() {
            if let Some(pv) = range
                .includes(stamp)
                .then(|| orbit.sparse().pv(stamp).ok())
                .flatten()
            {
                return Some(pv);
            }
        }
        None
    }

    pub fn initial(&self) -> &DenseOrbit {
        &self.segments.first().unwrap().orbit
    }

    pub fn kind(&self) -> ManeuverType {
        self.kind
    }

    pub fn start(&self) -> Nanotime {
        self.segments
            .iter()
            .filter_map(|e| Some(e.range.start()?))
            .next()
            .unwrap()
    }

    pub fn end(&self) -> Nanotime {
        self.segments
            .iter()
            .filter_map(|e| Some(e.range.end()?))
            .last()
            .unwrap()
    }

    pub fn dvs(&self) -> impl Iterator<Item = (Nanotime, Vec2)> + use<'_> {
        self.segments
            .iter()
            .filter_map(|m| Some((m.range.start()?, m.impulse?)))
    }

    pub fn future_dvs(&self, stamp: Nanotime) -> impl Iterator<Item = (Nanotime, Vec2)> + use<'_> {
        self.dvs().filter(move |(t, _)| *t > stamp)
    }

    pub fn dv(&self) -> f32 {
        self.segments
            .iter()
            .map(|n| n.impulse.unwrap_or(Vec2::ZERO).length())
            .sum()
    }

    pub fn orbits(&self) -> impl Iterator<Item = (TimeRange, Option<Vec2>, &DenseOrbit)> + use<'_> {
        self.segments.iter().map(|e| (e.range, e.impulse, &e.orbit))
    }

    pub fn and_then(&self, other: Self, kind: ManeuverType) -> Option<Self> {
        if self.end() >= other.start() {
            return None;
        }

        let dvs: Vec<_> = self.dvs().chain(other.dvs()).collect();

        ManeuverPlan::new(self.end(), kind, *self.initial().sparse(), &dvs)
    }
}

impl std::fmt::Display for ManeuverPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Maneuver Plan ({:?}) ({:0.1})\n", self.kind, self.dv())?;
        if self.segments.is_empty() {
            return write!(f, " (empty)");
        }
        for (i, node) in self.segments.iter().enumerate() {
            let endline = if i + 1 < self.segments.len() {
                "\n"
            } else {
                ""
            };
            write!(
                f,
                "{}. {:?} dV {:0.1} to {:?} orbit{}",
                i + 1,
                node.range,
                node.impulse.unwrap_or(Vec2::ZERO),
                node.orbit.sparse().class(),
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
    Compound,
}

#[derive(Debug, Clone)]
struct ManeuverSegment {
    range: TimeRange,
    orbit: DenseOrbit,
    impulse: Option<Vec2>,
}

impl ManeuverSegment {
    fn next(&self, dv: Vec2, stamp: Nanotime, end: Option<Nanotime>) -> Option<Self> {
        let pv = self.orbit.sparse().pv(stamp).ok()? + PV::vel(dv);
        let orbit = SparseOrbit::from_pv(pv, self.orbit.sparse().body, stamp)?;
        let dense = if let Some(end) = end {
            DenseOrbit::in_range(&orbit, stamp, end)?
        } else {
            DenseOrbit::new(&orbit)
        };
        Some(Self {
            range: TimeRange::maybe_between(stamp, end),
            orbit: dense,
            impulse: Some(dv),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ManeuverNode {
    pub stamp: Nanotime,
    pub impulse: PV,
    // TODO remove?
    pub orbit: SparseOrbit,
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
    let r1 = current.periapsis_r();
    let r2 = destination.radius_at_angle(current.arg_periapsis + PI);
    let a_transfer = (r1 + r2) / 2.0;
    let v1 = vis_viva_equation(mu, r1, a_transfer);

    let t1 = current.t_next_p(now)?;
    let before = current.pv(t1).ok()?;
    let prograde = before.vel.normalize_or_zero();
    let after = PV::new(before.pos, prograde * v1);

    let dv1 = after.vel - before.vel;

    let transfer_orbit = SparseOrbit::from_pv(after, current.body, t1)?;

    let t2 = t1 + transfer_orbit.period()? / 2;
    let before = transfer_orbit.pv(t2).ok()?;
    let (after, _) = destination.nearest(before.pos);
    let after = PV::new(before.pos, after.vel);

    let dv2 = after.vel - before.vel;

    ManeuverPlan::new(
        now,
        ManeuverType::Hohmann,
        *current,
        &[(t1, dv1), (t2, dv2)],
    )
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

    let r1 = current.semi_major_axis;
    let r2 = destination.semi_major_axis;

    let (r1, r2) = (r1.min(r2), r1.max(r2));

    if r2 / r1 < 11.94 {
        // hohmann transfer is always better
        return None;
    }

    // if ratio is greater than 15.58, any bi-elliptic transfer is better

    let rb = current.apoapsis_r().max(destination.apoapsis_r()) * 2.0;

    if rb > current.body.soi * 0.9 {
        return None;
    }

    let intermediate =
        SparseOrbit::circular(rb, current.body, Nanotime::zero(), current.is_retrograde());

    let p1 = hohmann_transfer(current, &intermediate, now)?;

    let (_, _, intermediate) = p1.orbits().skip(1).next()?;

    let p2 = hohmann_transfer(intermediate.sparse(), destination, p1.end())?;

    p1.and_then(p2, ManeuverType::Bielliptic)
}

fn direct_transfer(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    get_next_intersection(now, current, destination)
        .ok()
        .flatten()
        .map(|(t, pvf)| {
            let pvi = current.pv(t).ok()?;
            ManeuverPlan::new(
                now,
                ManeuverType::Direct,
                *current,
                &[(t, pvf.vel - pvi.vel)],
            )
        })
        .flatten()
}

fn generate_maneuver_plans(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Vec<ManeuverPlan> {
    if current.is_retrograde() != destination.is_retrograde() {
        return vec![];
    }

    // let destination = if current.is_retrograde() == destination.is_retrograde() {
    //     *destination
    // } else {
    //     match destination.inverse() {
    //         Some(d) => d,
    //         None => return vec![],
    //     }
    // };

    let direct = direct_transfer(current, &destination, now);
    let hohmann = hohmann_transfer(current, &destination, now);
    let bielliptic = bielliptic_transfer(current, &destination, now);

    [direct, hohmann, bielliptic]
        .into_iter()
        .flatten()
        .collect()
}

pub fn best_maneuver_plan(
    current: &SparseOrbit,
    destination: &SparseOrbit,
    now: Nanotime,
) -> Option<ManeuverPlan> {
    let mut plans = generate_maneuver_plans(current, destination, now);
    plans.sort_by_key(|m| (m.dv() * 1000.0) as i32);
    plans.first().cloned()
}

#[derive(Debug, Clone, Copy)]
pub enum TimeRange {
    None,
    After(Nanotime),
    Before(Nanotime),
    Between(Nanotime, Nanotime),
}

impl TimeRange {
    pub fn after(start: Nanotime) -> Self {
        Self::After(start)
    }

    pub fn before(end: Nanotime) -> Self {
        Self::Before(end)
    }

    pub fn between(start: Nanotime, end: Nanotime) -> Self {
        Self::Between(start, end)
    }

    pub fn maybe_between(start: Nanotime, end: Option<Nanotime>) -> Self {
        if let Some(end) = end {
            Self::Between(start, end)
        } else {
            Self::After(start)
        }
    }

    pub fn none() -> Self {
        TimeRange::None
    }

    pub fn start(&self) -> Option<Nanotime> {
        match self {
            TimeRange::None | TimeRange::Before(_) => None,
            TimeRange::After(start) | TimeRange::Between(start, _) => Some(*start),
        }
    }

    pub fn end(&self) -> Option<Nanotime> {
        match self {
            TimeRange::None | TimeRange::After(_) => None,
            TimeRange::Before(end) | TimeRange::Between(_, end) => Some(*end),
        }
    }

    pub fn includes(&self, t: Nanotime) -> bool {
        match self {
            TimeRange::None => false,
            TimeRange::Before(end) => t <= *end,
            TimeRange::After(start) => *start <= t,
            TimeRange::Between(start, end) => *start <= t && t <= *end,
        }
    }
}
