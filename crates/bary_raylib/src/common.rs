use crate::{
    sim::{Part, Thruster},
    *,
};
use bary_core::prelude::{Ent, Isometry2d};
use bary_parts::PartPrototype;
use bary_sim::Computer;
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

pub type MessageQueue<T> = Arc<SegQueue<T>>;

pub fn new_message_queue<T>() -> MessageQueue<T> {
    Arc::new(SegQueue::new())
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum MessageLevel {
    Telemetry,
    Command,
    Response,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    pub source: String,
    pub level: MessageLevel,
    pub kind: MessageKind,
}

impl Message {
    pub fn new(source: impl Into<String>, level: MessageLevel, kind: MessageKind) -> Self {
        Self {
            source: source.into(),
            level,
            kind,
        }
    }

    pub fn telemetry(source: impl Into<String>, kind: MessageKind) -> Self {
        Self::new(source, MessageLevel::Telemetry, kind)
    }

    pub fn response(source: impl Into<String>, kind: MessageKind) -> Self {
        Self::new(source, MessageLevel::Response, kind)
    }

    pub fn command(source: impl Into<String>, kind: MessageKind) -> Self {
        Self::new(source, MessageLevel::Command, kind)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum MessageKind {
    Pong,
    Ping,
    CurrentTick(u64),
    Introduction { username: String },
    Text(String),
    SetSimSpeed(u32),
    GridPos(String, Isometry2d),
    Ack,
    Nack,
    Unsupported,
    FindGridByName(String),
    Entity(Ent),
    ServerInfo { connected_users: usize },
    ListGrids,
    ListProtos,
    ListParts,
    ListThrusters,
    ListComputers,
    GridInfo(Ent, String, Isometry2d),
    Proto(Ent, PartPrototype),
    Part(Ent, Part),
    Thruster(Ent, Thruster),
    Computer(Ent, Computer),
}

impl MessageKind {
    pub fn with_source(self, s: impl Into<String>) -> Message {
        Message {
            source: s.into(),
            level: MessageLevel::Telemetry,
            kind: self,
        }
    }
}

impl From<MessageKind> for Message {
    fn from(value: MessageKind) -> Self {
        Self {
            source: String::new(),
            level: MessageLevel::Telemetry,
            kind: value,
        }
    }
}

pub fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}
