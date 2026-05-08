use super::common::*;
use bary_core::prelude::randint;
use log::{error, info};
use renet::*;
use renet_netcode::*;
use std::net::*;
use std::time::Instant;

pub struct Client {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_updated: Instant,
    has_introduced: bool,
    username: String,
}

impl Client {
    pub fn new() -> Self {
        let username = format!("u{}", randint(1, 1000000));

        let client = RenetClient::new(ConnectionConfig::default());

        let client_id = (get_current_time().as_micros() % 1000000000) as u64;

        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let current_time = get_current_time();

        let authentication = ClientAuthentication::Unsecure {
            server_addr: SERVER_ADDR,
            client_id,
            user_data: None,
            protocol_id: 0,
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

    fn reconnect(&mut self) {
        let username = format!("u{}", randint(1, 1000000));

        let client = RenetClient::new(ConnectionConfig::default());

        let client_id = (get_current_time().as_micros() % 1000000000) as u64;
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let current_time = get_current_time();

        let authentication = ClientAuthentication::Unsecure {
            server_addr: SERVER_ADDR,
            client_id,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        *self = Self {
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

    pub fn update(&mut self) -> Vec<ServerMessage> {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;

        self.client.update(duration);

        if let Err(e) = self.transport.update(duration, &mut self.client) {
            error!("Failed to update network transport: {}", e);
            self.reconnect();
            return Vec::new();
        }

        let mut messages = Vec::new();

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

                match &msg {
                    ServerMessage::Ping(check, stamp) => {
                        let new_stamp = get_current_time();
                        let latency = new_stamp - *stamp;
                        info!("Latency: {:?}", latency);
                        self.send_message(ClientMessage::Pong(*check, new_stamp));
                    }
                    ServerMessage::Text(msg) => {
                        info!("Server message: ~ {}", msg);
                    }
                    ServerMessage::Transaction(_tr) => {}
                }

                messages.push(msg);
            }
        }

        self.transport.send_packets(&mut self.client).unwrap();

        messages
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
