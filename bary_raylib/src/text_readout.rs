use log::debug;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub type ReadoutLog = (SystemTime, String);

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Readout {
    lines: Vec<ReadoutLog>,
}

impl Readout {
    pub fn log(&mut self, s: impl Into<String>) {
        let now = SystemTime::now();
        let s = s.into();
        debug!("{:?}: {}", now, s);
        self.lines.push((now, s));
    }

    pub fn new() -> Self {
        let mut ret = Self::default();

        for _ in 0..20 {
            ret.log("Implement me!");
        }

        ret
    }
}
