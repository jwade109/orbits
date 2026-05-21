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
    incoming_transactions: MessageQueue<Message>,
    outgoing_transactions: MessageQueue<Message>,
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
            for _ in 0..self.world.tick_rate {
                update_world(&mut self.world);
            }
        }

        if self.sync_timer.tick() {
            self.send_tlm_current_tick();
            self.send_tlm_grid_pos();
            self.send_tlm_server_info();
        }
    }

    fn send_tlm_current_tick(&mut self) {
        self.outgoing_transactions
            .push(MessageKind::CurrentTick(self.world.ticks).with_source("server"));
    }

    fn send_tlm_grid_pos(&mut self) {
        for grid in self.world.grids.values() {
            let pos = grid.particle_location;
            self.outgoing_transactions
                .push(MessageKind::GridPos(grid.name.clone(), pos).with_source("server"));
        }
    }

    fn send_tlm_ack(&mut self) {
        self.outgoing_transactions
            .push(MessageKind::Ack.with_source("server"));
    }

    fn send_tlm_server_info(&mut self) {
        self.outgoing_transactions
            .push(MessageKind::ServerInfo { connected_users: 0 }.with_source("server"));
        self.outgoing_transactions
            .push(MessageKind::Text("Hello there!".to_string()).with_source("server"));
    }

    fn on_accept_message(&mut self, msg: Message) {
        warn!("Got a command: {:?}", msg);
        self.send_tlm_ack();

        if msg.level != MessageLevel::Command {
            return;
        }

        match msg.kind {
            MessageKind::Ping => {
                self.on_accept_ping();
            }
            MessageKind::Text(s) => {
                self.on_accept_text(s);
            }
            MessageKind::SetSimSpeed(s) => {
                self.on_accept_set_sim_speed(s);
            }
            MessageKind::FindGridByName(name) => {
                self.on_accept_find_grid_by_name(name);
            }
            MessageKind::ListGrids => {
                self.on_accept_list_grids();
            }
            MessageKind::ListProtos => {
                self.on_accept_list_protos();
            }
            MessageKind::ListParts => {
                self.on_accept_list_parts();
            }
            MessageKind::ListThrusters => {
                self.on_accept_list_thrusters();
            }
            MessageKind::ListComputers => {
                self.on_accept_list_computers();
            }
            _ => self.on_unsupported_message(),
        }
    }

    fn on_unsupported_message(&mut self) {
        warn!("Unsupported message type!");
        self.outgoing_transactions.push(Message::new(
            "server",
            MessageLevel::Response,
            MessageKind::Unsupported,
        ));
    }

    fn on_accept_ping(&mut self) {
        self.outgoing_transactions.push(Message::new(
            "server",
            MessageLevel::Response,
            MessageKind::Pong,
        ));
    }

    fn on_accept_text(&mut self, s: String) {
        self.outgoing_transactions.push(Message::new(
            "server",
            MessageLevel::Response,
            MessageKind::Text(format!("Here king, you dropped this: \"{s:}\"")),
        ));
    }

    fn on_accept_set_sim_speed(&mut self, speed: u32) {
        self.world.tick_rate = speed;
        self.outgoing_transactions.push(Message::new(
            "server",
            MessageLevel::Response,
            MessageKind::Ack,
        ));
    }

    fn on_accept_find_grid_by_name(&mut self, name: String) {
        if let Some(id) = get_grid_by_name(&self.world.grids, &name) {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::Entity(id),
            ));
        } else {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::Text("No grid with that name.".into()),
            ));
        }
    }

    fn on_accept_list_grids(&mut self) {
        for (id, grid) in self.world.grids.iter() {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::GridInfo(*id, grid.name.clone(), grid.particle_location),
            ));
        }
    }

    fn on_accept_list_protos(&mut self) {
        for (id, proto) in self.world.prototypes.iter() {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::Proto(*id, proto.clone()),
            ));
        }
    }

    fn on_accept_list_parts(&mut self) {
        for (id, part) in self.world.parts.iter() {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::Part(*id, *part),
            ));
        }
    }

    fn on_accept_list_thrusters(&mut self) {
        for (id, thr) in self.world.thrusters.iter() {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::Thruster(*id, thr.clone()),
            ));
        }
    }

    fn on_accept_list_computers(&mut self) {
        for (id, cpu) in self.world.computers.iter() {
            self.outgoing_transactions.push(Message::new(
                "server",
                MessageLevel::Response,
                MessageKind::Computer(*id, cpu.clone()),
            ));
        }
    }
}

fn server_thread(incoming_queue: MessageQueue<Message>, outgoing_queue: MessageQueue<Message>) {
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
            info!("{} users connected", server.renet().clients_id().len());
        }

        while let Some(sm) = outgoing_queue.pop() {
            server.broadcast(sm);
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
