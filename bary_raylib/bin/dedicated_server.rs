use bary_raylib::multiplayer::*;
use bary_raylib::scenarios::dev_world;
use bary_raylib::world::update_world_logged;

fn main() {
    let mut world = dev_world("assets").unwrap();
    let mut server = Server::new();

    println!("Starting dedicated server...");
    loop {
        server.update();

        std::thread::sleep(std::time::Duration::from_millis(20));

        update_world_logged(&mut world, (1080.0, 720.0).into(), None);
    }
}
