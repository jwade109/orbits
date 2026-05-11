use std::time::{Duration, Instant};

pub struct WallTimer {
    last_updated: Instant,
    duration: Duration,
}

impl WallTimer {
    pub fn with_dur(duration: Duration) -> Self {
        Self {
            last_updated: Instant::now(),
            duration,
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let dt = now - self.last_updated;
        if dt >= self.duration {
            self.last_updated = now;
            true
        } else {
            false
        }
    }
}
