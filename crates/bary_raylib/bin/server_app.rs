use bary_core::prelude::{Components, TableIdent, distance_str_v};
use bary_ipc::*;
use bary_raylib::assets::Assets;
use bary_raylib::persistence::{list_saves_in_dir, load_world, save_world};
use bary_raylib::render::draw_terminal;
use bary_raylib::sim::World;
use bary_raylib::utils::{Application, BasicApp, WallTimer};
use bary_raylib::world_builder::WorldBuilder;
use bary_raylib::*;
use bary_terminal::Terminal;
use clap::Parser;
use log::{info, warn};
use raylib::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct ServerApp {
    saves_dir: PathBuf,
    save_name: Option<String>,
    world: World,
    server: Arc<RwLock<ServerNode>>,
    incoming_transactions: MessageQueue<Message>,
    broadcast: MessageQueue<Message>,
    outgoing: MessageQueue<(ClientId, Message)>,
    _server_thread: JoinHandle<()>,
    world_timer: WallTimer,
    sync_timer: WallTimer,
}

impl ServerApp {
    pub fn new(saves_dir: impl Into<PathBuf>, save_name: impl Into<String>, port: u16) -> Self {
        let saves_dir = saves_dir.into();
        let save_name = save_name.into();

        let incoming_transactions = new_message_queue();
        let broadcast = new_message_queue();
        let outgoing = new_message_queue();

        let server = Arc::new(RwLock::new(ServerNode::new(port)));

        let world = WorldBuilder::new()
            .assets()
            .blueprint(("pollux", 0))
            .blueprint(("pollux", 2))
            .blueprint("foundation")
            .blueprint("bellerophon")
            .blueprint("remora")
            .blueprint("spacestation")
            .blueprint("icecream")
            .blueprint("lander")
            .blueprint("manta")
            .build();

        Self {
            saves_dir,
            save_name: Some(save_name),
            world,
            server: server.clone(),
            incoming_transactions: incoming_transactions.clone(),
            broadcast: broadcast.clone(),
            outgoing: outgoing.clone(),
            _server_thread: std::thread::spawn(|| {
                server_thread(server, incoming_transactions, broadcast, outgoing)
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
        self.broadcast
            .push(MessageKind::CurrentTick(self.world.ticks).with_source(MessageSource::Server));
    }

    fn send_tlm_grid_pos(&mut self) {
        for grid in self.world.grids.values() {
            let pos = grid.particle_location;
            self.broadcast.push(
                MessageKind::GridPos(grid.name.clone(), pos).with_source(MessageSource::Server),
            );
        }
    }

    fn send_tlm_ack(&mut self) {
        self.broadcast
            .push(MessageKind::Ack.with_source(MessageSource::Server));
    }

    fn send_tlm_server_info(&mut self) {
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Telemetry,
            MessageKind::ServerStatistics(self.get_statistics()),
        ));
        self.broadcast
            .push(MessageKind::Text("Hello there!".to_string()).with_source(MessageSource::Server));
    }

    fn on_accept_message(&mut self, msg: Message) {
        warn!("Got a command: {:?}", msg);
        self.send_tlm_ack();

        if msg.level != MessageLevel::Command {
            return;
        }

        let MessageSource::Client(client_id) = msg.source else {
            warn!("Got a message with unexpected source: {:?}", msg.source);
            return;
        };

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
                self.on_accept_list_prototypes();
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
            MessageKind::ClientBlobRequest(table) => {
                self.on_accept_client_blob_request(client_id, table);
            }
            _ => self.on_unsupported_message(),
        }
    }

    fn on_unsupported_message(&mut self) {
        warn!("Unsupported message type!");
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Unsupported,
        ));
    }

    fn on_accept_ping(&mut self) {
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Pong,
        ));
    }

    fn on_accept_text(&mut self, s: String) {
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Text(format!("Here king, you dropped this: \"{s:}\"")),
        ));
    }

    fn on_accept_set_sim_speed(&mut self, speed: u32) {
        self.world.tick_rate = speed;
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Ack,
        ));
    }

    fn on_accept_find_grid_by_name(&mut self, name: String) {
        if let Some(id) = get_grid_by_name(&self.world.grids, &name) {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Entity(id),
            ));
        } else {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Text("No grid with that name.".into()),
            ));
        }
    }

    fn on_accept_list_grids(&mut self) {
        for (id, grid) in self.world.grids.iter() {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::GridInfo(*id, grid.name.clone(), grid.particle_location),
            ));
        }
    }

    fn on_accept_list_prototypes(&mut self) {
        for (id, proto) in self.world.prototypes.iter() {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Proto(*id, proto.clone()),
            ));
        }
    }

    fn on_accept_list_parts(&mut self) {
        for (id, part) in self.world.parts.iter() {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Part(*id, *part),
            ));
        }
    }

    fn on_accept_list_thrusters(&mut self) {
        for (id, thr) in self.world.thrusters.iter() {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Thruster(*id, thr.clone()),
            ));
        }
    }

    fn on_accept_list_computers(&mut self) {
        for (id, cpu) in self.world.computers.iter() {
            self.broadcast.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Computer(*id, cpu.clone()),
            ));
        }
    }

    fn on_accept_req_server_info(&mut self) {
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::ServerStatistics(self.get_statistics()),
        ));
    }

    fn send_blob_data<T: Serialize>(
        out: &MessageQueue<Message>,
        table: TableIdent,
        entities: &Components<T>,
    ) {
        if let Ok(bytes) = bincode::serialize(&entities) {
            let blob = Blob::new(bytes, table);
            out.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::BlobResponse(blob),
            ));
        } else {
            out.push(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Text(format!("Failed to serialize table: {:?}", table)),
            ));
        }
    }

    fn serialize_table<T: Serialize>(entities: &Components<T>) -> Option<Vec<u8>> {
        bincode::serialize(&entities).ok()
    }

    fn get_blob(&self, table: TableIdent) -> Option<Blob> {
        let data = match table {
            TableIdent::Blueprints => Self::serialize_table(&self.world.blueprints),
            TableIdent::Grids => Self::serialize_table(&self.world.grids),
            TableIdent::Protos => Self::serialize_table(&self.world.prototypes),
            TableIdent::Parts => Self::serialize_table(&self.world.parts),
            TableIdent::Thrusters => Self::serialize_table(&self.world.thrusters),
            TableIdent::Computers => Self::serialize_table(&self.world.computers),
            TableIdent::Chunks => Self::serialize_table(&self.world.terrain_chunks),
            TableIdent::Tiles => Self::serialize_table(&self.world.terrain_tiles),
            TableIdent::Inventories => Self::serialize_table(&self.world.inventories),
            TableIdent::Machines => Self::serialize_table(&self.world.machines),
        };
        data.map(|data| Blob::new(data, table))
    }

    fn on_accept_client_blob_request(&mut self, client: ClientId, table: TableIdent) {
        if let Some(blob) = self.get_blob(table) {
            let msg = Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::BlobResponse(blob),
            );
            self.outgoing.push((client, msg));
        } else {
            let msg = Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Text(format!("Failed to serialize table: {:?}", table)),
            );
            self.outgoing.push((client, msg));
        }
    }
}

fn server_thread(
    server: Arc<RwLock<ServerNode>>,
    incoming_queue: MessageQueue<Message>,
    broadcast: MessageQueue<Message>,
    outgoing: MessageQueue<(ClientId, Message)>,
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

            while let Some(sm) = broadcast.pop() {
                server.broadcast(sm);
            }

            while let Some((id, msg)) = outgoing.pop() {
                server.send(id, msg);
            }
        }
    }
}

struct DedicatedServerApp {
    app: BasicApp,
    terminal: Terminal<TermCmd>,
    server: ServerApp,
    assets: Assets,
}

impl DedicatedServerApp {
    fn new(saves_dir: &str, save_name: &str, port: u16) -> Self {
        let mut app = BasicApp::new("Barycenter Server", TraceLogLevel::LOG_INFO);

        let mut assets = Assets::default();

        assets::load_assets(&mut assets, &mut app.handle, &app.thread);

        let mut cmds = server_console_commands();
        cmds.extend(world_delta_commands());
        cmds.extend(blob_info_commands());

        Self {
            app,
            terminal: Terminal::with_commands(cmds),
            server: ServerApp::new(saves_dir, save_name, port),
            assets,
        }
    }

    fn on_terminal_command(&mut self, cmd: TermCmd) {
        match cmd {
            TermCmd::Say(_) => self.terminal.log_info("Woooo!".to_string()),
            TermCmd::Clear => self.terminal.clear(),
            TermCmd::Exit => self.app.exit(),
            TermCmd::EchoSaveInfo => {
                self.echo_save_info();
            }
            TermCmd::LoadSave(name) => {
                self.load_save_file(name);
            }
            TermCmd::SaveWorldToDisk(path, overwrite) => {
                self.save_world_to_disk(path, overwrite);
            }
            TermCmd::ListSaves => {
                self.list_saves();
            }
            TermCmd::ListGrids => {
                self.list_grids();
            }
            TermCmd::ListParts => {
                self.list_parts();
            }
            TermCmd::ListProtos => {
                self.list_prototypes();
            }
            TermCmd::SetSimSpeed(speed) => {
                self.server.world.tick_rate = speed;
            }
            TermCmd::World(delta) => match self.server.world.apply(delta) {
                Ok(()) => self.terminal.log_info("OK"),
                Err(e) => self.terminal.log_error(format!("FAILED: {:?}", e)),
            },
            TermCmd::ListBlueprints => {
                self.list_blueprints();
            }
            TermCmd::PrintBlobInfo(TableIdent::Blueprints) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.blueprints);
            }
            TermCmd::PrintBlobInfo(TableIdent::Grids) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.grids);
            }
            TermCmd::PrintBlobInfo(TableIdent::Protos) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.prototypes);
            }
            TermCmd::PrintBlobInfo(TableIdent::Parts) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.parts);
            }
            TermCmd::PrintBlobInfo(TableIdent::Thrusters) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.thrusters);
            }
            TermCmd::PrintBlobInfo(TableIdent::Computers) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.computers);
            }
            TermCmd::PrintBlobInfo(TableIdent::Chunks) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.terrain_chunks);
            }
            TermCmd::PrintBlobInfo(TableIdent::Tiles) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.terrain_tiles);
            }
            TermCmd::PrintBlobInfo(TableIdent::Inventories) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.inventories);
            }
            TermCmd::PrintBlobInfo(TableIdent::Machines) => {
                Self::list_blob_info(&mut self.terminal, &self.server.world.machines);
            }
            _ => self.terminal.log_warn(format!("Unsupported: {:?}", cmd)),
        }
    }

    fn list_blob_info<T: Serialize>(term: &mut Terminal<TermCmd>, entities: &Components<T>) {
        let start = Instant::now();
        let bin = bincode::serialize(entities);
        let delta = Instant::now() - start;
        match bin {
            Ok(bin) => {
                let md5 = md5::compute(&bin);
                term.log_info(format!("Len:   {} entities", entities.len()));
                term.log_info(format!("Bytes: {}", bin.len()));
                term.log_info(format!("MD5:   {:?}", md5));
                term.log_info(format!("Dur:   {} us", delta.as_micros()));
            }
            Err(e) => {
                term.log_error(format!("Failed to serialize: {:?}", e));
            }
        }
    }

    fn list_prototypes(&mut self) {
        if self.server.world.prototypes.is_empty() {
            self.terminal.log_info("(no prototypes)");
        }
        for (id, proto) in self.server.world.prototypes.iter() {
            let s = format!(
                "{} {} {:?} {}",
                id,
                proto.name,
                proto.classification(),
                proto.mass,
            );
            self.terminal.log_info(s);
        }
    }

    fn list_grids(&mut self) {
        if self.server.world.grids.is_empty() {
            self.terminal.log_info("(no grids)");
        }
        for (id, grid) in self.server.world.grids.iter() {
            let s = format!(
                "{} {}, {:?} {}",
                id,
                grid.name,
                grid.blueprint,
                distance_str_v(grid.particle_location.translation.into()),
            );
            self.terminal.log_info(s);
        }
    }

    fn list_parts(&mut self) {
        if self.server.world.parts.is_empty() {
            self.terminal.log_info("(no parts)");
        }
        for (id, part) in self.server.world.parts.iter() {
            let s = format!("{} {:?}", id, part);
            self.terminal.log_info(s);
        }
    }

    fn list_saves(&mut self) {
        let saves = list_saves_in_dir(&self.server.saves_dir);
        for s in saves {
            self.terminal.log_info(format!("{}", s.display()));
        }
    }

    fn list_blueprints(&mut self) {
        if self.server.world.blueprints.is_empty() {
            self.terminal.log_info("(no blueprints)");
        }
        for (id, bp) in self.server.world.blueprints.iter() {
            let s = format!(
                "{} {} v{} {} parts",
                id,
                bp.id.0,
                bp.id.1,
                bp.blueprint.part_count()
            );
            self.terminal.log_info(s);
        }
    }

    fn echo_save_info(&mut self) {
        self.terminal.log_info(format!(
            "Saves found in {}",
            self.server.saves_dir.display()
        ));
        self.terminal
            .log_info(format!("Save name is {:?}", self.server.save_name));
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
            d.clear_background(Color::new(20, 20, 20, 255));

            for grid in self.server.world.grids.values() {
                let pos = grid.particle_location.translation;
                let x = pos.x as i32 + d.get_render_width() / 2;
                let y = pos.y as i32 + d.get_render_height() / 2;
                let size = 12;
                d.draw_circle(x, y, 3.0, Color::WHITE);
                d.draw_text(
                    &grid.name.to_uppercase(),
                    x + 8,
                    y - size / 2,
                    size,
                    Color::WHITE,
                );
            }

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
                format!("Machines:  {}", self.server.world.machines.len()),
                format!("Asteroids: {}", self.server.world.asteroids.len()),
                format!("Chunks:    {}", self.server.world.terrain_chunks.len()),
                format!("Tiles:     {}", self.server.world.terrain_tiles.len()),
                format!("GAUs:      {}", self.server.world.grid_acceleration_updates),
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
