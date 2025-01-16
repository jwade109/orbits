use crate::core::*;
use bevy::math::Vec2;

// determines timestamp where condition goes from true to false
pub fn binary_search_recurse(
    start: (Nanotime, bool),
    end: (Nanotime, bool),
    tol: Nanotime,
    cond: impl Fn(Nanotime) -> bool,
) -> Option<Nanotime> {
    let midpoint = start.0 + (end.0 - start.0) / 2;
    if end.0 - start.0 < tol {
        return Some(midpoint);
    }

    let a = start.1;
    let b = cond(midpoint);
    let c = end.1;

    if !a {
        None
    } else if a && !b {
        binary_search_recurse(start, (midpoint, b), tol, cond)
    } else if b && !c {
        binary_search_recurse((midpoint, b), end, tol, cond)
    } else {
        None
    }
}

pub fn binary_search(
    start: Nanotime,
    end: Nanotime,
    tol: Nanotime,
    cond: impl Fn(Nanotime) -> bool,
) -> Option<Nanotime> {
    let a = cond(start);
    let c = cond(end);
    binary_search_recurse((start, a), (end, c), tol, cond)
}

pub fn get_future_path(
    sys: &OrbitalSystem,
    id: ObjectId,
    start: Nanotime,
    end: Nanotime,
    dt: Nanotime,
) -> Option<(Vec<Vec2>, Option<Nanotime>)> {
    if sys.otype(id)? != ObjectType::Orbiter {
        return None;
    }

    let mut t = start;
    let obj = sys.lookup(id)?;
    let mut ret = vec![];
    while t < end {
        let pos = obj.orbit.pv_at_time(t).pos;

        if pos.length() < sys.primary.radius {
            let tend = binary_search(t - dt, t, Nanotime(100), |s: Nanotime| {
                obj.orbit.pv_at_time(s).pos.length() > sys.primary.radius
            });
            let pend = obj.orbit.pv_at_time(tend.unwrap_or(t)).pos;
            ret.push(pend);
            return Some((ret, tend));
        }

        if pos.length() > sys.primary.soi {
            let tend = binary_search(t - dt, t, Nanotime(100), |s: Nanotime| {
                obj.orbit.pv_at_time(s).pos.length() < sys.primary.soi
            });
            let pend = obj.orbit.pv_at_time(tend.unwrap_or(t)).pos;
            ret.push(pend);
            return Some((ret, tend));
        }

        for (obj, ss) in &sys.subsystems {
            let spos = obj.orbit.pv_at_time(t).pos;
            let d = pos.distance(spos);
            if d < ss.primary.soi {
                let tend = binary_search(t - dt, t, Nanotime(100), |s: Nanotime| {
                    let p1 = obj.orbit.pv_at_time(s).pos;
                    let p2 = obj.orbit.pv_at_time(s).pos;
                    p1.distance(p2) > ss.primary.soi
                });
                let pend = obj.orbit.pv_at_time(tend.unwrap_or(t)).pos;
                ret.push(pend);
                return Some((ret, tend));
            }
        }

        ret.push(pos);
        t += dt;
    }

    Some((ret, None))
}
