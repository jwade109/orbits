use crate::client::*;
use crate::imgui::ImGui;
use crate::multiplayer::*;
use crate::sim::*;
use crate::sounds::SoundEffects;
use std::time::{Duration, Instant};

pub struct WorldRunner {
    pub client_info: ClientSpecificInfo,
    pub world: World,
    last_update: Instant,
    nominal_world_duration: Duration,
}

impl WorldRunner {
    pub const TICK_DURATION: Duration = Duration::from_millis(20);

    pub fn new(world: World) -> Self {
        let now = Instant::now();
        Self {
            client_info: ClientSpecificInfo::new(),
            world,
            last_update: now,
            nominal_world_duration: Duration::ZERO,
        }
    }

    pub fn update(
        &mut self,
        debug: &mut DebugInfo,
        sounds: &mut SoundEffects,
        _actions: &mut Vec<Action>,
    ) {
        let now = Instant::now();
        let delta = now - self.last_update;
        self.nominal_world_duration += delta * self.world.tick_rate;
        self.last_update = now;

        pre_simulation_update(&mut self.world, &mut self.client_info, sounds);

        let pre_physics = Instant::now();

        while apparent_elapsed_time(&mut self.world) < self.nominal_world_duration {
            update_world(&mut self.world);
        }

        let post_physics = Instant::now();

        debug.timers.physics = post_physics - pre_physics;

        post_simulation_update(&mut self.world, &mut self.client_info, sounds);
    }
}
