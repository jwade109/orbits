use bary_core::prelude::*;
use bary_raylib::sim::World;
use bary_raylib::utils::WallTimer;
use bary_raylib::world_builder::WorldBuilder;
use bary_raylib::*;
use log::{info, warn};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ServerApp {
    world: World,
    incoming_transactions: MessageQueue<ClientMessage>,
    outgoing_transactions: MessageQueue<ServerMessage>,
    _server_thread: JoinHandle<()>,
    world_timer: WallTimer,
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
            .spawn("remora", "jill", Isometry2d::from_pos(randvec(20.0, 40.0)))
            .spawn("remora", "", Isometry2d::from_pos(randvec(20.0, 40.0)))
            .spawn("remora", "bob", Isometry2d::from_pos(randvec(20.0, 40.0)))
            .waypoint("bob", (-900.0, 140.0, 0.0))
            .waypoint("jill", (700.0, -400.0, 0.4))
            .build();

        Self {
            world,
            incoming_transactions: incoming_transactions.clone(),
            outgoing_transactions: outgoing_transactions.clone(),
            _server_thread: std::thread::spawn(|| {
                server_thread(incoming_transactions, outgoing_transactions)
            }),
            world_timer: WallTimer::with_dur(Duration::from_millis(20)),
            world_echo_timer: WallTimer::with_dur(Duration::from_secs(1)),
            sync_timer: WallTimer::with_dur(Duration::from_millis(250)),
        }
    }

    pub fn update(&mut self) {
        while let Some(msg) = self.incoming_transactions.pop() {
            self.on_accept_message(msg);
        }

        if self.world_echo_timer.tick() {
            // info!("Running world: {:?}", self.world);
        }

        if self.world_timer.tick() {
            update_world(&mut self.world);
        }

        if self.sync_timer.tick() {
            self.send_tlm_fast_forward();
            self.send_tlm_grid_pos();
            self.send_tlm_server_info();
        }
    }

    fn send_tlm_fast_forward(&mut self) {
        let action = Action::World(WorldAction::FastForwardTo(self.world.ticks));
        let tr = Transaction::new(self.world.ticks, action);
        self.outgoing_transactions
            .push(ServerMessage::Transaction(tr));
    }

    fn send_tlm_grid_pos(&mut self) {
        for grid in self.world.grids.values() {
            let pos = grid.particle_location;
            self.outgoing_transactions
                .push(ServerMessage::GridPos(grid.name.clone(), pos));
        }
    }

    fn send_tlm_ack(&mut self) {
        self.outgoing_transactions.push(ServerMessage::Ack);
    }

    fn send_tlm_server_info(&mut self) {
        self.outgoing_transactions
            .push(ServerMessage::ServerInfo { connected_users: 0 });
    }

    fn on_accept_message(&mut self, msg: ClientMessage) {
        warn!("Got a command: {:?}", msg);
        self.send_tlm_ack();

        match msg {
            ClientMessage::Transaction(tr) => {
                self.on_accept_transaction(tr);
            }
            _ => (),
        }
    }

    fn on_accept_transaction(&mut self, tr: Transaction) {
        apply_action(&mut self.world, tr.action);
    }
}

fn server_thread(
    incoming_queue: MessageQueue<ClientMessage>,
    outgoing_queue: MessageQueue<ServerMessage>,
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
            server.broadcast(tr);
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
