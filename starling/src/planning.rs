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

pub fn get_future_path(
    sys: &OrbitalSystem,
    id: ObjectId,
    start: Nanotime,
    end: Nanotime,
) -> Result<Option<(Nanotime, EventType)>, PredictError> {

    let max_iters = 100;

    let mut t = start;
    let obj = sys
        .lookup(id, start)
        .ok_or(PredictError::Lookup)?
        .object;

    // TODO consider frame-relative velocity, not global velocity!
    let v1 = sys
        .lookup(id, start)
        .ok_or(PredictError::Lookup)?
        .pv()
        .vel;
    let v2 = sys
        .lookup(id, end)
        .ok_or(PredictError::Lookup)?
        .pv()
        .vel;
    let v = ((v1 + v2) * 0.5).length();
    let dt = Nanotime::secs_f32(40.0 / v);

    if dt < Nanotime(100) {
        return Err(PredictError::BadTimeDelta);
    }

    while t < end {
        let pos = obj.orbit.pv_at_time(t).pos;

        if pos.length() < sys.primary.radius && t > start {
            let p_before = obj.orbit.pv_at_time(t - dt).pos;
            if p_before.length() >= sys.primary.radius {
                let tend = binary_search(t - dt, t, Nanotime(5), max_iters, |s: Nanotime| {
                    obj.orbit.pv_at_time(s).pos.length() > sys.primary.radius
                })
                .map_err(|e| PredictError::Collision(e))?;
                return Ok(Some((tend, EventType::Collide)));
            }
        }

        if pos.length() > sys.primary.soi {
            let tend = binary_search(t - dt, t, Nanotime(5), max_iters, |s: Nanotime| {
                obj.orbit.pv_at_time(s).pos.length() < sys.primary.soi
            })
            .map_err(|e| PredictError::Escape(e))?;
            return Ok(Some((tend, EventType::Escape)));
        }

        for (sysobj, ss) in &sys.subsystems {
            let d1 = mutual_separation(&obj.orbit, &sysobj.orbit, t - dt);
            let d2 = mutual_separation(&obj.orbit, &sysobj.orbit, t);
            if d1 > ss.primary.soi && d2 <= ss.primary.soi {
                let tend = binary_search(t - dt, t, Nanotime(5), max_iters, |s: Nanotime| {
                    let d = mutual_separation(&obj.orbit, &sysobj.orbit, s);
                    d > ss.primary.soi
                })
                .map_err(|e| PredictError::Encounter(e))?;
                return Ok(Some((tend, EventType::Encounter(sysobj.id))));
            }
        }

        t += dt;
    }

    Ok(None)
}
