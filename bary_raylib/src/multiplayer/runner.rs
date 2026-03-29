use crate::client::*;
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

    pub fn update(&mut self) -> (Vec<Action>, SoundEffects) {
        let now = Instant::now();
        let delta = now - self.last_update;
        self.nominal_world_duration += delta * self.world.tick_rate;
        self.last_update = now;

        let mut sounds = SoundEffects::new();

        pre_simulation_update(&mut self.world, &mut self.client_info, &mut sounds);

        while apparent_elapsed_time(&mut self.world) < self.nominal_world_duration {
            update_world(&mut self.world);
        }

        post_simulation_update(&mut self.world, &mut self.client_info, &mut sounds);

        (vec![], sounds)
    }
}
