// use crate::sounds::SoundEffects;
// use crate::*;
// use crate::{sim::*, utils::WallTimer};
// use std::time::{Duration, Instant};

// pub struct WorldRunner {
//     wall_timer: WallTimer,
// }

// fn frame_update(
//     world: &mut World,
//     client: &mut ClientSpecificInfo,
//     sounds: &mut SoundEffects,
// ) -> DebugTimers {
//     pre_simulation_update(world, client, sounds);
//     let mut timers = DebugTimers::default();
//     timers += update_world(world);
//     post_simulation_update(world, client, sounds);
//     timers
// }

// impl WorldRunner {
//     pub const TICK_DURATION: Duration = Duration::from_millis(20);

//     pub fn new() -> Self {
//         Self {
//             wall_timer: WallTimer::with_dur(Self::TICK_DURATION),
//         }
//     }
// }
