use super::common::*;
use log::{error, info};
use renet::*;
use renet_netcode::*;
use std::net::*;
use std::time::{Instant, SystemTime};

fn create_renet_client(
    username: String,
    server_addr: SocketAddr,
) -> (RenetClient, NetcodeClientTransport) {
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
        user_data: Some(Username(username).to_netcode_user_data()),
        protocol_id: PROTOCOL_ID,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

    (client, transport)
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

        let (client, transport) = create_renet_client(username.to_string(), addr.parse().unwrap());

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
