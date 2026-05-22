use crate::{Message, MessageKind};
use log::{debug, info};
use renet::*;
use renet_netcode::*;
use std::net::*;
use std::time::{Instant, SystemTime};

pub struct Server {
    renet: RenetServer,
    transport: NetcodeServerTransport,
    last_updated: Instant,
}

impl Server {
    pub fn new(port: u16) -> Self {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
        let socket: UdpSocket = UdpSocket::bind(addr).unwrap();

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        let server_config = ServerConfig {
            current_time,
            max_clients: 64,
            protocol_id: 0,
            public_addresses: vec![
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 5000),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 252)), 5000),
            ],
            authentication: ServerAuthentication::Unsecure,
        };

        let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

        let renet = RenetServer::new(ConnectionConfig::default());

        Self {
            renet,
            transport,
            last_updated: Instant::now(),
        }
    }

    pub fn renet(&self) -> &RenetServer {
        &self.renet
    }

    pub fn broadcast_telemetry(&mut self, kind: MessageKind) {
        let msg = Message::telemetry("server", kind);
        self.broadcast(msg);
    }

    pub fn send_telemetry(&mut self, client_id: u64, kind: MessageKind) {
        let msg = Message::telemetry("server", kind);
        self.send(client_id, msg);
    }

    pub fn broadcast_response(&mut self, kind: MessageKind) {
        let msg = Message::response("server", kind);
        self.broadcast(msg);
    }

    pub fn send_response(&mut self, client_id: u64, kind: MessageKind) {
        let msg = Message::response("server", kind);
        self.send(client_id, msg);
    }

    pub fn broadcast(&mut self, msg: Message) {
        let message = bincode::serialize(&msg).unwrap();
        self.renet
            .broadcast_message(DefaultChannel::ReliableOrdered, message);
    }

    pub fn send(&mut self, client_id: u64, msg: Message) {
        let message = bincode::serialize(&msg).unwrap();
        self.renet
            .send_message(client_id, DefaultChannel::ReliableOrdered, message);
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();

        let now = Instant::now();
        let duration = now - self.last_updated;
        self.renet.update(duration);
        self.transport.update(duration, &mut self.renet).unwrap();

        while let Some(event) = self.renet.get_event() {
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

        for client_id in self.renet.clients_id() {
            while let Some(message) = self
                .renet
                .receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                if let Ok(message) = bincode::deserialize::<Message>(&message) {
                    debug!("Received message from client {}: {:?}", client_id, message);
                    messages.push(message);
                }
            }
        }

        self.transport.send_packets(&mut self.renet);

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
        let mut server = Server::new(7000);
        let mut client = Client::new(127, 0, 0, 1, 7000);

        let dur = Duration::from_millis(10);

        for _ in 0..20 {
            _ = server.update();
            _ = client.update();
            std::thread::sleep(dur);
        }

        assert!(client.is_connected());

        server.broadcast_telemetry(MessageKind::Ack);
        _ = server.update();

        let msgs = client.update();

        let msg = msgs.first().unwrap();

        let is_ack = if let MessageKind::Ack = msg.kind {
            true
        } else {
            false
        };

        assert!(is_ack);
    }
}
