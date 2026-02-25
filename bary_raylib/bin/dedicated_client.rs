use bary_raylib::multiplayer::*;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

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

fn send_stdin_channel_to_server(channel: &Receiver<String>, client: &mut Client) {
    match channel.try_recv() {
        Ok(text) => {
            if text == "exit" {
                client.disconnect();
                return;
            }
            let msg = ClientMessage::Text(text);
            client.send_message(msg);
        }
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
    }
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .unwrap();

    let args: Vec<String> = std::env::args().collect();

    let username = &args[1];
    let addr = &args[2];

    let mut client = Client::new(addr, username);

    let stdin_channel: Receiver<String> = spawn_stdin_channel();

    loop {
        client.update();
        send_stdin_channel_to_server(&stdin_channel, &mut client);
    }
}
