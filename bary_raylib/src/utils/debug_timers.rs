use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct DebugTimers {
    pub ticks: u64,
    pub timers: BTreeMap<String, Duration>,
}

impl DebugTimers {
    pub fn total(&self) -> Duration {
        self.timers.iter().map(|e| e.1).sum()
    }

    pub fn scope<'a>(&'a mut self, name: &str) -> ScopeTimer<'a> {
        ScopeTimer {
            timers: self,
            name: name.to_string(),
            start: Instant::now(),
        }
    }

    pub fn to_pie_chart(&self) -> Vec<f32> {
        self.timers.iter().map(|s| s.1.as_secs_f32()).collect()
    }

    pub fn update(&mut self, other: &DebugTimers) {
        for (k, new_value) in &other.timers {
            self.timers
                .entry(k.clone())
                .and_modify(|old_value| {
                    if *new_value > *old_value {
                        let delta = *new_value - *old_value;
                        *old_value += delta / 100;
                    } else {
                        let delta = *old_value - *new_value;
                        *old_value -= delta / 100;
                    }
                })
                .or_insert(*new_value);
        }
    }
}

impl std::ops::AddAssign for DebugTimers {
    fn add_assign(&mut self, rhs: Self) {
        self.ticks += rhs.ticks;
        for (k, v) in rhs.timers {
            self.timers.entry(k).and_modify(|e| *e += v).or_insert(v);
        }
    }
}

pub struct ScopeTimer<'a> {
    timers: &'a mut DebugTimers,
    name: String,
    start: Instant,
}

impl<'a> Drop for ScopeTimer<'a> {
    fn drop(&mut self) {
        let dur = Instant::now() - self.start;
        self.timers
            .timers
            .entry(self.name.clone())
            .and_modify(|e| *e += dur)
            .or_insert(dur);
    }
}
