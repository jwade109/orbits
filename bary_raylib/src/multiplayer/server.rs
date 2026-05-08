use super::common::*;
use super::transactions::Transaction;
use log::{debug, info};
use renet::*;
use renet_netcode::*;
use std::collections::BTreeMap;
use std::net::*;
use std::time::{Instant, SystemTime};

pub const SYSTEM_MESSAGE_CLIENT_ID: ClientId = 0;
pub const HOST_CLIENT_ID: ClientId = 1;

#[derive(Debug)]
pub struct UserInfo {
    pub last_ping_sent: Instant,
    pub last_message_received: Instant,
    pub expected_ping_check: u64,
}

pub struct Server {
    pub server: RenetServer,
    pub transport: NetcodeServerTransport,
    pub usernames: BTreeMap<ClientId, String>,
    pub messages: Vec<ClientMessage>,
    pub last_updated: Instant,
}

impl Server {
    pub fn new() -> Self {
        let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR).unwrap();
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let server_config = ServerConfig {
            current_time,
            max_clients: 64,
            protocol_id: 0,
            public_addresses: vec![SERVER_ADDR],
            authentication: ServerAuthentication::Unsecure,
        };

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

        let server: RenetServer = RenetServer::new(ConnectionConfig::default());

        let usernames = BTreeMap::new();

        Self {
            server,
            transport,
            usernames,
            messages: vec![],
            last_updated: Instant::now(),
        }
    }

    pub fn broadcast(&mut self, msg: ServerMessage) {
        let message = bincode::serialize(&msg).unwrap();
        self.server
            .broadcast_message(DefaultChannel::ReliableOrdered, message);
    }

    pub fn update(&mut self) -> Vec<Transaction> {
        let mut messages = Vec::new();

        let now = Instant::now();
        let duration = now - self.last_updated;
        self.server.update(duration);
        self.transport.update(duration, &mut self.server).unwrap();

        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!("Client connected: {}", client_id);
                }
                ServerEvent::ClientDisconnected {
                    client_id,
                    reason: _,
                } => {
                    info!("Client disconnected: {}", client_id);
                }
            }
        }

        for client_id in self.server.clients_id() {
            while let Some(message) = self
                .server
                .receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                if let Ok(message) = bincode::deserialize::<ClientMessage>(&message) {
                    debug!("Received message from client {}: {:?}", client_id, message);
                    if let ClientMessage::Transaction(tr) = message {
                        let forward = ServerMessage::Transaction(tr.clone());
                        let bytes = bincode::serialize(&forward).unwrap();
                        self.server.broadcast_message_except(
                            client_id,
                            DefaultChannel::ReliableOrdered,
                            bytes,
                        );
                        messages.push(tr);
                    }
                }
            }
        }

        self.transport.send_packets(&mut self.server);

        self.last_updated = now;

        messages
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::time::Duration;

    #[test]
    fn test_connection() {
        let mut server = Server::new();
        let mut client = Client::new();

        let dur = Duration::from_millis(10);

        for _ in 0..20 {
            _ = server.update();
            client.update();
            std::thread::sleep(dur);
        }

        assert_eq!(client.is_connected(), true);
    }
}
