use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct WallTimer {
    next_firing: Instant,
    last_visited: Instant,
    duration: Duration,
    times_fired: usize,
    dt: Duration,
    fired_last_tick: bool,
    firings: Vec<Instant>,
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
        self.times_fired += 1;
        while self.next_firing < now {
            self.next_firing += self.duration;
        }
        if self.fired_last_tick {
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
