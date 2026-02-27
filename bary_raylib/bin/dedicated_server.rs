use bary_core::prelude::randint;
use bary_raylib::multiplayer::*;
use bary_raylib::scenarios::dev_world;
use bary_raylib::world::update_world;
use log::info;
use std::time::{Duration, Instant};

fn world_thread() {
    let mut world = dev_world("assets").unwrap();
    loop {
        if world.ticks % 120 == 0 {
            info!(
                "Running world -- tick {}, {:?}",
                world.ticks, world.timers.update
            );
        }
        update_world(&mut world, (1080.0, 720.0).into(), None);

        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn server_thread() {
    let mut server = Server::new("idgaf".to_string());
    let mut last_update = Instant::now();
    let mut last_log = Instant::now();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        let now = Instant::now();
        let dt = now - last_update;

        _ = server.update(dt);

        last_update = now;

        if now - last_log < Duration::from_secs(5) {
            continue;
        }

        dbg!(server.transport.addresses());

        server.broadcast(ServerMessage::Ping(
            randint(1, 1000000) as u64,
            get_current_time(),
        ));

        info!("{} users connected", server.server.clients_id().len());

        last_log = now;
    }
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .unwrap();

    println!("Starting dedicated server...");
    let _world_thread = std::thread::spawn(world_thread);
    let _server_thread = std::thread::spawn(server_thread);

    loop {}
}
