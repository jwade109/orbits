use bary_core::prelude::Vec2;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

pub const PROTOCOL_ID: u64 = 7;

pub const SERVER_PUBLIC_ADDR: &'static str = "127.0.0.1:0";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ClientMessage {
    Pong(u64, Duration),
    Introduction { username: String },
    Text(String),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerMessage {
    Ping(u64, Duration),
    ShipPosition(Vec2),
}

pub fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}
