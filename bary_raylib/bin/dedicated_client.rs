use bary_raylib::multiplayer::*;
use bary_raylib::scenarios::dev_world;
use bary_raylib::world::update_world_logged;
use std::{
    net::{SocketAddr, UdpSocket},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant, SystemTime},
};

use renet::{ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent};
use renet_netcode::{
    ClientAuthentication, NETCODE_USER_DATA_BYTES, NetcodeClientTransport, NetcodeServerTransport,
    ServerAuthentication, ServerConfig,
};

const PROTOCOL_ID: u64 = 7;

fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        loop {
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer).unwrap();
            tx.send(buffer.trim_end().to_string()).unwrap();
        }
    });
    rx
}

struct Client {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_updated: Instant,
}

impl Client {
    pub fn new() -> Self {
        let username = Username("Bob".to_string());
        let server_addr = "127.0.0.1:8000".parse().unwrap();
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
            user_data: Some(username.to_netcode_user_data()),
            protocol_id: PROTOCOL_ID,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

        let last_updated = Instant::now();

        Self {
            client,
            transport,
            last_updated,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let duration = now - self.last_updated;
        self.last_updated = now;

        self.client.update(duration);
        self.transport.update(duration, &mut self.client).unwrap();

        if self.client.is_connected() {
            while let Some(text) = self.client.receive_message(DefaultChannel::ReliableOrdered) {
                let text = String::from_utf8(text.into()).unwrap();
                println!("{}", text);
            }
        }

        self.transport.send_packets(&mut self.client).unwrap();
    }

    pub fn send_message(&mut self, msg: Vec<u8>) {
        self.client
            .send_message(DefaultChannel::ReliableOrdered, msg);
    }
}

fn main() {
    let mut client = Client::new();

    let stdin_channel: Receiver<String> = spawn_stdin_channel();

    loop {
        client.update();

        match stdin_channel.try_recv() {
            Ok(text) => {
                let msg = ClientMessage { message: text };
                let msg = bincode::serialize(&msg).unwrap();
                client.send_message(msg);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        }
    }
}
