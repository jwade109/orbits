use super::common::*;
use log::{debug, info};
use renet::*;
use renet_netcode::*;
use std::collections::BTreeMap;
use std::net::*;
use std::time::{Duration, Instant, SystemTime};

pub const SYSTEM_MESSAGE_CLIENT_ID: ClientId = 0;
pub const HOST_CLIENT_ID: ClientId = 1;

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
    pub usernames: BTreeMap<ClientId, String>,
    pub messages: Vec<ClientMessage>,
}

impl Server {
    pub fn new(host_username: String) -> Self {
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
        }
    }

    pub fn broadcast(&mut self, msg: ServerMessage) {
        let message = bincode::serialize(&msg).unwrap();
        self.server
            .broadcast_message(DefaultChannel::ReliableOrdered, message);
    }

    pub fn update(&mut self, duration: Duration) -> Result<(), std::io::Error> {
        self.server.update(duration);
        self.transport.update(duration, &mut self.server).unwrap();

        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!("Client connected: {}", client_id);
                    // let user_data = self.transport.user_data(client_id).unwrap();
                    // let username = Username::from_user_data(&user_data).0;
                    // self.usernames.insert(client_id, username.clone());
                }
                ServerEvent::ClientDisconnected {
                    client_id,
                    reason: _,
                } => {
                    info!("Client disconnected: {}", client_id);
                    self.usernames.remove(&client_id);
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
                        let forward = ServerMessage::Transaction(tr);
                        let bytes = bincode::serialize(&forward).unwrap();
                        self.server.broadcast_message_except(
                            client_id,
                            DefaultChannel::ReliableOrdered,
                            bytes,
                        )
                    }
                }
            }
        }

        self.transport.send_packets(&mut self.server);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::time::Duration;

    #[test]
    fn test_connection() {
        let mut server = Server::new("whatever".to_string());
        let mut client = Client::new();

        let dur = Duration::from_millis(10);

        for _ in 0..20 {
            _ = server.update(dur);
            client.update();
            std::thread::sleep(dur);
        }

        assert_eq!(client.is_connected(), true);
    }
}
