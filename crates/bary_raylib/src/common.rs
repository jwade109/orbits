use crate::*;
use bary_core::prelude::Isometry2d;
use crossbeam_queue::SegQueue;
use renet_netcode::NETCODE_USER_DATA_BYTES;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime},
};

pub type MessageQueue<T> = Arc<SegQueue<T>>;

pub fn new_message_queue<T>() -> MessageQueue<T> {
    Arc::new(SegQueue::new())
}

pub const PROTOCOL_ID: u64 = 7;

pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);

#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMessage {
    Pong(u64, Duration),
    Introduction { username: String },
    Text(String),
    Transaction(Transaction),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerMessage {
    Ping(u64, Duration),
    Text(String),
    Transaction(Transaction),
    GridPos(String, Isometry2d),
    Ack,
    ServerInfo { connected_users: usize },
}

impl ServerMessage {
    pub fn transaction(tick: u64, action: Action) -> Self {
        Self::Transaction(Transaction::new(tick, action))
    }
}

pub fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

// Helper struct to pass an username in the user data
pub struct Username(pub String);

impl Username {
    pub fn to_netcode_user_data(&self) -> [u8; NETCODE_USER_DATA_BYTES] {
        let mut user_data = [0u8; NETCODE_USER_DATA_BYTES];
        if self.0.len() > NETCODE_USER_DATA_BYTES - 8 {
            panic!("Username is too big");
        }
        user_data[0..8].copy_from_slice(&(self.0.len() as u64).to_le_bytes());
        user_data[8..self.0.len() + 8].copy_from_slice(self.0.as_bytes());

        user_data
    }

    pub fn from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> Self {
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(&user_data[0..8]);
        let mut len = u64::from_le_bytes(buffer) as usize;
        len = len.min(NETCODE_USER_DATA_BYTES - 8);
        let data = user_data[8..len + 8].to_vec();
        let username = String::from_utf8(data).unwrap();
        Self(username)
    }
}
