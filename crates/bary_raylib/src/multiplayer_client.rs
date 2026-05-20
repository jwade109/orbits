use super::common::*;
use bary_core::prelude::randint;
use log::{error, info};
use renet::*;
use renet_netcode::*;
use std::collections::VecDeque;
use std::net::*;
use std::time::Instant;

const CLIENT_BIND_ADDRESS: &'static str = "127.0.0.1:0";

#[derive(Clone, Copy, Debug)]
pub struct ClientStatistics {
    pub is_connected: bool,
    pub rtt: f64,
    pub packet_loss: f64,
    pub bytes_sent_per_second: f64,
    pub bytes_received_per_second: f64,
    pub rx_count: usize,
    pub tx_count: usize,
}

pub struct Client {
    server_addr: SocketAddr,
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_updated: Instant,
    has_introduced: bool,
    username: String,
    rx_count: usize,
    tx_count: usize,
    message_history: VecDeque<(usize, ServerMessage)>,
}

impl Client {
    pub fn new(server_addr: SocketAddr) -> Self {
        let username = format!("u{}", randint(1, 1000000));

        let client = RenetClient::new(ConnectionConfig::default());

        let client_id = (get_current_time().as_micros() % 1000000000) as u64;

        let socket = UdpSocket::bind(CLIENT_BIND_ADDRESS).unwrap();
        let current_time = get_current_time();

        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        Self {
            server_addr,
            client,
            transport,
            last_updated,
            has_introduced: false,
            username: username.to_string(),
            tx_count: 0,
            rx_count: 0,
            message_history: VecDeque::new(),
        }
    }

    fn reconnect(&mut self) {
        let username = format!("u{}", randint(1, 1000000));

        let client = RenetClient::new(ConnectionConfig::default());

        let client_id = (get_current_time().as_micros() % 1000000000) as u64;
        let socket = UdpSocket::bind(CLIENT_BIND_ADDRESS).unwrap();
        let current_time = get_current_time();

        let authentication = ClientAuthentication::Unsecure {
            server_addr: self.server_addr,
            client_id,
            user_data: None,
            protocol_id: 0,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        *self = Self {
            server_addr: self.server_addr,
            client,
            transport,
            last_updated,
            has_introduced: false,
            username: username.to_string(),
            tx_count: 0,
            rx_count: 0,
            message_history: VecDeque::new(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    pub fn stats(&self) -> ClientStatistics {
        let is_connected = self.client.is_connected();
        let info = self.client.network_info();

        ClientStatistics {
            is_connected,
            rtt: info.rtt,
            packet_loss: info.packet_loss,
            bytes_sent_per_second: info.bytes_sent_per_second,
            bytes_received_per_second: info.bytes_received_per_second,
            tx_count: self.tx_count,
            rx_count: self.rx_count,
        }
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

                self.message_history.push_back((self.rx_count, msg.clone()));
                if self.message_history.len() > 20 {
                    self.message_history.pop_front();
                }

                self.rx_count += 1;

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
                    _ => (),
                }

                messages.push(msg);
            }
        }

        if let Err(e) = self.transport.send_packets(&mut self.client) {
            error!("Send packets failed: {e:?}");
        }

        messages
    }

    pub fn history(&self) -> impl Iterator<Item = &(usize, ServerMessage)> {
        self.message_history.iter().rev()
    }

    pub fn send_message(&mut self, msg: ClientMessage) {
        let bytes = bincode::serialize(&msg).unwrap();
        self.client
            .send_message(DefaultChannel::ReliableOrdered, bytes);
        self.tx_count += 1;
    }

    pub fn disconnect(&mut self) {
        self.transport.disconnect();
        self.client.disconnect();
    }
}
