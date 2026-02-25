use renet::*;
use renet_netcode::*;
use serde::{Deserialize, Serialize};
use std::net::*;
use std::time::{Instant, SystemTime};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClientMessage {
    pub message: String,
}

pub struct Server {
    pub server: RenetServer,
    pub transport: NetcodeServerTransport,
    pub last_updated: Instant,
}

const PROTOCOL_ID: u64 = 7;

impl Server {
    pub fn new() -> Self {
        let server_addr: SocketAddr = "0.0.0.0:8000".parse().unwrap();

        let connection_config = ConnectionConfig::default();
        let server: RenetServer = RenetServer::new(connection_config);

        let last_updated = Instant::now();

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

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
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;
        self.server.update(duration);
        self.transport.update(duration, &mut self.server).unwrap();

        while let Some(event) = self.server.get_event() {
            println!("{:?}", event);
        }

        for client_id in self.server.clients_id() {
            while let Some(message) = self
                .server
                .receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                println!("Client {} sent: {:?}", client_id, message);

                let msg: Result<ClientMessage, _> = bincode::deserialize(&message);

                dbg!(msg);
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
