use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub struct PVS {
    pub stamp: Duration,
    pub pv: PV,
}

pub fn get_approach_info(
    system: &OrbitalSystem,
    a: ObjectId,
    b: ObjectId,
    start: Duration,
    end: Duration,
) -> Option<Vec<(PVS, PVS, f32)>> {
    let pa = get_future_positions(system, a, start, end, 100);
    let pb = get_future_positions(system, b, start, end, 100);

    let distances: Vec<_> = pa
        .into_iter()
        .zip(pb.into_iter())
        .map(|e| (e.0, e.1, e.0.pv.pos.distance(e.1.pv.pos)))
        .collect();

    // distances
    //     .iter()
    //     .zip(distances.iter().skip(1))
    //     .map(|(a, b)| {
    //     });

    Some(distances)

    // let tol = Duration::from_millis(10);

    // if end - start < tol {
    //     return Some(ret);
    // }

    // let mid = (end + start) / 2;

    // if ret.0.stamp < mid {
    //     get_approach_info(system, a, b, start, mid)
    // } else {
    //     get_approach_info(system, a, b, mid, end)
    // }
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
