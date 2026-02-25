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
    let mut server = Server::new();
    let mut last_update = Instant::now();

    loop {
        server.update();

        let now = Instant::now();

        if now - last_update < Duration::from_secs(5) {
            continue;
        }

        info!("{} users connected", server.users.len());

        last_update = now;
    }
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .unwrap();

    println!("Starting dedicated server...");
    let _world_thread = std::thread::spawn(world_thread);
    let _server_thread = std::thread::spawn(server_thread);

    loop {}
}
