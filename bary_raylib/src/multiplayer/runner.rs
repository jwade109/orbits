use crate::client::*;
use crate::imgui::ImGui;
use crate::multiplayer::*;
use crate::sim::*;
use crate::sounds::SoundEffects;
use std::time::{Duration, Instant};

pub struct WorldRunner {
    last_update: Instant,
    nominal_world_duration: Duration,
}

fn run_timer(mut func: impl FnMut()) -> Duration {
    let start = Instant::now();
    func();
    Instant::now() - start
}

fn update_headless(world: &mut World, nominal_world_dur: Duration) {
    while apparent_elapsed_time(world) < nominal_world_dur {
        update_world(world);
    }
}

fn frame_update(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    nominal_world_dur: Duration,
    sounds: &mut SoundEffects,
) -> Duration {
    pre_simulation_update(world, client, sounds);
    let physics_time = run_timer(|| {
        update_headless(world, nominal_world_dur);
    });
    post_simulation_update(world, client, sounds);
    physics_time
}

impl WorldRunner {
    pub const TICK_DURATION: Duration = Duration::from_millis(20);

    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            nominal_world_duration: Duration::ZERO,
        }
    }

    pub fn update_headless(&mut self, world: &mut World) {
        let now = Instant::now();
        let delta = now - self.last_update;
        self.nominal_world_duration += delta * world.tick_rate;
        self.last_update = now;
        update_headless(world, self.nominal_world_duration);
    }

    pub fn update(
        &mut self,
        world: &mut World,
        client: &mut ClientSpecificInfo,
        debug: &mut DebugInfo,
        sounds: &mut SoundEffects,
        _actions: &mut Vec<Action>,
    ) {
        let now = Instant::now();
        let delta = now - self.last_update;
        self.nominal_world_duration += delta * world.tick_rate;
        self.last_update = now;

        debug.timers.physics = frame_update(world, client, self.nominal_world_duration, sounds);
    }
}
