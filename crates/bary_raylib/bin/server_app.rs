use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::*;
use bary_raylib::assets::Assets;
use bary_raylib::persistence::{list_saves_in_dir, load_world, save_world};
use bary_raylib::render::{draw_terminal, draw_world};
use bary_raylib::sim::{
    ClientSpecificInfo, animate_camera_towards_target, apparent_datetime, apply_delta,
    timedelta_from_delta_ticks,
};
use bary_raylib::utils::{Application, BasicApp};
use bary_raylib::*;
use bary_sim::*;
use bary_terminal::Terminal;
use clap::Parser;
use log::{debug, info, warn};
use raylib::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct RateCalculator {
    buffer: Vec<Instant>,
    times_called: u64,
}

impl RateCalculator {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            times_called: 0,
        }
    }

    fn update_rate(&mut self) {
        let now = Instant::now();
        self.buffer.push(now);
        if self.buffer.len() > 100 {
            self.buffer.remove(0);
        }
        self.times_called += 1;
    }

    fn rate(&self) -> f64 {
        if self.buffer.len() < 2 {
            return 0.0;
        }

        let first = self.buffer.first().unwrap();
        let last = self.buffer.last().unwrap();
        let delta = *last - *first;

        (self.buffer.len() - 1) as f64 / delta.as_secs_f64()
    }

    fn count(&self) -> u64 {
        self.times_called
    }
}

struct DeltaLog {
    tick: u64,
    source: MessageSource,
    delta: WorldDelta,
}

pub struct ServerApp {
    app: BasicApp,
    client: ClientSpecificInfo,
    terminal: Terminal<TermCmd>,
    assets: Assets,
    saves_dir: PathBuf,
    save_name: Option<String>,
    world: World,
    client_telemetry: HashMap<ClientId, ClientTelemetry>,
    tick_rate: u32,
    node: Arc<RwLock<ServerNode>>,
    incoming_transactions: MessageQueue<Message>,
    broadcast: MessageQueue<Message>,
    outgoing: MessageQueue<(ClientId, Message)>,
    _server_thread: JoinHandle<()>,
    update_timer: WallTimer,
    world_timer: WallTimer,
    sync_timer: WallTimer,
    draw_timer: WallTimer,
    queued_deltas: Vec<WorldDelta>,
    delta_history: Vec<DeltaLog>,
    physics_rate: RateCalculator,
    draw_rate: RateCalculator,
}

impl ServerApp {
    pub fn new(
        saves_dir: impl Into<PathBuf>,
        save_name: impl Into<String>,
        port: u16,
    ) -> BaryResult<Self> {
        let saves_dir = saves_dir.into();
        let save_name = save_name.into();

        let incoming_transactions = new_message_queue();
        let broadcast = new_message_queue();
        let outgoing = new_message_queue();

        let node = Arc::new(RwLock::new(ServerNode::new(port)));

        let mut terminal = Terminal::new();

        terminal.register_commands(server_console_commands());
        terminal.register_commands(world_delta_commands());
        terminal.register_commands(blob_info_commands());
        terminal.register_commands(terminal_commands());

        let mut app = BasicApp::new("Barycenter Server", TraceLogLevel::LOG_INFO);
        let mut assets = Assets::default();

        assets::load_assets(&mut assets, &mut app.handle, &app.thread);

        let path = saves_dir.join(&save_name);

        let world = load_world(path)?;

        Ok(Self {
            app,
            client: ClientSpecificInfo::new(),
            terminal,
            assets,
            saves_dir,
            save_name: Some(save_name),
            world,
            client_telemetry: HashMap::new(),
            tick_rate: 1,
            node: node.clone(),
            incoming_transactions: incoming_transactions.clone(),
            broadcast: broadcast.clone(),
            outgoing: outgoing.clone(),
            _server_thread: std::thread::spawn(|| {
                server_thread(node, incoming_transactions, broadcast, outgoing)
            }),
            update_timer: WallTimer::with_dur(Duration::from_millis(10)),
            world_timer: WallTimer::with_dur(Duration::from_millis(20)),
            sync_timer: WallTimer::with_dur(Duration::from_millis(20)),
            draw_timer: WallTimer::with_dur(Duration::from_millis(25)),
            queued_deltas: Vec::new(),
            delta_history: Vec::new(),
            physics_rate: RateCalculator::new(),
            draw_rate: RateCalculator::new(),
        })
    }

    pub fn get_statistics(&self) -> ServerStatistics {
        if let Ok(server) = self.node.read() {
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

    pub fn node(&self) -> Option<RwLockReadGuard<'_, bary_ipc::ServerNode>> {
        self.node.read().ok()
    }

    #[must_use]
    pub fn update(&mut self) -> Vec<Message> {
        let mut messages = Vec::new();

        if !self.update_timer.tick() {
            return messages;
        }

        while let Some(msg) = self.incoming_transactions.pop() {
            self.on_accept_message(msg.clone());
            messages.push(msg);
        }

        if self.world_timer.tick() {
            for _ in 0..self.tick_rate {
                update_world(&mut self.world);
            }
        }

        self.physics_rate.update_rate();

        if self.sync_timer.tick() {
            self.send_driver_packet();
            self.send_tlm_current_tick();
            self.send_tlm_grid_pos();
            self.send_tlm_server_info();
        }

        messages
    }

    fn send_driver_packet(&mut self) {
        let deltas = self.queued_deltas.drain(..).collect();
        let msg = MessageKind::Driver {
            ticks: self.world.ticks,
            deltas,
        };
        self.broadcast.push(msg.with_source(MessageSource::Server));
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

    fn on_accept_tlm(&mut self, id: ClientId, kind: MessageKind) {
        match kind {
            MessageKind::ClientTelemetry(tlm) => {
                self.on_accept_client_tlm(id, tlm);
            }
            _ => (),
        }
    }

    fn on_accept_message(&mut self, msg: Message) {
        debug!("Got a command: {:?}", msg);
        self.send_tlm_ack();

        let MessageSource::Client(client_id) = msg.source else {
            warn!("Got a message with unexpected source: {:?}", msg.source);
            return;
        };

        match msg.level {
            MessageLevel::Command => (),
            MessageLevel::Response => todo!(),
            MessageLevel::Telemetry => return self.on_accept_tlm(client_id, msg.kind),
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
            MessageKind::PrintEntityInfo(TableIdent::Grids) => {
                self.on_accept_list_grids();
            }
            MessageKind::PrintEntityInfo(TableIdent::Protos) => {
                self.on_accept_list_prototypes();
            }
            MessageKind::PrintEntityInfo(TableIdent::Parts) => {
                self.on_accept_list_parts();
            }
            MessageKind::PrintEntityInfo(TableIdent::Thrusters) => {
                self.on_accept_list_thrusters();
            }
            MessageKind::PrintEntityInfo(TableIdent::Computers) => {
                self.on_accept_list_computers();
            }
            MessageKind::RequestServerStatistics => {
                self.on_accept_req_server_info();
            }
            MessageKind::ClientBlobRequest(table) => {
                self.on_accept_client_blob_request(client_id, table);
            }
            MessageKind::ClientBlobRequestAll => {
                self.on_accept_client_blob_request_all(client_id);
            }
            MessageKind::RequestDelta(delta) => {
                if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
                    self.terminal
                        .log_error(format!("Failed to apply delta: {e}"));
                } else {
                    self.queued_deltas.push(delta.clone());
                    let log = DeltaLog {
                        tick: self.world.ticks,
                        source: msg.source,
                        delta,
                    };
                    self.delta_history.push(log);
                }
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

    fn on_accept_client_tlm(&mut self, id: ClientId, tlm: ClientTelemetry) {
        self.client_telemetry.insert(id, tlm);
    }

    fn on_accept_text(&mut self, s: String) {
        self.broadcast.push(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Text(format!("Here king, you dropped this: \"{s:}\"")),
        ));
    }

    fn on_accept_set_sim_speed(&mut self, speed: u32) {
        self.tick_rate = speed;
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
            TableIdent::Asteroids => Self::serialize_table(&self.world.asteroids),
            TableIdent::Pipes => Self::serialize_table(&self.world.pipes),
            TableIdent::Lights => Self::serialize_table(&self.world.lights),
        };
        data.map(|data| Blob::new(data, table))
    }

    fn on_accept_client_blob_request(&mut self, client: ClientId, table: TableIdent) {
        info!("Responding to blob request for {table} from {client}");
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

    fn on_accept_client_blob_request_all(&mut self, client: ClientId) {
        let blobs: Result<Vec<Blob>, BaryError> = TableIdent::all()
            .map(|table| {
                self.get_blob(table)
                    .ok_or(BaryError::FailedToSerialize(table))
            })
            .collect();

        match blobs {
            Ok(blobs) => {
                let msg = Message::new(
                    MessageSource::Server,
                    MessageLevel::Response,
                    MessageKind::MultiBlobResponse(
                        self.world.spawner.next(),
                        self.world.ticks,
                        blobs,
                    ),
                );
                self.outgoing.push((client, msg));
            }
            Err(err) => {
                let msg = Message::new(
                    MessageSource::Server,
                    MessageLevel::Response,
                    MessageKind::Text(format!("Failed to serialize table: {:?}", err)),
                );
                self.outgoing.push((client, msg));
            }
        }
    }

    fn print_blob_info(&mut self, table: TableIdent) {
        match table {
            TableIdent::Blueprints => Self::lsblob(&mut self.terminal, &self.world.blueprints),
            TableIdent::Grids => Self::lsblob(&mut self.terminal, &self.world.grids),
            TableIdent::Protos => Self::lsblob(&mut self.terminal, &self.world.prototypes),
            TableIdent::Parts => Self::lsblob(&mut self.terminal, &self.world.parts),
            TableIdent::Thrusters => Self::lsblob(&mut self.terminal, &self.world.thrusters),
            TableIdent::Computers => Self::lsblob(&mut self.terminal, &self.world.computers),
            TableIdent::Asteroids => Self::lsblob(&mut self.terminal, &self.world.asteroids),
            TableIdent::Chunks => Self::lsblob(&mut self.terminal, &self.world.terrain_chunks),
            TableIdent::Tiles => Self::lsblob(&mut self.terminal, &self.world.terrain_tiles),
            TableIdent::Inventories => Self::lsblob(&mut self.terminal, &self.world.inventories),
            TableIdent::Machines => Self::lsblob(&mut self.terminal, &self.world.machines),
            TableIdent::Pipes => Self::lsblob(&mut self.terminal, &self.world.pipes),
            TableIdent::Lights => Self::lsblob(&mut self.terminal, &self.world.lights),
        }
    }

    fn on_terminal_command(&mut self, cmd: TermCmd) {
        match cmd {
            TermCmd::Say(_) => self.terminal.log_info("Woooo!".to_string()),
            TermCmd::Clear => self.terminal.clear(),
            TermCmd::Exit => self.app.exit(),
            TermCmd::PrintSaveInfo => {
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
            TermCmd::PrintEntityInfo(table) => {
                self.print_entity_info(table);
            }
            TermCmd::PrintBlobInfo(table) => {
                self.print_blob_info(table);
            }
            TermCmd::SetSimSpeed(speed) => {
                self.tick_rate = speed;
            }
            TermCmd::World(delta) => match apply_delta(&mut self.world, delta) {
                Ok(()) => self.terminal.log_info("OK"),
                Err(e) => self.terminal.log_error(format!("FAILED: {:?}", e)),
            },
            TermCmd::Help => {
                self.terminal.print_help_command();
            }
            TermCmd::ToggleDebugMode => {
                self.terminal.toggle_debug_mode();
            }
            _ => self.terminal.log_warn(format!("Unsupported: {:?}", cmd)),
        }
    }
    fn lsblob<T: Serialize>(term: &mut Terminal<TermCmd>, entities: &Components<T>) {
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

    fn print_entity_info(&mut self, table: TableIdent) {
        match table {
            TableIdent::Blueprints => self.list_blueprints(),
            TableIdent::Grids => self.list_grids(),
            TableIdent::Protos => self.list_prototypes(),
            TableIdent::Parts => self.list_parts(),
            TableIdent::Thrusters => todo!(),
            TableIdent::Computers => todo!(),
            TableIdent::Asteroids => todo!(),
            TableIdent::Chunks => todo!(),
            TableIdent::Tiles => todo!(),
            TableIdent::Inventories => todo!(),
            TableIdent::Machines => todo!(),
            TableIdent::Pipes => todo!(),
            TableIdent::Lights => todo!(),
        }
    }

    fn list_prototypes(&mut self) {
        if self.world.prototypes.is_empty() {
            self.terminal.log_info("(no prototypes)");
        }
        for (id, proto) in self.world.prototypes.iter() {
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
        if self.world.grids.is_empty() {
            self.terminal.log_info("(no grids)");
        }
        for (id, grid) in self.world.grids.iter() {
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
        if self.world.parts.is_empty() {
            self.terminal.log_info("(no parts)");
        }
        for (id, part) in self.world.parts.iter() {
            let s = format!("{} {:?}", id, part);
            self.terminal.log_info(s);
        }
    }

    fn list_saves(&mut self) {
        let saves = list_saves_in_dir(&self.saves_dir);
        for s in saves {
            self.terminal.log_info(format!("{}", s.display()));
        }
    }

    fn list_blueprints(&mut self) {
        if self.world.blueprints.is_empty() {
            self.terminal.log_info("(no blueprints)");
        }
        for (id, bp) in self.world.blueprints.iter() {
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
        self.terminal
            .log_info(format!("Saves found in {}", self.saves_dir.display()));
        self.terminal
            .log_info(format!("Save name is {:?}", self.save_name));
    }

    fn load_save_file(&mut self, name: String) {
        let path = self.saves_dir.join(&name);

        match load_world(&path) {
            Ok(world) => {
                self.world = world;
                self.terminal.log_info(format!("Loaded save {name}"));
                self.save_name = Some(name);
            }
            Err(e) => {
                self.terminal
                    .log_error(format!("Failed to load world: {e:?}",));
            }
        }
    }

    fn save_world_to_disk(&mut self, save_name: String, overwrite: bool) {
        let path = self.saves_dir.join(save_name);
        let res = save_world(&path, &self.world, overwrite);
        if let Err(e) = res {
            self.terminal.log_error(format!("Failed: {e:?}"));
        } else {
            self.terminal
                .log_info(format!("Saved to {}", path.display()));
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

    loop {
        if let Ok(mut server) = server.write() {
            if update_timer.tick() {
                for msg in server.update() {
                    incoming_queue.push(msg);
                }
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

fn date_line(i: usize, ticks: u64, server_ticks: u64) -> String {
    let t = apparent_datetime(ticks)
        .format("%b %d %Y %I:%M:%S %p")
        .to_string();
    if i > 0 {
        let dticks = ticks as i64 - server_ticks as i64;
        let dt = timedelta_from_delta_ticks(dticks).as_seconds_f32();
        format!("{:2} {} ({:0.2})", i, t, dt)
    } else {
        format!("~S {}", t)
    }
}

fn get_server_camera_pos(world: &World) -> (Vec2, f32) {
    if world.grids.is_empty() {
        return (Vec2::ZERO, 4.0);
    }

    let sum: Vec2 = world
        .grids
        .values()
        .map(|g| g.centroid_isometry().translation)
        .sum();

    let center = sum / world.grids.len() as f32;

    let mut max_dist = 0.0;
    for grid in world.grids.values() {
        let d = center.distance(grid.centroid_isometry().translation);
        if max_dist < d {
            max_dist = d;
        }
    }

    let zoom = if max_dist == 0.0 {
        16.0
    } else {
        300.0 / max_dist
    };

    (center, zoom)
}

// fn draw_ship_label(d: &mut RaylibDrawHandle, font: &Font, p: Vector2, name: &str, color: Color) {
//     let size = 20.0;
//     d.draw_circle_v(p, 3.0, color);
//     let p = p + Vector2::new(8.0, -size / 2.0);
//     let t = name.to_uppercase();
//     d.draw_text_ex(font, &t, p, size, 0.0, color);
// }

impl Application for ServerApp {
    fn update(&mut self) {
        self.app.frame();

        for msg in self.update() {
            let s = format!("{:?}", msg);
            self.terminal.log_debug(s);
        }

        let cmds = self.terminal.handle_input(&self.app.input);

        for cmd in cmds {
            self.on_terminal_command(cmd);
        }

        self.delta_history
            .retain(|log| log.tick + 1000 > self.world.ticks);

        self.client.screen_dims = Vec2::new(
            self.app.handle.get_render_width() as f32,
            self.app.handle.get_render_height() as f32,
        );

        let (center, zoom) = get_server_camera_pos(&self.world);

        self.client.target_camera.isometry.translation = center;
        self.client.target_camera.zoom = zoom;

        animate_camera_towards_target(&mut self.client.target_camera, &mut self.client.camera);
    }

    fn draw(&mut self) {
        if !self.draw_timer.tick() {
            return;
        }

        self.draw_rate.update_rate();

        let n_clients = self.get_statistics().clients.len();

        // let font = self.assets.consolas.as_ref().unwrap();

        let mut client_info_lines = Vec::new();
        if let Some(node) = self.node() {
            for (id, info) in node.client_info() {
                let s = format!("  {}: TX {} RX {}", id.0, info.tx_count, info.rx_count);
                client_info_lines.push(s);
            }
        }

        let mut date_lines = vec![date_line(0, self.world.ticks, self.world.ticks)];
        for (i, tlm) in self.client_telemetry.values().enumerate() {
            let s = date_line(i + 1, tlm.ticks, self.world.ticks);
            date_lines.push(s);
        }

        let mut lines = vec![
            "Server Application".to_string(),
            format!("Ticks:     {}", self.world.ticks),
            format!(
                "Update:    {:0.2} Hz, {}",
                self.physics_rate.rate(),
                self.physics_rate.count()
            ),
            format!(
                "Draw:      {:0.2} Hz, {}",
                self.draw_rate.rate(),
                self.draw_rate.count()
            ),
            format!("Grids:     {}", self.world.grids.len()),
            format!("Parts:     {}", self.world.parts.len()),
            format!("Protos:    {}", self.world.prototypes.len()),
            format!("BPs:       {}", self.world.blueprints.len()),
            format!("Thrusters: {}", self.world.thrusters.len()),
            format!("Invs:      {}", self.world.inventories.len()),
            format!("Machines:  {}", self.world.machines.len()),
            format!("Asteroids: {}", self.world.asteroids.len()),
            format!("Chunks:    {}", self.world.terrain_chunks.len()),
            format!("Tiles:     {}", self.world.terrain_tiles.len()),
            format!("GAUs:      {}", self.world.grid_acceleration_updates),
            format!("Clients:   {}", n_clients),
        ];

        lines.extend(client_info_lines.clone());
        lines.push(String::new());
        lines.extend(date_lines.clone());

        lines.push(String::new());

        for log in self.delta_history.iter().rev() {
            let s = format!("{} {:?} {:?}", log.tick, log.source, log.delta);
            lines.push(s);
        }

        // let w = self.app.handle.get_render_width();
        // let h = self.app.handle.get_render_height();

        // let scale = 4.0;

        // let w2s = |p: Vec2| -> Vector2 {
        //     let x = p.x * scale + w as f32 / 2.0;
        //     let y = p.y * scale + h as f32 / 2.0;
        //     Vector2::new(x, y)
        // };

        self.app.handle.draw(&self.app.thread, |mut d| {
            d.clear_background(Color::new(20, 20, 20, 255));

            // for grid in self.world.grids.values() {
            //     let p = w2s(grid.particle_location.translation);
            //     draw_ship_label(&mut d, font, p, &grid.name, Color::WHITE);
            // }

            // for (_id, tlm) in &self.client_telemetry {
            //     for (_, name, iso) in &tlm.grid_transforms {
            //         let p = w2s(iso.translation);
            //         draw_ship_label(&mut d, font, p, name, Color::ORANGE.alpha(0.4));
            //     }
            // }

            let text = lines.join("\n");

            d.draw_text_ex(
                self.assets.consolas.as_ref().unwrap(),
                &text,
                Vector2::new(10.0, 10.0),
                16.0,
                0.0,
                Color::ORANGE,
            );

            let input = InputState::default();
            let gui = imgui::ImGui::new(Vec2::ZERO, None, input);

            draw_world(&self.world, &self.client, &self.assets, &gui, &mut d);

            draw_terminal(
                &mut d,
                &self.terminal,
                &self.assets,
                Color::BLACK.alpha(0.8),
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

fn main() -> BaryResult<()> {
    let args = Args::parse();

    info!("Starting dedicated server...");

    ServerApp::new(&args.saves_dir, &args.save_name, args.server_port)?.spin_forever();

    Ok(())
}
