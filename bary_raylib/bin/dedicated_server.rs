use bary_raylib::scenarios::dev_world;
use bary_raylib::world::update_world_logged;
use std::{
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant, SystemTime},
};

use renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

const PROTOCOL_ID: u64 = 7;

struct Server {
    server: RenetServer,
    transport: NetcodeServerTransport,
    last_updated: Instant,
}

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

    fn update(&mut self) {
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
            }
        }

        self.transport.send_packets(&mut self.server);
    }
}

fn main() {
    let mut world = dev_world("assets").unwrap();
    let mut server = Server::new();

    println!("Starting dedicated server...");
    loop {
        server.update();

        std::thread::sleep(Duration::from_millis(20));

        update_world_logged(&mut world, (1080.0, 720.0).into(), None);
    }
}
