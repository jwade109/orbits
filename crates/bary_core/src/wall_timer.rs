use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct WallTimer {
    next_firing: Instant,
    last_visited: Instant,
    duration: Duration,
    times_fired: usize,
    dt: Duration,
    fired_last_tick: bool,
}

impl WallTimer {
    pub fn with_dur(duration: Duration) -> Self {
        let now = Instant::now();
        Self {
            next_firing: now,
            last_visited: now,
            duration,
            times_fired: 0,
            dt: Duration::ZERO,
            fired_last_tick: false,
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        self.dt = now - self.last_visited;
        self.last_visited = now;
        self.fired_last_tick = now >= self.next_firing;
        self.times_fired += 1;
        if self.fired_last_tick {
            self.next_firing += self.duration;
        }
        self.fired_last_tick
    }
}
