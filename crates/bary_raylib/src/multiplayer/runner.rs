use crate::client::*;
use crate::multiplayer::*;
use crate::sim::*;
use crate::sounds::SoundEffects;
use std::time::{Duration, Instant};

pub struct WorldRunner {
    last_update: Instant,
}

fn update_headless(world: &mut World, nominal_world_dur: Duration) -> DebugTimers {
    let start = Instant::now();
    let max_dur = Duration::from_millis(10);
    let mut dur = Duration::ZERO;

    let mut timers = DebugTimers::default();

    while apparent_elapsed_time(world) < nominal_world_dur && dur < max_dur {
        timers += update_world(world);
        dur = Instant::now() - start;
    }

    timers
}

fn frame_update(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    nominal_world_dur: Duration,
    sounds: &mut SoundEffects,
) -> DebugTimers {
    pre_simulation_update(world, client, sounds);
    let mut timers = DebugTimers::default();
    timers += update_headless(world, nominal_world_dur);
    post_simulation_update(world, client, sounds);
    timers
}

impl WorldRunner {
    pub const TICK_DURATION: Duration = Duration::from_millis(20);

    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
        }
    }

    pub fn update_headless(&mut self, world: &mut World) {
        let now = Instant::now();
        let delta = now - self.last_update;
        let world_time = apparent_elapsed_time(world);
        let nominal_world_dur = world_time + delta * world.tick_rate;
        self.last_update = now;
        update_headless(world, nominal_world_dur);
    }

    pub fn update(
        &mut self,
        world: &mut World,
        client: &mut ClientSpecificInfo,
        sounds: &mut SoundEffects,
        _actions: &mut Vec<Action>,
    ) -> DebugTimers {
        let now = Instant::now();
        let delta = now - self.last_update;
        let world_time = apparent_elapsed_time(world);
        let nominal_world_dur = world_time + delta * world.tick_rate;
        self.last_update = now;

        frame_update(world, client, nominal_world_dur, sounds)
    }
}
