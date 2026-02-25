use super::common::*;
use bary_core::prelude::*;
use log::{error, info, warn};
use renet::*;
use renet_netcode::*;
use std::collections::BTreeMap;
use std::net::*;
use std::time::{Duration, Instant, SystemTime};

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

impl Server {
    pub fn new() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

        let connection_config = ConnectionConfig::default();
        let server: RenetServer = RenetServer::new(connection_config);

        let last_updated = Instant::now();

        let current_time = get_current_time();

        let server_config = ServerConfig {
            current_time,
            max_clients: 64,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![socket.local_addr().unwrap()],
            authentication: ServerAuthentication::Unsecure,
        };

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

        dbg!(transport.addresses());

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
                    info!("User connected: {}", client_id);
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
