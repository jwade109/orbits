use bevy::prelude::*;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

#[derive(Resource, Default)]
pub struct Plots {
    signals: BTreeMap<String, VecDeque<Duration>>,
}

pub const MAX_SAMPLES: usize = 500;

impl Plots {
    pub fn add<'p>(&mut self, name: impl Into<String>, v: Duration) {
        self.signals
            .entry(name.into())
            .and_modify(|s| {
                s.push_back(v);
                if s.len() > MAX_SAMPLES {
                    s.pop_front();
                }
            })
            .or_insert(vec![v].into());
    }

    pub fn signals(&self) -> impl Iterator<Item = (&String, &VecDeque<Duration>)> + use<'_> {
        self.signals.iter()
    }
}
