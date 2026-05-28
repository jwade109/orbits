use std::time::{Duration, Instant};

pub struct WallTimer {
    next_firing: Instant,
    duration: Duration,
}

impl WallTimer {
    pub fn with_dur(duration: Duration) -> Self {
        Self {
            next_firing: Instant::now(),
            duration,
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        if now < self.next_firing {
            return false;
        }

        // fire!
        // TODO(gross) we shouldn't need a while loop here
        while self.next_firing < now {
            self.next_firing += self.duration;
        }

        true
    }
}
