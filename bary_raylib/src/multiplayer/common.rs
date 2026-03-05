use crate::{
    input_state::InputState, multiplayer::*, sounds::SoundEffects, world::{World, update_world}
};
use crossbeam_queue::SegQueue;
use renet_netcode::NETCODE_USER_DATA_BYTES;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
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

pub struct WorldRunner {
    pub world: World,
    last_update: Instant,
}

impl WorldRunner {
    pub const TICK_DURATION: Duration = Duration::from_millis(20);

    pub fn new(world: World) -> Self {
        let now = Instant::now();
        Self {
            world,
            last_update: now,
        }
    }

    pub fn update(&mut self, input: &mut InputState) -> (Vec<Action>, SoundEffects) {
        let now = Instant::now();
        let mut delta = now - self.last_update;
        let mut ret = Vec::new();
        let mut sounds = SoundEffects::default();
        while delta > Self::TICK_DURATION {
            delta -= Self::TICK_DURATION;
            let (actions, s) = update_world(&mut self.world, input);
            ret.extend_from_slice(&actions);
            sounds.effects.extend_from_slice(&s.effects);
            self.last_update += Self::TICK_DURATION;
        }
        (ret, sounds)
    }
}
