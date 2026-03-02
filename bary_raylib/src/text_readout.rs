use chrono::{DateTime, Local};
use log::debug;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ReadoutLog {
    instant: Instant,
    local_time: DateTime<Local>,
    text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Readout {
    lines: Vec<ReadoutLog>,
}

pub fn format_log(log: &ReadoutLog) -> String {
    format!(
        "[{}] {}",
        log.local_time.format("%Y-%m-%d %H:%M:%S"),
        log.text
    )
}

impl Readout {
    pub fn log(&mut self, s: impl Into<String>) {
        let local_time = chrono::offset::Local::now();
        let s = s.into();
        debug!("{:?}: {}", local_time, s);
        self.lines.push(ReadoutLog {
            instant: Instant::now(),
            local_time,
            text: s,
        });
    }

    pub fn drop_old_messages(&mut self) {
        let now = Instant::now();
        self.lines
            .retain(|l| now - l.instant < Duration::from_secs(5));
    }

    pub fn logs(&self) -> impl Iterator<Item = &ReadoutLog> + use<'_> {
        self.lines.iter()
    }

    pub fn new() -> Self {
        let mut ret = Self::default();

        for _ in 0..20 {
            ret.log("Implement me!");
        }

        ret
    }
}
