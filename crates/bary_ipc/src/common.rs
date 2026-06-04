use bary_core::prelude::{Ent, Isometry2d, TableIdent};
use bary_parts::PartPrototype;
use bary_sim::{Computer, Part, Thruster, WorldDelta};
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::Blob;

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

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientId({})", self.0)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSource {
    Server,
    Client(ClientId),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Message {
    pub source: MessageSource,
    pub level: MessageLevel,
    pub kind: MessageKind,
}

impl Message {
    pub fn new(source: MessageSource, level: MessageLevel, kind: MessageKind) -> Self {
        Self {
            source,
            level,
            kind,
        }
    }

    pub fn ack_tlm(source: MessageSource) -> Self {
        Self::new(source, MessageLevel::Telemetry, MessageKind::Ack)
    }

    pub fn nack_tlm(source: MessageSource) -> Self {
        Self::new(source, MessageLevel::Telemetry, MessageKind::Ack)
    }

    pub fn is_ack(&self) -> bool {
        if let MessageKind::Ack = self.kind {
            true
        } else {
            false
        }
    }

    pub fn is_nack(&self) -> bool {
        if let MessageKind::Nack = self.kind {
            true
        } else {
            false
        }
    }

    pub fn telemetry(source: MessageSource, kind: MessageKind) -> Self {
        Self::new(source, MessageLevel::Telemetry, kind)
    }

    pub fn response(source: MessageSource, kind: MessageKind) -> Self {
        Self::new(source, MessageLevel::Response, kind)
    }

    pub fn command(source: MessageSource, kind: MessageKind) -> Self {
        Self::new(source, MessageLevel::Command, kind)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientTelemetry {
    pub ticks: u64,
}

/// clone of [`renet::NetworkInfo`]
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct NetworkInfo {
    pub rtt: f64,
    pub packet_loss: f64,
    pub bytes_sent_per_second: f64,
    pub bytes_received_per_second: f64,
}

impl From<renet::NetworkInfo> for NetworkInfo {
    fn from(value: renet::NetworkInfo) -> Self {
        Self {
            rtt: value.rtt,
            packet_loss: value.packet_loss,
            bytes_received_per_second: value.bytes_received_per_second,
            bytes_sent_per_second: value.bytes_sent_per_second,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ServerStatistics {
    pub clients: Vec<(u64, NetworkInfo)>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum MessageKind {
    Pong,
    Ping,
    CurrentTick(u64),
    Introduction {
        username: String,
    },
    Text(String),
    SetSimSpeed(u32),
    GridPos(String, Isometry2d),
    Ack,
    Nack,
    Unsupported,
    FindGridByName(String),
    Entity(Ent),
    ServerStatistics(ServerStatistics),
    PrintEntityInfo(TableIdent),
    GridInfo(Ent, String, Isometry2d),
    Proto(Ent, PartPrototype),
    Part(Ent, Part),
    Thruster(Ent, Thruster),
    Computer(Ent, Computer),
    RequestServerStatistics,
    SetWaypoint(Ent, Isometry2d),
    CameraPosition(Isometry2d),
    /// request from the client for table blobs
    ClientBlobRequest(TableIdent),
    /// client requests all blobs
    ClientBlobRequestAll,
    /// server response containing a single blob
    BlobResponse(Blob),
    /// server response containing multiple blobs and the current tick
    MultiBlobResponse(Ent, u64, Vec<Blob>),
    /// information sent consistently by each client
    ClientTelemetry(ClientTelemetry),
    /// driver packet sent to each client each tick
    Driver {
        ticks: u64,
        deltas: Vec<WorldDelta>,
    },
    /// request by the client to perform a certain [`WorldDelta`]
    RequestDelta(WorldDelta),
}

impl MessageKind {
    pub fn with_source(self, source: MessageSource) -> Message {
        Message {
            source,
            level: MessageLevel::Telemetry,
            kind: self,
        }
    }
}

impl From<MessageKind> for Message {
    fn from(kind: MessageKind) -> Self {
        Self {
            source: MessageSource::Client(ClientId(0)),
            level: MessageLevel::Telemetry,
            kind,
        }
    }
}

pub fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}
