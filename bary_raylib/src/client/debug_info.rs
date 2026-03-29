use std::time::Duration;

#[derive(Default)]
pub struct DebugInfo {
    pub timers: Timers,
}

#[derive(Default)]
pub struct Timers {
    pub physics: Duration,
    pub render: Duration,
    pub total: Duration,
}
