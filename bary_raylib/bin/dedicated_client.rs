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

// Helper struct to pass an username in the user data
struct Username(String);

impl Username {
    fn to_netcode_user_data(&self) -> [u8; NETCODE_USER_DATA_BYTES] {
        let mut user_data = [0u8; NETCODE_USER_DATA_BYTES];
        if self.0.len() > NETCODE_USER_DATA_BYTES - 8 {
            panic!("Username is too big");
        }
        user_data[0..8].copy_from_slice(&(self.0.len() as u64).to_le_bytes());
        user_data[8..self.0.len() + 8].copy_from_slice(self.0.as_bytes());

        user_data
    }

    fn from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> Self {
        let mut buffer = [0u8; 8];
        buffer.copy_from_slice(&user_data[0..8]);
        let mut len = u64::from_le_bytes(buffer) as usize;
        len = len.min(NETCODE_USER_DATA_BYTES - 8);
        let data = user_data[8..len + 8].to_vec();
        let username = String::from_utf8(data).unwrap();
        Self(username)
    }
}

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

fn client(server_addr: SocketAddr, username: Username) {
    let connection_config = ConnectionConfig::default();
    let mut client = RenetClient::new(connection_config);

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

    let mut transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
    let stdin_channel: Receiver<String> = spawn_stdin_channel();

    let mut last_updated = Instant::now();
    loop {
        let now = Instant::now();
        let duration = now - last_updated;
        last_updated = now;

        client.update(duration);
        transport.update(duration, &mut client).unwrap();

        if client.is_connected() {
            match stdin_channel.try_recv() {
                Ok(text) => {
                    client.send_message(DefaultChannel::ReliableOrdered, text.as_bytes().to_vec())
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
            }

            while let Some(text) = client.receive_message(DefaultChannel::ReliableOrdered) {
                let text = String::from_utf8(text.into()).unwrap();
                println!("{}", text);
            }
        }

        transport.send_packets(&mut client).unwrap();
        thread::sleep(Duration::from_millis(50));
    }
}

fn main() {
    let addr = "127.0.0.1:8000".parse().unwrap();
    client(addr, Username("Bob".to_string()));
}
