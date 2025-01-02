use crate::core::*;
use bevy::math::Vec2;
use chrono::TimeDelta;

#[derive(Debug, Copy, Clone)]
pub struct PVS {
    pub stamp: TimeDelta,
    pub pv: PV,
}

fn to_distances<'a>(a: &'a [PVS], b: &'a [PVS]) -> Vec<(&'a PVS, &'a PVS, f32)> {
    a.into_iter()
        .zip(b.into_iter())
        .map(|e| (e.0, e.1, e.0.pv.pos.distance(e.1.pv.pos)))
        .collect()
}

pub fn get_time_at_separation(
    system: &OrbitalSystem,
    a: ObjectId,
    b: ObjectId,
    start: TimeDelta,
    end: TimeDelta,
    radius: f32,
) -> Option<(PVS, PVS)> {
    // assuming d(start) < d(target) < d(end)
    //       or d(start) > d(target) > d(end)
    let pa = get_future_positions(system, a, start, end, 3);
    let pb = get_future_positions(system, b, start, end, 3);

    let distances = to_distances(&pa, &pb);

    let e1 = distances.get(0)?;
    let e2 = distances.get(1)?;
    let e3 = distances.get(2)?;

    let tol = 0.05;

    if (e1.2 - e3.2).abs() < tol {
        let e = distances.get(1)?;
        return Some((*e.0, *e.1));
    }

    let mid = e2.0.stamp;
    if (e1.2 <= radius && radius <= e2.2) || (e1.2 >= radius && radius >= e2.2) {
        // between e1.2 and e2.2, one way or the other
        get_time_at_separation(system, a, b, start, mid, radius)
    } else {
        // between e2.2 and r3
        get_time_at_separation(system, a, b, mid, end, radius)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EncounterEvent {
    pub start: TimeDelta,
    pub end: TimeDelta,
    pub a: ObjectId,
    pub b: ObjectId,
    pub threshold: f32,
}

pub fn get_approach_info(
    system: &OrbitalSystem,
    a: ObjectId,
    b: ObjectId,
    start: TimeDelta,
    end: TimeDelta,
    radius: f32,
) -> Vec<(PVS, PVS)> {
    let pa = get_future_positions(system, a, start, end, 100);
    let pb = get_future_positions(system, b, start, end, 100);

    let distances = to_distances(&pa, &pb);

    return distances
        .windows(2)
        .filter_map(|e| {
            let r1 = e[0].2;
            let r2 = e[1].2;
            if (r1 > radius && r2 <= radius)
                || (r1 >= radius && r2 < radius)
                || (r1 < radius && r2 >= radius)
                || (r1 <= radius && r2 > radius)
            {
                return get_time_at_separation(&system, a, b, e[0].0.stamp, e[1].0.stamp, radius);
            }
            None
        })
        .collect::<Vec<_>>();
}

pub fn get_future_positions(
    system: &OrbitalSystem,
    id: ObjectId,
    start: TimeDelta,
    end: TimeDelta,
    steps: usize,
) -> Vec<PVS> {
    let dur_nanos = match (end - start).num_nanoseconds() {
        Some(d) => d,
        None => return vec![],
    };
    (0..steps)
        .filter_map(|i| {
            let s = i as f32 / (steps - 1) as f32;
            let t = start + TimeDelta::nanoseconds((dur_nanos as f32 * s) as i64);
            Some(PVS {
                stamp: t,
                pv: system.transform_from_id(id, t)?,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum EncounterDir {
    Enter,
    Exit,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SepTracker {
    previous: Option<(TimeDelta, f32)>,
    current: Option<(TimeDelta, f32)>,
}

impl SepTracker {
    pub fn update(&mut self, stamp: TimeDelta, sep: f32) {
        self.previous = self.current;
        self.current = Some((stamp, sep));
    }

    pub fn crosses(&self, sep: f32) -> Option<(EncounterDir, (TimeDelta, f32), (TimeDelta, f32))> {
        let p = self.previous?;
        let c = self.current?;
        if p.1 <= sep && sep <= c.1 {
            Some((EncounterDir::Exit, p, c))
        } else if p.1 >= sep && sep >= c.1 {
            Some((EncounterDir::Enter, p, c))
        } else {
            None
        }
    }
}
