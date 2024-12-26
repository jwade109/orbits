use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub struct PVS {
    pub stamp: Duration,
    pub pv: PV,
}

pub fn get_minimum_approach(
    system: &OrbitalSystem,
    a: ObjectId,
    b: ObjectId,
    start: Duration,
    end: Duration,
) -> Option<(PVS, PVS)> {
    // assuming there is one global minimum in this time range
    let pa = get_future_positions(system, a, start, end, 20);
    let pb = get_future_positions(system, b, start, end, 20);

    let min = pa.into_iter().zip(pb.into_iter()).min_by(|e1, e2| {
        let d1 = e1.0.pv.pos.distance_squared(e1.1.pv.pos);
        let d2 = e2.0.pv.pos.distance_squared(e2.1.pv.pos);
        d1.total_cmp(&d2)
    })?;

    let tol = Duration::from_micros(30);
    if end - start < tol {
        return Some(min);
    }

    let mid = (end + start) / 2;

    // recurse!
    if min.0.stamp < mid {
        get_minimum_approach(system, a, b, start, mid)
    } else {
        get_minimum_approach(system, a, b, mid, end)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ApproachEvent {
    pub stamp: Duration,
    pub a: (ObjectId, PV),
    pub b: (ObjectId, PV),
}

impl ApproachEvent {
    pub fn dist(&self) -> f32 {
        self.a.1.pos.distance(self.b.1.pos)
    }
}

pub fn get_approach_info(
    system: &OrbitalSystem,
    a: ObjectId,
    b: ObjectId,
    start: Duration,
    end: Duration,
    radius: f32,
) -> Option<Vec<ApproachEvent>> {
    let pa = get_future_positions(system, a, start, end, 50);
    let pb = get_future_positions(system, b, start, end, 50);

    let distances: Vec<_> = pa
        .into_iter()
        .zip(pb.into_iter())
        .map(|e| (e.0, e.1, e.0.pv.pos.distance(e.1.pv.pos)))
        .collect();

    let min_approaches = distances
        .windows(3)
        .filter_map(|e| {
            let r1 = e[0].2;
            let r2 = e[1].2;
            let r3 = e[2].2;

            if r2 > radius {
                return None;
            }

            if r1 > r2 && r2 < r3 {
                let t1 = e[0].0.stamp;
                let t2 = e[2].0.stamp;
                get_minimum_approach(system, a, b, t1, t2)
            } else {
                None
            }
        })
        .map(|e| ApproachEvent {
            stamp: e.0.stamp,
            a: (a, e.0.pv),
            b: (a, e.1.pv),
        })
        .collect();

    Some(min_approaches)
}

pub fn get_future_positions(
    system: &OrbitalSystem,
    id: ObjectId,
    start: Duration,
    end: Duration,
    steps: usize,
) -> Vec<PVS> {
    let dur = end - start;
    (0..steps)
        .filter_map(|i| {
            let t = start + i as u32 * dur / (steps - 1) as u32;
            Some(PVS {
                stamp: t,
                pv: system.transform_from_id(Some(id), t)?,
            })
        })
        .collect()
}
