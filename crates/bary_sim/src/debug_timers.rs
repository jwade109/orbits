use std::collections::*;
use std::time::Duration;
use std::time::Instant;

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
