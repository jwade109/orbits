use crate::core::*;
use crate::orbit::Orbit;
use bevy::math::Vec2;

#[derive(Debug, Clone, Copy)]
pub enum ConvergeError {
    Initial((Nanotime, bool), (Nanotime, bool)),
    Final((Nanotime, bool), (Nanotime, bool)),
}

// determines timestamp where condition goes from true to false
pub fn binary_search_recurse(
    start: (Nanotime, bool),
    end: (Nanotime, bool),
    tol: Nanotime,
    cond: impl Fn(Nanotime) -> bool,
) -> Result<Nanotime, ConvergeError> {
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
        binary_search_recurse(start, (midpoint, b), tol, cond)
    } else if b && !c {
        binary_search_recurse((midpoint, b), end, tol, cond)
    } else {
        Err(ConvergeError::Final(start, end))
    }
}

pub fn binary_search(
    start: Nanotime,
    end: Nanotime,
    tol: Nanotime,
    cond: impl Fn(Nanotime) -> bool,
) -> Result<Nanotime, ConvergeError> {
    let a = cond(start);
    let c = cond(end);
    binary_search_recurse((start, a), (end, c), tol, cond)
}

#[derive(Debug, Clone, Copy)]
pub enum PredictError {
    BadType,
    Lookup,
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
) -> Result<(Vec<Vec2>, Option<Nanotime>), PredictError> {
    if sys.otype(id).ok_or(PredictError::Lookup)? != ObjectType::Orbiter {
        return Err(PredictError::BadType);
    }

    let mut t = start;
    let obj = sys.lookup(id).ok_or(PredictError::Lookup)?;

    let v1 = sys.pv(id, start).ok_or(PredictError::Lookup)?.vel;
    let v2 = sys.pv(id, end).ok_or(PredictError::Lookup)?.vel;
    let v = ((v1 + v2) * 0.5).length();
    let dt = Nanotime::secs_f32(40.0 / v);

    let mut ret = vec![];
    while t < end {
        let pos = obj.orbit.pv_at_time(t).pos;

        if pos.length() < sys.primary.radius && t > start {
            let p_before = obj.orbit.pv_at_time(t - dt).pos;
            if p_before.length() >= sys.primary.radius {
                let tend = binary_search(t - dt, t, Nanotime(5), |s: Nanotime| {
                    obj.orbit.pv_at_time(s).pos.length() > sys.primary.radius
                })
                .map_err(|e| PredictError::Collision(e))?;
                let pend = obj.orbit.pv_at_time(tend).pos;
                ret.push(pend);
                return Ok((ret, Some(tend)));
            }
        }

        if pos.length() > sys.primary.soi {
            let tend = binary_search(t - dt, t, Nanotime(5), |s: Nanotime| {
                obj.orbit.pv_at_time(s).pos.length() < sys.primary.soi
            })
            .map_err(|e| PredictError::Escape(e))?;
            let pend = obj.orbit.pv_at_time(tend).pos;
            ret.push(pend);
            return Ok((ret, Some(tend)));
        }

        for (sysobj, ss) in &sys.subsystems {
            let d1 = mutual_separation(&obj.orbit, &sysobj.orbit, t - dt);
            let d2 = mutual_separation(&obj.orbit, &sysobj.orbit, t);
            if d1 > ss.primary.soi && d2 <= ss.primary.soi {
                let tend = binary_search(t - dt, t, Nanotime(5), |s: Nanotime| {
                    let d = mutual_separation(&obj.orbit, &sysobj.orbit, s);
                    d > ss.primary.soi
                })
                .map_err(|e| PredictError::Encounter(e))?;
                let pend = obj.orbit.pv_at_time(tend).pos;
                ret.push(pend);
                return Ok((ret, Some(tend)));
            }
        }

        ret.push(pos);
        t += dt;
    }

    Ok((ret, None))
}
