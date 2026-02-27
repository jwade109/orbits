use bary_core::prelude::{chance, randint, randvec};
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

        if now - last_log < Duration::from_millis(500) {
            continue;
        }

        for id in server.server.clients_id() {
            if let Ok(info) = server.server.network_info(id) {
                println!(
                    "{} {} {} {}",
                    info.bytes_received_per_second,
                    info.bytes_sent_per_second,
                    info.packet_loss,
                    info.rtt
                );
            }
        }

        // server.broadcast(ServerMessage::Ping(
        //     randint(1, 1000000) as u64,
        //     get_current_time(),
        // ));

        let p = randvec(1.0, 100.0);

        // server.broadcast(ServerMessage::Transaction(Transaction::Ping(p)));

        if chance(0.05) {
            server.broadcast(ServerMessage::Transaction(Transaction::SpawnShip(
                "remora".to_string(),
            )));
        }

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
