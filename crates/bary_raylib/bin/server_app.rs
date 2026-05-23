use bary_ipc::*;
use bary_raylib::assets::Assets;
use bary_raylib::persistence::{list_saves_in_dir, load_world, save_world};
use bary_raylib::render::draw_terminal;
use bary_raylib::sim::World;
use bary_raylib::utils::{Application, BasicApp, WallTimer};
use bary_raylib::*;
use bary_terminal::Terminal;
use clap::Parser;
use log::{info, warn};
use raylib::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ServerApp {
    saves_dir: PathBuf,
    save_name: Option<String>,
    world: World,
    server: Arc<RwLock<Server>>,
    incoming_transactions: MessageQueue<Message>,
    outgoing_transactions: MessageQueue<Message>,
    _server_thread: JoinHandle<()>,
    world_timer: WallTimer,
    sync_timer: WallTimer,
}

impl ServerApp {
    pub fn new(saves_dir: impl Into<PathBuf>, save_name: impl Into<String>, port: u16) -> Self {
        let saves_dir = saves_dir.into();
        let save_name = save_name.into();

        let incoming_transactions = new_message_queue();
        let outgoing_transactions = new_message_queue();

        let server = Arc::new(RwLock::new(Server::new(port)));

        Self {
            saves_dir,
            save_name: Some(save_name),
            world: World::empty(),
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
            .push(MessageKind::CurrentTick(self.world.ticks).with_source(MessageSource::Server));
    }

    fn send_tlm_grid_pos(&mut self) {
        for grid in self.world.grids.values() {
            let pos = grid.particle_location;
            self.outgoing_transactions.push(
                MessageKind::GridPos(grid.name.clone(), pos).with_source(MessageSource::Server),
            );
        }
    }

    fn send_tlm_ack(&mut self) {
        self.outgoing_transactions
            .push(MessageKind::Ack.with_source(MessageSource::Server));
    }

    fn send_tlm_server_info(&mut self) {
        self.outgoing_transactions.push(Message::new(
            MessageSource::Server,
            MessageLevel::Telemetry,
            MessageKind::ServerStatistics(self.get_statistics()),
        ));
        self.outgoing_transactions
            .push(MessageKind::Text("Hello there!".to_string()).with_source(MessageSource::Server));
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
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Unsupported,
        ));
    }

    fn on_accept_ping(&mut self) {
        self.outgoing_transactions.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Pong,
        ));
    }

    fn on_accept_text(&mut self, s: String) {
        self.outgoing_transactions.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Text(format!("Here king, you dropped this: \"{s:}\"")),
        ));
    }

    fn on_accept_set_sim_speed(&mut self, speed: u32) {
        self.world.tick_rate = speed;
        self.outgoing_transactions.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Ack,
        ));
    }

    fn on_accept_find_grid_by_name(&mut self, name: String) {
        if let Some(id) = get_grid_by_name(&self.world.grids, &name) {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Entity(id),
            ));
        } else {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Text("No grid with that name.".into()),
            ));
        }
    }

    fn on_accept_list_grids(&mut self) {
        for (id, grid) in self.world.grids.iter() {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::GridInfo(*id, grid.name.clone(), grid.particle_location),
            ));
        }
    }

    fn on_accept_list_protos(&mut self) {
        for (id, proto) in self.world.prototypes.iter() {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Proto(*id, proto.clone()),
            ));
        }
    }

    fn on_accept_list_parts(&mut self) {
        for (id, part) in self.world.parts.iter() {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Part(*id, *part),
            ));
        }
    }

    fn on_accept_list_thrusters(&mut self) {
        for (id, thr) in self.world.thrusters.iter() {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Thruster(*id, thr.clone()),
            ));
        }
    }

    fn on_accept_list_computers(&mut self) {
        for (id, cpu) in self.world.computers.iter() {
            self.outgoing_transactions.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Computer(*id, cpu.clone()),
            ));
        }
    }

    fn on_accept_req_server_info(&mut self) {
        self.outgoing_transactions.push(Message::new(
            MessageSource::Server,
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
    fn new(saves_dir: &str, save_name: &str, port: u16) -> Self {
        let mut app = BasicApp::new("Barycenter Server", TraceLogLevel::LOG_INFO);

        let mut assets = Assets::default();

        assets::load_assets(&mut assets, &mut app.handle, &app.thread);

        Self {
            app,
            terminal: Terminal::with_commands(server_console_commands()),
            server: ServerApp::new(saves_dir, save_name, port),
            assets,
        }
    }

    fn on_terminal_command(&mut self, cmd: Action) {
        match cmd {
            Action::Say(_) => self.terminal.log_info("Woooo!".to_string()),
            Action::Clear => self.terminal.clear(),
            Action::Exit => self.app.exit(),
            Action::EchoSave => {
                self.list_saves();
            }
            Action::LoadSave(name) => {
                self.load_save_file(name);
            }
            Action::SaveWorldToDisk(path, overwrite) => {
                self.save_world_to_disk(path, overwrite);
            }
            Action::ListSaves => {
                self.list_saves();
            }
            Action::SetSimSpeed(speed) => {
                self.server.world.tick_rate = speed;
            }
            _ => self.terminal.log_warn(format!("Unsupported: {:?}", cmd)),
        }
    }

    fn list_saves(&mut self) {
        let saves = list_saves_in_dir(&self.server.saves_dir);
        for s in saves {
            self.terminal.log_info(format!("{}", s.display()));
        }
    }

    fn load_save_file(&mut self, name: String) {
        let path = self.server.saves_dir.join(&name);

        match load_world(&path) {
            Ok(world) => {
                self.server.world = world;
                self.terminal.log_info(format!("Loaded save {name}"));
                self.server.save_name = Some(name);
            }
            Err(e) => {
                self.terminal
                    .log_error(format!("Failed to load world: {e:?}",));
            }
        }
    }

    fn save_world_to_disk(&mut self, save_name: String, overwrite: bool) {
        let path = self.server.saves_dir.join(save_name);
        let res = save_world(&path, &self.server.world, overwrite);
        if let Err(e) = res {
            self.terminal.log_error(format!("Failed: {e:?}"));
        } else {
            self.terminal
                .log_info(format!("Saved to {}", path.display()));
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

        let mut cmds = Vec::new();

        for e in self.app.input.events() {
            if let Some(cmd) = self.terminal.on_event(e) {
                cmds.push(cmd);
            }
        }

        for cmd in cmds {
            self.on_terminal_command(cmd);
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

            let lines = vec![
                "Server Application".to_string(),
                format!("Ticks:     {}", self.server.world.ticks),
                format!("Grids:     {}", self.server.world.grids.len()),
                format!("Parts:     {}", self.server.world.parts.len()),
                format!("Protos:    {}", self.server.world.prototypes.len()),
                format!("BPs:       {}", self.server.world.blueprints.len()),
                format!("Thrusters: {}", self.server.world.thrusters.len()),
                format!("Invs:      {}", self.server.world.inventories.len()),
                format!("Asteroids: {}", self.server.world.asteroids.len()),
                format!("Clients:   {}", self.server.get_statistics().clients.len()),
            ];

            let text = lines.join("\n");

            d.draw_text_ex(
                self.assets.consolas.as_ref().unwrap(),
                &text,
                Vector2::new(10.0, 10.0),
                16.0,
                0.0,
                Color::ORANGE,
            );
        });
    }

    fn should_exit(&self) -> bool {
        !self.app.should_loop()
    }
}

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    server_port: u16,
    saves_dir: String,
    save_name: String,
}

fn main() {
    let args = Args::parse();

    info!("Starting dedicated server...");

    DedicatedServerApp::new(&args.saves_dir, &args.save_name, args.server_port).spin_forever();
}
