use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct WallTimer {
    next_firing: Instant,
    last_visited: Instant,
    duration: Duration,
    times_fired: usize,
    dt: Duration,
    fired_last_tick: bool,
    firings: Vec<Instant>,
}

impl std::fmt::Debug for WallTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WallTimer")
            .field("next_firing", &self.next_firing)
            .field("last_visited", &self.last_visited)
            .field("duration", &self.duration)
            .field("times_fired", &self.times_fired)
            .field("dt", &self.dt)
            .field("fired_last_tick", &self.fired_last_tick)
            .field("firings", &self.firings.len())
            .finish()
    }
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
            firings: Vec::new(),
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        self.dt = now - self.last_visited;
        self.last_visited = now;
        self.fired_last_tick = now >= self.next_firing;

        if self.fired_last_tick {
            self.next_firing += self.duration;
            self.times_fired += 1;
            self.firings.push(now);
            if self.firings.len() > 100 {
                self.firings.remove(0);
            }
        }
        self.fired_last_tick
    }

    pub fn nominal_rate(&self) -> f64 {
        1.0 / self.duration.as_secs_f64()
    }

    pub fn actual_rate(&self) -> f64 {
        if self.firings.len() < 2 {
            return 0.0;
        }

        let first = self.firings.first().unwrap();
        let last = self.firings.last().unwrap();
        let delta = *last - *first;
        if delta == Duration::ZERO {
            return 0.0;
        }

        (self.firings.len() - 1) as f64 / delta.as_secs_f64()
    }
}
