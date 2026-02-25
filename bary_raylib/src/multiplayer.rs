use bary_core::prelude::*;
use log::{error, info, warn};
use renet::*;
use renet_netcode::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::*;
use std::time::{Duration, Instant, SystemTime};

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

#[derive(Debug)]
pub struct UserInfo {
    pub last_ping_sent: Instant,
    pub last_message_received: Instant,
    pub expected_ping_check: u64,
    pub username: Option<String>,
}

pub struct Server {
    pub server: RenetServer,
    pub transport: NetcodeServerTransport,
    pub last_updated: Instant,
    pub users: BTreeMap<u64, UserInfo>,
}

const PROTOCOL_ID: u64 = 7;

fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

impl Server {
    pub fn new() -> Self {
        let addr = "0.0.0.0:8000";
        info!("Hosting server at {}", addr);

        let server_addr: SocketAddr = addr.parse().unwrap();

        let connection_config = ConnectionConfig::default();
        let server: RenetServer = RenetServer::new(connection_config);

        let last_updated = Instant::now();

        let current_time = get_current_time();

        let server_config = ServerConfig {
            current_time,
            max_clients: 64,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![server_addr],
            authentication: ServerAuthentication::Unsecure,
        };

        let socket: UdpSocket = UdpSocket::bind(server_addr).unwrap();

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

        Self {
            server,
            transport,
            last_updated,
            users: BTreeMap::new(),
        }
    }

    pub fn users(&self) -> impl Iterator<Item = (&u64, &UserInfo)> {
        self.users.iter()
    }

    pub fn send_message(&mut self, client_id: u64, msg: ServerMessage) {
        let bytes = bincode::serialize(&msg).unwrap();
        self.server
            .send_message(client_id, DefaultChannel::ReliableOrdered, bytes);
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;
        self.server.update(duration);
        self.transport.update(duration, &mut self.server).unwrap();

        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    self.users.insert(
                        client_id,
                        UserInfo {
                            last_ping_sent: now,
                            last_message_received: now,
                            expected_ping_check: 0,
                            username: None,
                        },
                    );
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    self.users.remove(&client_id);
                    warn!("User {} disconnected {}", client_id, reason);
                }
            }
        }

        for client_id in self.server.clients_id() {
            while let Some(message) = self
                .server
                .receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                let msg: Result<ClientMessage, _> = bincode::deserialize(&message);

                let msg = match msg {
                    Ok(msg) => msg,
                    Err(error) => {
                        info!("Failed to parse: {:?}", error);
                        continue;
                    }
                };

                // update the stamp of last message received
                self.users
                    .entry(client_id)
                    .and_modify(|e| e.last_message_received = now);

                info!("Client {} sent: {:?}", client_id, msg);

                match msg {
                    ClientMessage::Pong(check, stamp) => {
                        let expected = self
                            .users
                            .get(&client_id)
                            .map(|e| e.expected_ping_check)
                            .unwrap_or(0);

                        let now = get_current_time();
                        let latency = now - stamp;

                        info!("Latency: {:?}", latency);

                        if expected != check {
                            error!(
                                "Bad ping checksum from client {}; expected {}, got {}",
                                client_id, expected, check
                            );
                        }
                    }
                    ClientMessage::Text(text) => {
                        info!("Client {} says: {}", client_id, text);
                    }
                    ClientMessage::Introduction { username } => {
                        self.users
                            .entry(client_id)
                            .and_modify(|e| e.username = Some(username));
                    }
                }
            }

            let dur_since_last_msg = self
                .users
                .get(&client_id)
                .map(|e| now - e.last_message_received)
                .unwrap_or(Duration::from_secs(10));

            let dur_since_last_ping = self
                .users
                .get(&client_id)
                .map(|e| now - e.last_ping_sent)
                .unwrap_or(Duration::from_secs(10));

            const PING_TIMEOUT: Duration = Duration::from_secs(3);

            if dur_since_last_msg > PING_TIMEOUT && dur_since_last_ping > PING_TIMEOUT {
                let check = randint(1, 100000) as u64;
                self.users.entry(client_id).and_modify(|e| {
                    e.expected_ping_check = check;
                    e.last_ping_sent = now;
                });
                let current_time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                self.send_message(client_id, ServerMessage::Ping(check, current_time));
            }
        }

        self.transport.send_packets(&mut self.server);
    }
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

pub struct Client {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_updated: Instant,
    has_introduced: bool,
    username: String,
}

impl Client {
    pub fn new(addr: &str, username: &str) -> Self {
        info!("Connecting to server {} with username {}", addr, username);
        let uname = Username(username.to_string());
        let server_addr = addr.parse().unwrap();
        let connection_config = ConnectionConfig::default();
        let client = RenetClient::new(connection_config);

        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        let client_id = current_time.as_millis() as u64;

        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id,
            user_data: Some(uname.to_netcode_user_data()),
            protocol_id: PROTOCOL_ID,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        Self {
            client,
            transport,
            last_updated,
            has_introduced: false,
            username: username.to_string(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;

        self.client.update(duration);
        if let Err(e) = self.transport.update(duration, &mut self.client) {
            error!("Failed to update network transport: {}", e);
        }

        if self.client.is_connected() {
            if !self.has_introduced {
                self.has_introduced = true;
                self.send_message(ClientMessage::Introduction {
                    username: self.username.clone(),
                });
            }

            while let Some(bytes) = self.client.receive_message(DefaultChannel::ReliableOrdered) {
                let msg: ServerMessage = bincode::deserialize(&bytes).unwrap();
                info!("Got from server: {:?}", msg);

                match msg {
                    ServerMessage::Ping(check, stamp) => {
                        let new_stamp = get_current_time();
                        let latency = new_stamp - stamp;
                        info!("Latency: {:?}", latency);
                        self.send_message(ClientMessage::Pong(check, new_stamp));
                    }
                    ServerMessage::ShipPosition(ship_pos) => {
                        // interesting.
                    }
                }
            }
        }

        self.transport.send_packets(&mut self.client).unwrap();
    }

    pub fn send_message(&mut self, msg: ClientMessage) {
        let bytes = bincode::serialize(&msg).unwrap();
        self.client
            .send_message(DefaultChannel::ReliableOrdered, bytes);
    }

    pub fn disconnect(&mut self) {
        self.transport.disconnect();
        self.client.disconnect();
    }
}
