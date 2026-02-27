use bary_core::prelude::{chance, randvec};
use bary_raylib::multiplayer::*;
use bary_raylib::scenarios::dev_world;
use bary_raylib::wall_timer::WallTimer;
use bary_raylib::world::update_world;
use log::info;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ServerApp {
    _server_thread: JoinHandle<()>,
    _world_thread: JoinHandle<()>,
}

impl ServerApp {
    pub fn new() -> Self {
        Self {
            _server_thread: std::thread::spawn(server_thread),
            _world_thread: std::thread::spawn(world_thread),
        }
    }
}

fn world_thread() {
    let mut world = dev_world("assets").unwrap();

    const WORLD_TICKS_PER_SECOND: u64 = 50;
    const MILLISECONDS_PER_TICK: u64 = 1000 / WORLD_TICKS_PER_SECOND;

    let mut update_timer = WallTimer::with_dur(Duration::from_millis(MILLISECONDS_PER_TICK));
    let mut echo_timer = WallTimer::with_dur(Duration::from_secs(1));

    loop {
        if echo_timer.tick() {
            info!("Running world: {:?}", world);
        }

        if update_timer.tick() {
            update_world(&mut world, (1080.0, 720.0).into(), None);
        }
    }
}

fn server_thread() {
    let mut server = Server::new("idgaf".to_string());

    let mut update_timer = WallTimer::with_dur(Duration::from_millis(50));
    let mut echo_timer = WallTimer::with_dur(Duration::from_millis(1000));

    loop {
        if update_timer.tick() {
            server.update();
        }

        if echo_timer.tick() {
            info!("{} users connected", server.server.clients_id().len());
            // if chance(0.2) {
            //     let p = randvec(10.0, 150.0);
            //     let action = Action::SpawnShipAt("remora".to_string(), p);
            //     server.broadcast(ServerMessage::transaction(0, action));
            // }
        }
    }
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .unwrap();

    println!("Starting dedicated server...");

    let _app = ServerApp::new();

    loop {}
}
