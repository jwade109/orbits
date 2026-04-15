use bary_core::prelude::*;
use bary_raylib::multiplayer::*;
use bary_raylib::sim::World;
use bary_raylib::wall_timer::WallTimer;
use bary_raylib::world_builder::WorldBuilder;
use log::{info, warn};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ServerApp {
    world: World,
    runner: WorldRunner,
    incoming_transactions: MessageQueue<Transaction>,
    outgoing_transactions: MessageQueue<Transaction>,
    _server_thread: JoinHandle<()>,
    world_echo_timer: WallTimer,
    sync_timer: WallTimer,
}

impl ServerApp {
    pub fn new() -> Self {
        let incoming_transactions = new_message_queue();
        let outgoing_transactions = new_message_queue();

        let world = WorldBuilder::new()
            .assets()
            .blueprint("pollux")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .spawn("pollux", "", Isometry2d::ZERO)
            .spawn("remora", "", Isometry2d::from_pos(randvec(20.0, 40.0)))
            .spawn("remora", "", Isometry2d::from_pos(randvec(20.0, 40.0)))
            .spawn("remora", "", Isometry2d::from_pos(randvec(20.0, 40.0)))
            .build();

        Self {
            world,
            runner: WorldRunner::new(),
            incoming_transactions: incoming_transactions.clone(),
            outgoing_transactions: outgoing_transactions.clone(),
            _server_thread: std::thread::spawn(|| {
                server_thread(incoming_transactions, outgoing_transactions)
            }),
            world_echo_timer: WallTimer::with_dur(Duration::from_secs(3)),
            sync_timer: WallTimer::with_dur(Duration::from_secs(5)),
        }
    }

    pub fn update(&mut self) {
        while let Some(tr) = self.incoming_transactions.pop() {
            warn!("Got a transaction! {:?}", tr);
        }

        if self.world_echo_timer.tick() {
            info!("Running world: {:?}", self.world);
        }

        self.runner.update_headless(&mut self.world);

        if self.sync_timer.tick() {
            let tr = Transaction::new(self.world.ticks, Action::FastForwardTo(self.world.ticks));
            self.outgoing_transactions.push(tr);
            warn!("Sending sync packet");
        }
    }
}

fn server_thread(
    incoming_queue: MessageQueue<Transaction>,
    outgoing_queue: MessageQueue<Transaction>,
) {
    let mut server = Server::new();

    let mut update_timer = WallTimer::with_dur(Duration::from_millis(50));
    let mut echo_timer = WallTimer::with_dur(Duration::from_millis(3000));

    loop {
        if update_timer.tick() {
            for msg in server.update() {
                incoming_queue.push(msg);
            }
        }

        if echo_timer.tick() {
            info!("{} users connected", server.server.clients_id().len());
        }

        while let Some(tr) = outgoing_queue.pop() {
            server.broadcast(ServerMessage::Transaction(tr));
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

    let mut app = ServerApp::new();

    loop {
        app.update();
    }
}
