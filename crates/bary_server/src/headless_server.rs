use bary_core::prelude::*;
use bary_ipc::*;
use bary_sim::*;
use log::*;
use log::{debug, info, warn};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

pub fn sync_frame_from_world(world: &World) -> SyncFrame {
    let grid_bytes = bincode::serialize(&world.grids).unwrap();
    let grid_hash = u128::from_be_bytes(md5::compute(&grid_bytes).0);
    let inv_bytes = bincode::serialize(&world.inventories).unwrap();
    let inv_hash = u128::from_be_bytes(md5::compute(&inv_bytes).0);
    let thr_bytes = bincode::serialize(&world.thrusters).unwrap();
    let thr_hash = u128::from_be_bytes(md5::compute(&thr_bytes).0);
    SyncFrame {
        tick: world.ticks,
        next_id: world.spawner.next(),
        grids: grid_hash,
        inventories: inv_hash,
        thrusters: thr_hash,
    }
}

pub struct HeadlessServerApp {
    world: World,
    client_telemetry: HashMap<ClientId, ClientTelemetry>,
    tick_rate: u32,
    node: ServerNode,
    update_timer: WallTimer,
    print_timer: WallTimer,
    md5_sync_timer: WallTimer,
    queued_deltas: Vec<WorldDelta>,
}

impl HeadlessServerApp {
    pub fn new(save_path: Option<String>, port: u16, speed: u32) -> BaryResult<Self> {
        let node = ServerNode::new(port);

        let world = World::empty();

        let mut s = Self {
            world,
            client_telemetry: HashMap::new(),
            tick_rate: speed,
            node,
            update_timer: WallTimer::hz(TICKS_PER_SECOND as u32),
            print_timer: WallTimer::hz(1),
            md5_sync_timer: WallTimer::hz(2),
            queued_deltas: Vec::new(),
        };

        if let Some(path) = save_path {
            s.load_save(&path)?;
        }

        Ok(s)
    }

    pub fn load_save(&mut self, save: &str) -> BaryResult<()> {
        self.world = load_world(save)?;
        Ok(())
    }

    pub fn get_statistics(&self) -> ServerStatistics {
        let mut clients = Vec::new();
        for client in self.node.renet().clients_id_iter() {
            if let Ok(info) = self.node.renet().network_info(client) {
                clients.push((client, info.into()));
            }
        }
        ServerStatistics { clients }
    }

    fn send_driver_packet(&mut self) {
        let deltas = self.queued_deltas.drain(..).collect();
        let msg = MessageKind::Driver {
            ticks: self.world.ticks,
            deltas,
            players: self.world.players.clone(),
        };
        self.node.broadcast(msg.with_source(MessageSource::Server));
    }

    fn send_tlm_current_tick(&mut self) {
        self.node.broadcast(
            MessageKind::CurrentTick(self.world.ticks).with_source(MessageSource::Server),
        );
    }

    fn send_tlm_grid_pos(&mut self) {
        for grid in self.world.grids.values() {
            let pos = grid.particle_location;
            self.node.broadcast(
                MessageKind::GridPos(grid.name.clone(), pos).with_source(MessageSource::Server),
            );
        }
    }

    fn send_tlm_ack(&mut self) {
        self.node
            .broadcast(MessageKind::Ack.with_source(MessageSource::Server));
    }

    fn send_tlm_server_info(&mut self) {
        self.node.broadcast(Message::new(
            MessageSource::Server,
            MessageLevel::Telemetry,
            MessageKind::ServerStatistics(self.get_statistics()),
        ));
        self.node.broadcast(
            MessageKind::Text("Hello there!".to_string()).with_source(MessageSource::Server),
        );
    }

    fn send_md5_sync_packet(&mut self) {
        let frame = sync_frame_from_world(&self.world);
        let msg = MessageKind::SyncFrame(frame).with_source(MessageSource::Server);
        self.node.broadcast(msg);
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
                    error!("Failed to apply delta: {e}");
                } else {
                    debug!("Successfully applied delta: {delta:?}");
                    self.queued_deltas.push(delta.clone());
                }
            }
            MessageKind::LoadSave(path) => {
                self.on_accept_load_save(path);
            }
            _ => self.on_unsupported_message(),
        }
    }

    fn on_unsupported_message(&mut self) {
        warn!("Unsupported message type!");
        self.node.broadcast(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Unsupported,
        ));
    }

    fn on_accept_load_save(&mut self, path: String) {
        match load_world(&path) {
            Ok(world) => {
                self.world = world;
                info!("Loaded {path}");
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
    }

    fn on_accept_ping(&mut self) {
        self.node.broadcast(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Pong,
        ));
    }

    fn on_accept_client_tlm(&mut self, id: ClientId, tlm: ClientTelemetry) {
        self.client_telemetry.insert(id, tlm);
    }

    fn on_accept_text(&mut self, s: String) {
        self.node.broadcast(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Text(format!("Here king, you dropped this: \"{s:}\"")),
        ));
    }

    fn on_accept_set_sim_speed(&mut self, speed: u32) {
        self.tick_rate = speed;
        self.node.broadcast(Message::new(
            MessageSource::Server,
            MessageLevel::Response,
            MessageKind::Ack,
        ));
    }

    fn on_accept_find_grid_by_name(&mut self, name: String) {
        if let Some(id) = get_grid_by_name(&self.world.grids, &name) {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Entity(id),
            ));
        } else {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Text("No grid with that name.".into()),
            ));
        }
    }

    fn on_accept_list_grids(&mut self) {
        for (id, grid) in self.world.grids.iter() {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::GridInfo(*id, grid.name.clone(), grid.particle_location),
            ));
        }
    }

    fn on_accept_list_prototypes(&mut self) {
        for (id, proto) in self.world.prototypes.iter() {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Proto(*id, proto.clone()),
            ));
        }
    }

    fn on_accept_list_parts(&mut self) {
        for (id, part) in self.world.parts.iter() {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Part(*id, *part),
            ));
        }
    }

    fn on_accept_list_thrusters(&mut self) {
        for (id, thr) in self.world.thrusters.iter() {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Thruster(*id, thr.clone()),
            ));
        }
    }

    fn on_accept_list_computers(&mut self) {
        for (id, cpu) in self.world.computers.iter() {
            self.node.broadcast(Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Computer(*id, cpu.clone()),
            ));
        }
    }

    fn on_accept_req_server_info(&mut self) {
        self.node.broadcast(Message::new(
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
            TableIdent::Excavators => Self::serialize_table(&self.world.excavators),
            TableIdent::Players => Self::serialize_table(&self.world.players),
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
            self.node.send(client, msg);
        } else {
            let msg = Message::new(
                MessageSource::Server,
                MessageLevel::Response,
                MessageKind::Text(format!("Failed to serialize table: {:?}", table)),
            );
            self.node.send(client, msg);
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
                self.node.send(client, msg);
            }
            Err(err) => {
                let msg = Message::new(
                    MessageSource::Server,
                    MessageLevel::Response,
                    MessageKind::Text(format!("Failed to serialize table: {:?}", err)),
                );
                self.node.send(client, msg);
            }
        }
    }
}

impl Application for HeadlessServerApp {
    fn update(&mut self) {
        if !self.update_timer.tick() {
            return;
        }

        if self.md5_sync_timer.tick() {
            self.send_md5_sync_packet();
        }

        for msg in self.node.update() {
            self.on_accept_message(msg.clone());
        }

        for _ in 0..self.tick_rate {
            update_world(&mut self.world);
        }

        self.send_driver_packet();
        self.send_tlm_current_tick();
        self.send_tlm_grid_pos();
        self.send_tlm_server_info();
    }

    fn draw(&mut self) {
        if !self.print_timer.tick() {
            return;
        }

        info!("Running da server.");
        info!("Ticks:  {}", self.world.ticks);
        info!(
            "Update: {:0.2} / {:0.2} Hz",
            self.update_timer.actual_rate(),
            self.update_timer.nominal_rate()
        );
        info!(
            "Print:  {:0.2} / {:0.2} Hz",
            self.print_timer.actual_rate(),
            self.print_timer.nominal_rate()
        );
    }

    fn should_exit(&self) -> bool {
        false
    }
}
