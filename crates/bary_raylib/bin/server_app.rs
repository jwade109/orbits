use bary_core::prelude::*;
use bary_factory::*;
use bary_raylib::assets::Assets;
use bary_raylib::render::draw_terminal;
use bary_raylib::sim::World;
use bary_raylib::utils::{Application, BasicApp, WallTimer};
use bary_raylib::world_builder::WorldBuilder;
use bary_raylib::*;
use bary_terminal::Terminal;
use log::{info, warn};
use raylib::prelude::*;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ServerApp {
    world: World,
    server: Arc<RwLock<Server>>,
    incoming_transactions: MessageQueue<Message>,
    outgoing_transactions: MessageQueue<Message>,
    _server_thread: JoinHandle<()>,
    world_timer: WallTimer,
    sync_timer: WallTimer,
}

impl ServerApp {
    pub fn new() -> Self {
        let incoming_transactions = new_message_queue();
        let outgoing_transactions = new_message_queue();

        let world = WorldBuilder::new()
            .assets()
            .blueprint(("pollux", 0))
            .blueprint(("pollux", 2))
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .blueprint("foundation")
            .blueprint("miner")
            .blueprint("icecream")
            .spawn(("pollux", 0), "", (30.0, 0.0, 0.0))
            .spawn(("pollux", 2), "", (0.0, 0.0, 0.0))
            .insert_source((19, 7), Item::Magnesium)
            .insert_source((20, 7), Item::Iron)
            .insert_source((21, 7), Item::Titanium)
            .set_recipe((21, 7), RecipeListing::TitaniumLattice)
            .insert_source((22, 11), Item::Water)
            .insert_pipe((22, 11), (22, 10))
            .set_recipe((27, 7), RecipeListing::WaterElectrolysis)
            .insert_pipe((17, 10), (15, 10))
            .insert_pipe((27, 7), (34, 7))
            .insert_pipe((26, 7), (25, 6))
            .spawn("remora", "", (10.0, 30.0, 0.1))
            .spawn("miner", "", (-9.0, 12.0, -0.3))
            .spawn("remora", "", (-7.0, 23.0, 0.7))
            .spawn("bellerophon", "", (130.0, 50.0, 0.1))
            .command(WorldAction::SetSpeed(10))
            .command(WorldAction::Ping(Vec2::ZERO))
            .command(WorldAction::Ping(Vec2::splat(10.0)))
            .asteroid((-80.0, 30.0, 0.1), 20.0, 391)
            .asteroid((60.0, 300.0, 0.7), 50.0, 2384)
            .asteroid((400.0, -2000.0, 0.7), 500.0, 9312)
            .build();

        let server = Arc::new(RwLock::new(Server::new(5000)));

        Self {
            world,
            server: server.clone(),
            incoming_transactions: incoming_transactions.clone(),
            outgoing_transactions: outgoing_transactions.clone(),
            _server_thread: std::thread::spawn(|| {
                server_thread(server, incoming_transactions, outgoing_transactions)
            }),
            world_timer: WallTimer::with_dur(Duration::from_millis(20)),
            sync_timer: WallTimer::with_dur(Duration::from_millis(250)),
        }
    }

    pub fn get_statistics(&self) -> ServerStatistics {
        if let Ok(server) = self.server.read() {
            let mut clients = Vec::new();
            for client in server.renet().clients_id_iter() {
                if let Ok(info) = server.renet().network_info(client) {
                    clients.push((client, info.into()));
                }
            }
            ServerStatistics { clients }
        } else {
            ServerStatistics::default()
        }
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();
        while let Some(msg) = self.incoming_transactions.pop() {
            self.on_accept_message(msg.clone());
            messages.push(msg);
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

        messages
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
        self.outgoing_transactions.push(Message::new(
            "server",
            MessageLevel::Telemetry,
            MessageKind::ServerStatistics(self.get_statistics()),
        ));
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
            MessageKind::RequestServerStatistics => {
                self.on_accept_req_server_info();
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

    fn on_accept_req_server_info(&mut self) {
        self.outgoing_transactions.push(Message::new(
            "server",
            MessageLevel::Response,
            MessageKind::ServerStatistics(self.get_statistics()),
        ));
    }
}

fn server_thread(
    server: Arc<RwLock<Server>>,
    incoming_queue: MessageQueue<Message>,
    outgoing_queue: MessageQueue<Message>,
) {
    let mut update_timer = WallTimer::with_dur(Duration::from_millis(50));
    let mut echo_timer = WallTimer::with_dur(Duration::from_millis(3000));

    loop {
        if let Ok(mut server) = server.write() {
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
}

struct DedicatedServerApp {
    app: BasicApp,
    terminal: Terminal<Action>,
    server: ServerApp,
    assets: Assets,
}

impl DedicatedServerApp {
    fn new() -> Self {
        let mut app = BasicApp::new("Barycenter Server", TraceLogLevel::LOG_INFO);

        let mut assets = Assets::default();

        assets::load_assets(&mut assets, &mut app.handle, &app.thread);

        Self {
            app,
            terminal: Terminal::with_commands(all_commands()),
            server: ServerApp::new(),
            assets,
        }
    }
}

impl Application for DedicatedServerApp {
    fn update(&mut self) {
        self.app.frame();

        for msg in self.server.update() {
            let s = format!("{:?}", msg);
            self.terminal.log_debug(s);
        }

        for e in self.app.input.events() {
            self.terminal.on_event(e);
        }

        self.terminal.focus();
    }

    fn draw(&mut self) {
        self.app.handle.draw(&self.app.thread, |mut d| {
            d.clear_background(Color::new(140, 140, 30, 255));
            d.draw_rectangle(
                3,
                3,
                d.get_render_width() - 6,
                d.get_render_height() - 6,
                Color::ORANGE,
            );
            draw_terminal(&mut d, &self.terminal, &self.assets);
            d.draw_text("Server Application", 10, 10, 24, Color::ORANGE);
        });
    }

    fn should_exit(&self) -> bool {
        !self.app.should_loop()
    }
}

fn main() {
    info!("Starting dedicated server...");
    let app = DedicatedServerApp::new();
    app.spin_forever();
}
