use chrono::{DateTime, Local};
use log::debug;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ChatLog {
    instant: Instant,
    local_time: DateTime<Local>,
    text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Chat {
    lines: Vec<ChatLog>,
}

pub fn format_log(log: &ChatLog) -> String {
    format!(
        "[{}] {}",
        log.local_time.format("%Y-%m-%d %H:%M:%S"),
        log.text
    )
}

impl Chat {
    pub fn log(&mut self, s: impl Into<String>) {
        let local_time = chrono::offset::Local::now();
        let s = s.into();
        debug!("{:?}: {}", local_time, s);
        self.lines.push(ChatLog {
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

    pub fn logs(&self) -> impl Iterator<Item = &ChatLog> + use<'_> {
        self.lines.iter()
    }
}
