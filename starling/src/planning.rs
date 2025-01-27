use crate::core::*;
use crate::orbit::*;
use crate::orbiter::*;
use bevy::math::Vec2;

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
    cond: impl Fn(T) -> bool,
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
    Collision(ConvergeError<T>),
    Escape(ConvergeError<T>),
    Encounter(ConvergeError<T>),
}

fn mutual_separation(o1: &Orbit, o2: &Orbit, t: Nanotime) -> f32 {
    let p1 = o1.pv_at_time(t).pos;
    let p2 = o2.pv_at_time(t).pos;
    p1.distance(p2)
}

fn search_condition<T: BinarySearchKey>(
    t1: T,
    t2: T,
    tol: T,
    cond: impl Fn(T) -> bool,
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

#[derive(Debug, Clone, Copy)]
pub struct Propagator {
    stamp: Nanotime,
    dt: Nanotime,
    finished: bool,
}

impl Propagator {
    pub fn new(stamp: Nanotime) -> Self {
        Propagator {
            stamp,
            dt: Nanotime(0),
            finished: false,
        }
    }

    pub fn stamp(&self) -> Nanotime {
        self.stamp
    }

    pub fn calculated_to(&self, stamp: Nanotime) -> bool {
        return self.finished || self.stamp >= stamp;
    }

    pub fn reset(&mut self, stamp: Nanotime) {
        self.finished = false;
        self.stamp = stamp;
    }

    pub fn freeze(&mut self, stamp: Nanotime) {
        self.finished = true;
        self.stamp = stamp;
    }

    pub fn next(
        &mut self,
        ego: &Orbit,
        radius: f32,
        soi: f32,
        bodies: &[(ObjectId, Orbit, f32)],
    ) -> Result<Option<(Nanotime, EventType)>, PredictError<Nanotime>> {
        if self.finished {
            return Ok(None);
        }

        let tol = Nanotime(5);

        let can_hit_planet = ego.periapsis_r() <= radius;
        let can_escape = ego.eccentricity >= 1.0 || ego.apoapsis_r() >= soi;
        let near_planet = bodies
            .iter()
            .any(|(_, orb, soi)| mutual_separation(ego, orb, self.stamp) < soi * 3.0);

        self.dt = if can_hit_planet {
            Nanotime::millis(20)
        } else if can_escape {
            Nanotime::secs(2)
        } else if near_planet {
            Nanotime::millis(500)
        } else {
            Nanotime::secs(5)
        };

        let t1 = self.stamp;
        let t2 = self.stamp + self.dt;

        self.stamp = t2;

        let above_planet = |t: Nanotime| {
            let pos = ego.pv_at_time(t).pos;
            pos.length() > radius
        };

        let escape_soi = |t: Nanotime| {
            let pos = ego.pv_at_time(t).pos;
            pos.length() < soi
        };

        let encounter_nth = |i: usize| {
            move |t: Nanotime| {
                let (_, orbit, soi) = bodies[i];
                mutual_separation(&ego, &orbit, t) > soi
            }
        };

        if can_hit_planet {
            if !above_planet(t1) {
                self.stamp = t1;
                self.finished = true;
                return Ok(Some((t1, EventType::Collide)));
            }

            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, above_planet)
                .map_err(|e| PredictError::Collision(e))?
            {
                self.stamp = t;
                self.finished = true;
                return Ok(Some((t, EventType::Collide)));
            }
        }

        if can_escape {
            if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, escape_soi)
                .map_err(|e| PredictError::Escape(e))?
            {
                self.stamp = t;
                self.finished = true;
                return Ok(Some((t, EventType::Escape)));
            }
        }

        if near_planet {
            for i in 0..bodies.len() {
                let cond = encounter_nth(i);
                let id = bodies[i].0;
                if let Some(t) = search_condition::<Nanotime>(t1, t2, tol, cond)
                    .map_err(|e| PredictError::Encounter(e))?
                {
                    self.stamp = t;
                    self.finished = true;
                    return Ok(Some((t, EventType::Encounter(id))));
                }
            }
        }

        Ok(None)
    }
}

pub fn find_intersections(
    o1: &Orbit,
    o2: &Orbit,
) -> Result<Option<(f32, f32)>, ConvergeError<f32>> {
    let n = 100;
    let sample_angles = (0..n).map(|i| 2.0 * PI * i as f32 / n as f32);

    let radii = sample_angles
        .map(|a| {
            let r1 = o1.radius_at_angle(a);
            let r2 = o2.radius_at_angle(a);
            (a, r1, r2)
        })
        .collect::<Vec<_>>();

    let c1 = |a: f32| {
        let r1 = o1.radius_at_angle(a);
        let r2 = o2.radius_at_angle(a);
        r1 > r2
    };

    let c2 = |a: f32| {
        let r1 = o1.radius_at_angle(a);
        let r2 = o2.radius_at_angle(a);
        r1 < r2
    };

    let mut ret1 = None;
    let mut ret2 = None;

    // TODO you can calculate the second angle from the first with the
    // power of math; this is inefficient and lazy

    for ((a0, _, _), (a1, _, _)) in radii.windows(2).map(|r| (r[0], r[1])) {
        if let Some(a) = search_condition(a0, a1, 1E-4, c1)? {
            ret1 = Some(a);
        }
        if let Some(a) = search_condition(a0, a1, 1E-4, c2)? {
            ret2 = Some(a);
        }
    }

    Ok(ret1.zip(ret2))
}
