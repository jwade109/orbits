use crate::core::*;
use crate::orbit::Orbit;
use bevy::math::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum ConvergeError {
    Initial((Nanotime, bool), (Nanotime, bool)),
    Final((Nanotime, bool), (Nanotime, bool)),
    MaxIter,
}

// determines timestamp where condition goes from true to false
pub fn binary_search_recurse(
    start: (Nanotime, bool),
    end: (Nanotime, bool),
    tol: Nanotime,
    cond: impl Fn(Nanotime) -> bool,
    remaining: usize,
) -> Result<Nanotime, ConvergeError> {
    if remaining == 0 {
        return Err(ConvergeError::MaxIter);
    }

    if end.0 - start.0 < tol {
        return Ok(end.0);
    }

    let midpoint = start.0 + (end.0 - start.0) / 2;
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

pub fn binary_search(
    start: Nanotime,
    end: Nanotime,
    tol: Nanotime,
    max_iters: usize,
    cond: impl Fn(Nanotime) -> bool,
) -> Result<Nanotime, ConvergeError> {
    let a = cond(start);
    let c = cond(end);
    binary_search_recurse((start, a), (end, c), tol, cond, max_iters)
}

#[derive(Debug, Clone, Copy)]
pub enum PredictError {
    BadType,
    Lookup,
    BadTimeDelta,
    Collision(ConvergeError),
    Escape(ConvergeError),
    Encounter(ConvergeError),
}

fn mutual_separation(o1: &Orbit, o2: &Orbit, t: Nanotime) -> f32 {
    let p1 = o1.pv_at_time(t).pos;
    let p2 = o2.pv_at_time(t).pos;
    p1.distance(p2)
}

fn search_condition(
    t1: Nanotime,
    t2: Nanotime,
    cond: impl Fn(Nanotime) -> bool,
) -> Result<Option<Nanotime>, ConvergeError> {
    let a = cond(t1);
    if !a {
        return Ok(None);
    }
    let b = cond(t2);
    if b {
        return Ok(None);
    }
    binary_search(t1, t2, Nanotime(5), 100, cond).map(|t| Some(t))
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
    ) -> Result<Option<(Nanotime, EventType)>, PredictError> {
        if self.finished {
            return Ok(None);
        }

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

        let hit_planet = |t: Nanotime| {
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
            if let Some(t) =
                search_condition(t1, t2, hit_planet).map_err(|e| PredictError::Collision(e))?
            {
                self.finished = true;
                return Ok(Some((t, EventType::Collide)));
            }
        }

        if can_escape {
            if let Some(t) =
                search_condition(t1, t2, escape_soi).map_err(|e| PredictError::Escape(e))?
            {
                self.finished = true;
                return Ok(Some((t, EventType::Escape)));
            }
        }

        if near_planet {
            for i in 0..bodies.len() {
                let cond = encounter_nth(i);
                let id = bodies[i].0;
                if let Some(t) =
                    search_condition(t1, t2, cond).map_err(|e| PredictError::Encounter(e))?
                {
                    self.finished = true;
                    return Ok(Some((t, EventType::Encounter(id))));
                }
            }
        }

        Ok(None)
    }
}
