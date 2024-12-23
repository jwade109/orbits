use crate::core::*;

fn get_minimal_system(system: &OrbitalSystem, id: ObjectId) -> OrbitalSystem {
    let mut copy = system.clone();
    copy.objects.retain(|o| o.body.is_some() || o.id == id);
    copy
}

// pub fn get_future_pvs(system: &OrbitalSystem, id: ObjectId, steps: usize) -> (Vec<PV>, bool) {
//     let mut minimal = get_minimal_system(system, id);
//     let positions: Vec<_> = (0..steps)
//         .filter_map(|_| {
//             minimal.step();
//             let frame = minimal.frame(minimal.epoch);
//             Some(frame.lookup(id)?.1)
//         })
//         .collect();
//     let abridged = positions.len() < steps;
//     (positions, abridged)
// }
