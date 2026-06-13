use bary_core::prelude::*;
use bary_factory::*;
use bary_input::InputState;
use bary_ipc::*;
use bary_raylib::assets::*;
use bary_raylib::constants::*;
use bary_raylib::render::*;
use bary_raylib::sim::*;
use bary_raylib::sounds::*;
use bary_raylib::utils::*;
use bary_raylib::*;
use bary_server::*;
use bary_sim::Application;
use bary_sim::*;
use bary_terminal::Terminal;
use bary_ui::*;
use clap::Parser;
use early_returns::*;
use log::*;
use raylib::prelude::*;
use serde::Deserialize;
use std::thread::JoinHandle;
use std::time::Duration;

enum AppState {
    MainMenu,
    ListSaves,
    InWorld,
}

pub struct ClientApp {
    state: AppState,

    username: String,

    client: ClientSpecificInfo,
    world: World,
    #[allow(unused)]
    debug: DebugInfo,

    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,

    terminal: Terminal<TermCmd>,

    handle: RaylibHandle,
    thread: RaylibThread,
    assets: Assets,
    node: ClientNode,
    ui_state: UiInteractionState,

    update_timer: WallTimer,
    server_ping_timer: WallTimer,
    server_telemetry_timer: WallTimer,

    should_exit: bool,

    show_debug_text: bool,

    sounds: SoundEffects,

    menu_origin: Vec2,
    part_info_origin: Vec2,
}

impl ClientApp {
    fn new(args: Args, log_level: log::LevelFilter) -> Result<Self, Box<dyn std::error::Error>> {
        let input_queue = new_message_queue();
        let thread_copy = input_queue.clone();
        let _input_thread = std::thread::spawn(|| {
            if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
                println!("Error: {:?}", error)
            }
        });

        let world = if let Some(save) = args.save_file {
            load_world(&save)?
        } else {
            World::empty()
        };

        let mut terminal = Terminal::new();

        terminal.register_commands(all_commands());
        terminal.register_commands(client_request_blob_commands());
        terminal.register_commands(world_delta_commands());
        terminal.register_commands(terminal_commands());

        simple_logger::SimpleLogger::new()
            .with_level(log_level)
            .env()
            .init()
            .unwrap();

        let (mut handle, thread) = raylib::init()
            .size(1080, 700)
            .title("Barycenter")
            .log_level(TraceLogLevel::LOG_WARNING)
            .msaa_4x()
            .resizable()
            .build();

        handle.set_target_fps(1000);
        handle.maximize_window();
        handle.set_exit_key(None);
        handle.hide_cursor();

        let mut assets = Assets::default();
        load_assets(&mut assets, &mut handle, &thread);

        let node = ClientNode::with_str_addr(&args.server_addr)?;

        let update_timer = WallTimer::with_dur(Duration::from_millis(20));
        let server_ping_timer = WallTimer::with_dur(Duration::from_millis(1000));
        let server_telemetry_timer = WallTimer::with_dur(Duration::from_millis(100));
        let ui_state = UiInteractionState::default();

        Ok(ClientApp {
            state: AppState::MainMenu,
            username: args.username,
            world,
            client: ClientSpecificInfo::new(),
            debug: DebugInfo::default(),
            _input_thread,
            input_queue,
            terminal,
            handle,
            thread,
            assets,
            node,
            ui_state,
            update_timer,
            server_ping_timer,
            server_telemetry_timer,
            should_exit: false,
            show_debug_text: false,
            sounds: SoundEffects::new(),
            menu_origin: Vec2::new(300.0, 200.0),
            part_info_origin: Vec2::new(400.0, 350.0),
        })
    }

    fn send_telemetry_to_server(&mut self) {
        let tlm = ClientTelemetry {
            ticks: self.world.ticks,
        };
        self.node.send_telemetry(MessageKind::ClientTelemetry(tlm));

        let id = self
            .world
            .players
            .iter()
            .find_map(|(id, player)| (player.name == self.username).then(|| *id));

        if let Some(id) = id {
            self.client.player_id = Some(id);
            let delta = WorldDelta::SetPlayerPosition(id, self.client.camera.isometry);
            self.node.send_command(MessageKind::RequestDelta(delta));

            let world_pos = if let Some(screen_pos) = self.client.mouse_screen_position {
                Some(screen_to_world(
                    &self.client.camera,
                    screen_pos,
                    self.client.screen_dims,
                ))
            } else {
                None
            };

            let delta = WorldDelta::SetPlayerCursorPosition(id, world_pos);
            self.node.send_command(MessageKind::RequestDelta(delta));
        } else {
            warn!("Client with username {} isn't in the world", self.username);
            let delta = WorldDelta::SpawnPlayer(self.username.clone(), Isometry2d::ZERO);
            self.node.send_command(MessageKind::RequestDelta(delta));
        }
    }

    fn update(&mut self) {
        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        self.client.mouse_screen_position = self
            .handle
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(self.handle.get_mouse_position()));

        self.client.screen_dims = Vec2::new(
            self.handle.get_screen_width() as f32,
            self.handle.get_screen_height() as f32,
        );

        // HANDLE MESSAGES FROM NETWORK NODE

        while let Some(e) = self.input_queue.pop() {
            let focused = self.handle.is_window_focused();
            self.client.input.process_rdev_event(&e, focused);
        }

        let deltas = pre_simulation_update(&self.world, &mut self.client);

        for msg in self.node.update() {
            self.on_rcv_server_msg(msg);
        }

        let post_deltas = post_simulation_update(
            &self.world,
            &mut self.client,
            &mut self.sounds,
            self.terminal.is_focused(),
        );

        for delta in deltas.into_iter().chain(post_deltas) {
            self.node.send_command(MessageKind::RequestDelta(delta));
        }

        // HANDLE INPUTS FROM RDEV LISTENER THREAD

        if self.server_ping_timer.tick() {
            self.node.send_telemetry(MessageKind::Ping)
        }

        if self.server_telemetry_timer.tick() {
            self.send_telemetry_to_server();
        }

        // HANDLE RDEV EVENTS (DEPRECATED - USE INPUTSTATE)

        let cmds = self.terminal.handle_input(&self.client.input);

        for cmd in cmds {
            self.on_terminal_cmd(cmd);
        }

        self.process_events();
    }

    pub fn process_events(&mut self) {
        let on_gui = self.ui_state.is_on_gui();
        if !self.terminal.is_active() {
            process_event(&mut self.world, &mut self.client, &mut self.sounds, on_gui);

            if let Some(editor) = self.client.viewport.editor_mut() {
                let zoom = self.client.camera.zoom;
                if let Some(delta) = editor.handle_keys(&self.client.input, &self.world, zoom) {
                    self.node.send_command(MessageKind::RequestDelta(delta));
                }
            }
        }
    }

    pub fn on_rcv_server_msg(&mut self, msg: Message) {
        let s = format!("{:?}", msg);
        let k = if let MessageKind::Text(text) = &msg.kind {
            text.clone()
        } else {
            format!("{:?}", msg.kind)
        };
        let t = format!("[{:?}] {}", msg.source, k);
        self.terminal.log_debug(s);

        match msg.level {
            MessageLevel::Command => {
                self.terminal.log_info(t);
                info!("{msg:?}");
            }
            MessageLevel::Response => {
                self.terminal.log_info(t);
                info!("{msg:?}");
            }
            MessageLevel::Telemetry => {
                debug!("{msg:?}");
            }
        }

        match (msg.level, msg.kind) {
            (
                MessageLevel::Telemetry,
                MessageKind::Driver {
                    ticks,
                    deltas,
                    players,
                },
            ) => {
                self.on_driver_packet(ticks, deltas);
                self.world.players = players;
            }
            (MessageLevel::Response, MessageKind::BlobResponse(blob)) => {
                self.on_rcv_blob(blob);
            }
            (MessageLevel::Response, MessageKind::MultiBlobResponse(ent, ticks, blobs)) => {
                let delta = ticks as i64 - self.world.ticks as i64;
                self.terminal.log_warn(format!(
                    "Delta ticks: {} AKA {:0.3}",
                    delta,
                    timedelta_from_delta_ticks(delta).as_seconds_f64()
                ));
                self.world.spawner.set_next(ent);
                self.world.ticks = ticks;
                for blob in blobs {
                    self.on_rcv_blob(blob);
                }
            }
            (_, MessageKind::SyncFrame(frame)) => {
                debug!("Got frame: {:?}", frame);
                let our_frame = sync_frame_from_world(&self.world);
                debug!("Our frame: {:?}", our_frame);
            }
            _ => (),
        }
    }

    fn log_world_delta(&mut self, delta: &WorldDelta) {
        match delta {
            WorldDelta::SetPlayerPosition(_, _) => return,
            WorldDelta::SetPlayerCursorPosition(_, _) => return,
            _ => {
                self.client.chat.log(format!("{:?}", delta));
            }
        }
    }

    fn on_driver_packet(&mut self, ticks: u64, deltas: Vec<WorldDelta>) {
        while self.world.ticks + 1 < ticks {
            update_world(&mut self.world);
        }

        for delta in deltas {
            self.log_world_delta(&delta);
            if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
                error!("Failed to apply delta {:?}: {:?}", delta, e);
            }
        }

        if self.world.ticks < ticks {
            update_world(&mut self.world);
        }
    }

    fn unpack_blob<'a, T: Deserialize<'a>>(entities: &mut Components<T>, bytes: &'a [u8]) -> bool {
        if let Ok(e) = bincode::deserialize(bytes) {
            *entities = e;
            true
        } else {
            false
        }
    }

    fn on_rcv_blob(&mut self, blob: Blob) {
        info!("Got blob: {blob}");
        let table = blob.table();
        let success = match table {
            TableIdent::Blueprints => Self::unpack_blob(&mut self.world.blueprints, blob.data()),
            TableIdent::Grids => Self::unpack_blob(&mut self.world.grids, blob.data()),
            TableIdent::Protos => Self::unpack_blob(&mut self.world.prototypes, blob.data()),
            TableIdent::Parts => Self::unpack_blob(&mut self.world.parts, blob.data()),
            TableIdent::Thrusters => Self::unpack_blob(&mut self.world.thrusters, blob.data()),
            TableIdent::Computers => Self::unpack_blob(&mut self.world.computers, blob.data()),
            TableIdent::Chunks => Self::unpack_blob(&mut self.world.terrain_chunks, blob.data()),
            TableIdent::Tiles => Self::unpack_blob(&mut self.world.terrain_tiles, blob.data()),
            TableIdent::Inventories => Self::unpack_blob(&mut self.world.inventories, blob.data()),
            TableIdent::Machines => Self::unpack_blob(&mut self.world.machines, blob.data()),
            TableIdent::Asteroids => Self::unpack_blob(&mut self.world.asteroids, blob.data()),
            TableIdent::Pipes => Self::unpack_blob(&mut self.world.pipes, blob.data()),
            TableIdent::Lights => Self::unpack_blob(&mut self.world.lights, blob.data()),
            TableIdent::Excavators => Self::unpack_blob(&mut self.world.excavators, blob.data()),
            TableIdent::Players => Self::unpack_blob(&mut self.world.players, blob.data()),
        };

        if success {
            self.terminal
                .log_info(format!("Unpacked blob data for table {table}"));
        } else {
            self.terminal
                .log_error(format!("Failed to unpack blob for table {table}"));
        }
    }

    fn exit(&mut self) {
        info!("Exiting cleanly.");
        self.should_exit = true;
    }

    fn save(&mut self) {
        save_world_and_alert_chat(&self.world, &mut self.client);
    }

    fn open_editor(&mut self) {
        enter_ship_editor(&self.world, &mut self.client, &mut self.sounds)
    }

    fn leave_editor(&mut self) {
        if self.client.leave_editor() {
            self.sounds.push(SoundEffect::LeaveEditor);
        }
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.node.send_command(MessageKind::SetSimSpeed(speed));
    }

    fn docking_rotate(&mut self) {
        if let Some(free) = self.client.viewport.free_mut() {
            free.rotation = free.rotation.next();
        }
    }

    fn docking_shift(&mut self, delta: Vec2, is_x: bool) {
        if let Some(free) = self.client.viewport.free_mut() {
            if is_x {
                let delta = delta.with_y(0.0) / 4.0;
                let off = PartCoord(vround(delta));
                free.offset += off;
            } else {
                let delta = Vec2::new(0.0, delta.x);
                let off = PartCoord(vround(delta));
                free.offset += off;
            }
        }
    }

    fn docking_activate(&mut self) {
        if let Some(docking) = self.client.docking_interface() {
            let delta = WorldDelta::MergeGrids(
                docking.parent.grid_id,
                docking.child.grid_id,
                docking.offset,
                docking.rotation,
            );
            self.node.send_command(MessageKind::RequestDelta(delta));
        }
    }

    fn load_save_file(&mut self, path: String) {
        match load_world(&path) {
            Ok(world) => {
                self.world = world;
                self.state = AppState::InWorld;
                self.node.send_command(MessageKind::LoadSave(path));
            }
            Err(e) => {
                error!("Failed to load world: {e}");
            }
        }
    }

    pub fn on_drag(&mut self, id: UiMessage, delta: Vec2) {
        match id {
            UiMessage::DockingShiftX => {
                self.docking_shift(delta, true);
            }
            UiMessage::DockingShiftY => {
                self.docking_shift(delta, false);
            }
            UiMessage::DockingHandle => {
                self.menu_origin += delta;
            }
            UiMessage::MainMenuHandle => {
                self.menu_origin += delta;
            }
            UiMessage::SaveSelectHandle => {
                self.menu_origin += delta;
            }
            UiMessage::PartInfoHandle => {
                self.part_info_origin += delta;
            }
            _ => (),
        }
    }

    pub fn on_ui_event(&mut self, c: UiEvent) {
        debug!("{:?}", c);

        match c.kind {
            UiEventKind::Drag(delta) => {
                self.on_drag(c.msg, delta);
            }
            UiEventKind::Release => {
                self.sounds.push(sounds::SoundEffect::ButtonUp);
            }
            UiEventKind::Click => {
                self.sounds.push(sounds::SoundEffect::ButtonDown);
                match c.msg {
                    UiMessage::Exit => self.exit(),
                    UiMessage::SaveFile => self.save(),
                    UiMessage::OpenEditor => self.open_editor(),
                    UiMessage::LeaveEditor => self.leave_editor(),
                    UiMessage::AltMode => self.client.alt_mode ^= true,
                    UiMessage::DebugText => self.show_debug_text ^= true,
                    UiMessage::SimSpeed(sp) => self.set_sim_speed(sp),
                    UiMessage::DockingRotate => self.docking_rotate(),
                    UiMessage::DockingActivate => self.docking_activate(),
                    UiMessage::LoadSaveFile(path) => self.load_save_file(path),
                    UiMessage::GoToMainMenu => {
                        self.state = AppState::MainMenu;
                    }
                    UiMessage::SetMachineOnOff(machine_id, state) => {
                        warn!("Not handling machine on/off: {machine_id}, {state}");
                    }

                    UiMessage::LoadSinglePlayer => {
                        self.state = AppState::ListSaves;
                    }
                    UiMessage::JoinMultiplayer
                    | UiMessage::HostMultiplayer
                    | UiMessage::Settings => {
                        self.state = AppState::InWorld;
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn on_terminal_cmd(&mut self, cmd: TermCmd) {
        self.terminal.log_info(format!("{cmd:?}"));

        match cmd {
            TermCmd::Say(s) => {
                self.node.send_command(MessageKind::Text(s));
            }
            TermCmd::Ping => {
                self.node.send_command(MessageKind::Ping);
            }
            TermCmd::Clear => {
                self.terminal.clear();
            }
            TermCmd::SetSimSpeed(speed) => {
                self.set_sim_speed(speed);
            }
            TermCmd::FindGridByName(name) => {
                self.node.send_command(MessageKind::FindGridByName(name));
            }
            TermCmd::Exit => {
                self.exit();
            }
            TermCmd::PrintEntityInfo(table) => {
                self.node.send_command(MessageKind::PrintEntityInfo(table));
            }
            TermCmd::ServerConnect => {
                self.node.reconnect();
            }
            TermCmd::ServerDisconnect => {
                self.node.disconnect();
            }
            TermCmd::RequestServerStatistics => {
                self.node.send_command(MessageKind::RequestServerStatistics);
            }
            TermCmd::ClientReqBlob(table) => {
                self.node
                    .send_command(MessageKind::ClientBlobRequest(table));
            }
            TermCmd::ClientReqAllBlobs => {
                self.terminal
                    .log_warn(format!("Requesting all blobs at tick {}", self.world.ticks));
                self.node.send_command(MessageKind::ClientBlobRequestAll);
            }
            TermCmd::World(delta) => {
                self.node.send_command(MessageKind::RequestDelta(delta));
            }
            TermCmd::Help => {
                self.terminal.print_help_command();
            }
            TermCmd::ToggleDebugMode => {
                self.terminal.toggle_debug_mode();
            }
            _ => self.terminal.log_error(format!("Unsupported: {:?}", cmd)),
        }
    }
}

fn draw_debug_info(
    world: &World,
    client: &ClientSpecificInfo,
    assets: &Assets,
    timers: &DebugTimers,
    node: &ClientNode,
    update_timer: &WallTimer,
    ping_timer: &WallTimer,
    tlm_timer: &WallTimer,
    username: &str,
    user_id: Option<Ent>,
    d: &mut RaylibDrawHandle,
) {
    let size = size_in_bytes(world);
    let mut s = "Barycenter Client".to_string();

    let consist = is_world_consistent(world);
    s += &format!("\nOK: {:?}", consist);

    let fmt_time = |d: std::time::Duration, t: std::time::Duration| {
        let p = d.as_secs_f64() / t.as_secs_f64();
        format!(
            "    {:09} ns\n    {:05.04} ms\n    {:3.1}%",
            d.as_nanos(),
            d.as_nanos() as f64 / 1000000.0,
            p * 100.0
        )
    };

    s += &format!(
        "\nW {}/{} {} {:0.1} {}",
        timers.ticks,
        client.tick_rate,
        world.ticks,
        apparent_elapsed_time(world.ticks).as_secs_f64(),
        apparent_datetime(world.ticks).format("%b %d %Y %I:%M:%S %p"),
    );

    s += &format!("\nUsername:  {}", username);
    if let Some(uid) = user_id {
        s += &format!("\nUser ID:   {}", uid);
    }

    s += &format!("\nFPS:       {}", d.get_fps());
    s += &format!("\nMemory:    {:0.3} KB", size as f64 / 1000.0);
    s += &format!("\nZoom:      {:0.3}", client.camera.zoom);

    s += &format!(
        "\nUpdate:    {:0.2} Hz / {:0.2} Hz ",
        update_timer.actual_rate(),
        update_timer.nominal_rate()
    );
    s += &format!(
        "\nPing:      {:0.2} Hz / {:0.2} Hz ",
        ping_timer.actual_rate(),
        ping_timer.nominal_rate()
    );
    s += &format!(
        "\nTLM:       {:0.2} Hz / {:0.2} Hz ",
        tlm_timer.actual_rate(),
        tlm_timer.nominal_rate()
    );

    s += "\n";

    s += &format!("\nConnected: {}", node.is_connected());
    s += &format!("\nAddress:   {}", node.server_addr());
    s += &format!("\nRX:        {}", node.rx_count());
    s += &format!("\nTX:        {}", node.tx_count());
    s += &format!("\nErrors:    {}", node.errors());

    s += "\n";

    s += &format!("\nTicks:     {}", world.ticks);
    s += &format!("\nGrids:     {}", world.grids.len());
    s += &format!("\nParts:     {}", world.parts.len());
    s += &format!("\nProtos:    {}", world.prototypes.len());
    s += &format!("\nBPs:       {}", world.blueprints.len());
    s += &format!("\nThrusters: {}", world.thrusters.len());
    s += &format!("\nInvs:      {}", world.inventories.len());
    s += &format!("\nMachines:  {}", world.machines.len());
    s += &format!("\nAsteroids: {}", world.asteroids.len());
    s += &format!("\nChunks:    {}", world.terrain_chunks.len());
    s += &format!("\nTiles:     {}", world.terrain_tiles.len());
    s += &format!("\nGAUs:      {}", world.grid_acceleration_updates);
    s += &format!("\nPlayers:   {}", world.players.len());

    for player in world.players.values() {
        s += &format!("\n  {} {}", player.name, player.state);
    }

    s += "\n";

    let total = timers.total();

    s += &format!("\ntotal\n{}", fmt_time(total, total));

    for timer in timers.timers.iter() {
        let time = fmt_time(*timer.1, total);
        s += &format!("\n{}\n{}", timer.0, time);
    }

    let font_size = 16.0;

    let font = assets.consolas.as_ref().unwrap();
    let dims = font.measure_text(&s, font_size, 0.0);
    let padding = 10;
    let pos = Vector2::new(10.0, 60.0);

    d.draw_rectangle(
        pos.x as i32,
        pos.y as i32,
        dims.x as i32 + padding * 2,
        dims.y as i32 + padding * 2,
        Color::new(20, 20, 20, 255).alpha(0.9),
    );

    let rec = Rectangle::new(
        pos.x,
        pos.y,
        dims.x + padding as f32 * 2.0,
        dims.y + padding as f32 * 2.0,
    );

    d.draw_rectangle_lines_ex(rec, 3.0, Color::new(60, 60, 60, 255).alpha(0.9));

    let pos = Vector2::new(pos.x + padding as f32, pos.y + padding as f32);

    d.draw_text_ex(font, &s, pos, 16.0, 0.0, Color::ORANGE);
}

fn handle_sounds<'a>(
    sounds: &SoundEffects,
    audio: &'a RaylibAudio,
    active_sounds: &mut Vec<Sound<'a>>,
) {
    for sound in sounds {
        let path = sound.to_path();
        let sound = match audio.new_sound(path) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to load sound: {}", e);
                continue;
            }
        };

        sound.set_volume(0.4);
        sound.play();

        active_sounds.push(sound);
    }

    active_sounds.retain(|s| s.is_playing());
}

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    username: String,
    #[arg(short)]
    save_file: Option<String>,
    #[arg(short = 'a', default_value = "127.0.0.1:5000")]
    server_addr: String,
    #[arg(short = 'p', default_value = "5000")]
    server_port: u16,
    #[arg(long)]
    run_server: bool,
}

fn draw_ui_aabb(d: &mut RaylibDrawHandle, aabb: AABB, color: Color, fill: bool) {
    let tl = aabb.lower();
    if fill {
        d.draw_rectangle(
            tl.x as i32,
            tl.y as i32,
            aabb.span.x as i32,
            aabb.span.y as i32,
            color,
        );
    } else {
        d.draw_rectangle_lines(
            tl.x as i32,
            tl.y as i32,
            aabb.span.x as i32,
            aabb.span.y as i32,
            color,
        );
    }
}

#[allow(unused)]
fn generate_assets_ui(assets: &Assets, input: &InputState, width: f32, height: f32) -> Tree<()> {
    let mut root = Node::new(width, Size::Fit);

    for (ctx, mapping) in assets.keybinds.iter() {
        let mut col = Node::new(Size::Grow, height / 2.0)
            .down()
            .with_padding(4.0)
            .with_child_gap(1.0);

        let s = format!("{:?}", ctx);
        let header = Node::text(Size::Grow, 40, s);

        col.add_child(header);

        let actions = ActionSet::new(input, mapping);

        for (key, action) in mapping.iter() {
            let color = if actions.just_triggered(*action) {
                Color::TEAL
            } else if actions.is_active(*action) {
                Color::new(60, 60, 110, 255)
            } else {
                Color::new(20, 20, 20, 255)
            };

            let color = [
                color.r as f32 / 255.0,
                color.g as f32 / 255.0,
                color.b as f32 / 255.0,
                color.a as f32 / 255.0,
            ];

            let line = format!("{:?}: {:?}", key, action);
            let row = Node::text(Size::Grow, 30, line).with_color(color);
            col.add_child(row);
        }

        root.add_child(col);

        // x += width + padding;
    }

    Tree::new().with_layout(root, None)
}

fn bary_to_raylib(c: BColor) -> Color {
    Color::new(c.r, c.g, c.b, c.a)
}

fn node_color<T: bary_ui::UiMsg>(node: &bary_ui::Node<T>) -> Color {
    match node.kind() {
        NodeType::Text(_) => Color::TEAL.alpha(0.3),
        NodeType::Button(_, _) => Color::ORANGE,
        NodeType::Image(_) => Color::YELLOW,
        NodeType::Spacer => bary_to_raylib(BColor::gray(50)),
        NodeType::Row(_) => bary_to_raylib(BColor::gray(20)).alpha(0.95),
        NodeType::Column(_) => bary_to_raylib(BColor::gray(20)).alpha(0.95),
        NodeType::DragHandle(_) => bary_to_raylib(BColor::gray(60)),
        NodeType::ProgressBar(_) => Color::TEAL,
    }
}

pub struct UiBuilder<'a> {
    font: &'a Font,
    font_size: f32,
}

impl<'a> UiBuilder<'a> {
    fn new(font: &'a Font) -> Self {
        Self {
            font,
            font_size: UI_FONT_SIZE as f32,
        }
    }

    fn draghandle<T: UiMsg>(&self, msg: impl Into<T>) -> Node<T> {
        Node::<T>::handle(Size::Grow, 20, msg)
    }

    fn sep<T: UiMsg>(&self) -> Node<T> {
        Node::structural(Size::Grow, 2)
    }

    fn text<T: UiMsg>(&self, s: impl Into<String>) -> Node<T> {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x;
        let h = dims.y;
        Node::text(w, h, s)
    }

    fn button<T: UiMsg>(&self, msg: impl Into<T>, s: impl Into<String>) -> Node<T> {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x + 36.0;
        let h = dims.y + 18.0;
        Node::button(s, msg, w, h)
    }

    fn sprite<T: UiMsg>(&self, s: impl Into<String>, texture: &Texture2D) -> Node<T> {
        let h = 70;
        let actual_w = texture.width;
        let actual_h = texture.height;
        let scale = actual_h as f32 / h as f32;
        let w = actual_w as f32 / scale;
        Node::image(s, w as u32, h)
    }

    fn progress_bar<T: UiMsg>(&self, val: f32) -> Node<T> {
        Node::progress_bar(Size::Grow, 11, val)
    }
}

fn make_docking_ui(
    builder: &UiBuilder,
    docking: &DockingInterface,
    world: &World,
) -> Option<Node<UiMessage>> {
    let parent = world.grids.try_get(docking.parent.grid_id).ok()?;
    let child = world.grids.try_get(docking.child.grid_id).ok()?;

    let sep = parent
        .particle_location
        .translation
        .distance(child.particle_location.translation);

    let sep = distance_str(sep as f64);

    let docking_ui = Node::column(
        400,
        vec![
            builder.draghandle(UiMessage::DockingHandle),
            builder.text("Docking Control Panel"),
            builder.text("Multiline string\nAnother line\nAnd another!"),
            builder.text(format!("{} <- {}", parent.name, child.name)),
            builder.text(format!("Separation: {}", sep)),
            builder.button(UiMessage::DockingShiftX, "Offset X"),
            builder.button(UiMessage::DockingShiftY, "Offset Y"),
            builder.button(UiMessage::DockingRotate, "Rotate"),
            builder.button(UiMessage::DockingActivate, "Dock"),
        ],
    );

    Some(docking_ui)
}

fn make_main_menu(ui: &UiBuilder) -> Node<UiMessage> {
    Node::column(
        800,
        vec![
            ui.draghandle(UiMessage::MainMenuHandle),
            ui.button(UiMessage::LoadSinglePlayer, "Load Single Player"),
            ui.button(UiMessage::JoinMultiplayer, "Join Multiplayer"),
            ui.button(UiMessage::HostMultiplayer, "Host Multiplayer"),
            ui.button(UiMessage::Settings, "Settings"),
            ui.button(UiMessage::Exit, "Exit to Desktop"),
        ],
    )
}

fn make_savefiles_menu(ui: &UiBuilder, saves: &[String]) -> Node<UiMessage> {
    let mut buttons = vec![ui.draghandle(UiMessage::SaveSelectHandle)];

    for s in saves {
        let b = ui.button(UiMessage::LoadSaveFile(s.clone()), s.clone());
        buttons.push(b);
    }

    buttons.push(ui.button(UiMessage::GoToMainMenu, "Return to Main Menu"));

    Node::column(800, buttons)
}

fn make_world_ui(ui: &UiBuilder, app: &ClientApp, tree: &mut Tree<UiMessage>) {
    let mut root: Node<UiMessage> = Node::root(Size::Fit, Size::Fit).with_children(
        [
            ui.button(UiMessage::GoToMainMenu, "Return to Main Menu"),
            ui.button(UiMessage::SaveFile, "Save Game"),
            ui.button(UiMessage::AltMode, "Toggle Alt Mode"),
            ui.button(UiMessage::DebugText, "Toggle Debug Text"),
            ui.button(UiMessage::SimSpeed(0), "Toggle Pause"),
            ui.button(UiMessage::SimSpeed(1), "Sim 1x"),
            ui.button(UiMessage::SimSpeed(10), "Sim 10x"),
            ui.button(UiMessage::SimSpeed(100), "Sim 100x"),
            ui.button(UiMessage::SimSpeed(1000), "Sim 1000x"),
        ]
        .into_iter(),
    );

    if app.client.selected_grid_loc().is_some() && app.client.viewport.editor().is_none() {
        let b = ui.button(UiMessage::OpenEditor, "Open Ship Editor");
        root.add_child(b);
    }

    if app.client.viewport.editor().is_some() {
        root.add_child(ui.button(UiMessage::LeaveEditor, "Leave Ship Editor"));
    }

    tree.add_layout(root, None);

    if let Some(docking) = app.client.docking_interface() {
        if let Some(ui) = make_docking_ui(ui, &docking, &app.world) {
            tree.add_layout(ui, app.menu_origin)
        }
    }
}

fn slot_info_str(idx: usize, slot: &InvSlot) -> String {
    if let Some(contents) = slot.contents() {
        format!(
            "{}. {:?} ({}) ({:0.1}%) {}",
            idx + 1,
            contents.0,
            contents.1,
            100.0 * slot.fill_percentage(),
            slot.mass(),
        )
    } else {
        format!("{}. Empty - can store [{:?}]", idx + 1, slot.filter())
    }
}

fn computer_info_str(cpu: &Computer) -> String {
    let mut lines = vec![
        format!("On: {}", cpu.on),
        format!("\nStatus: {:?}", cpu.status),
        format!("\nTicks: {}", cpu.ticks_this_cycle),
        format!("\nFired: {}", cpu.fired_this_tick),
        format!("\nIters: {}", cpu.iters),
    ];

    for cmd in &cpu.command_queue {
        let line = format!("\n  - {}", cmd);
        lines.push(line);
    }

    lines.into_iter().collect()
}

fn make_part_info_gui(ui: &UiBuilder, app: &ClientApp, tree: &mut Tree<UiMessage>) {
    let gridloc = some_or_return!(app.client.selected_grid_loc());
    let grid = ok_or_return!(app.world.grids.try_get(gridloc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(gridloc.coord));

    let mut children = Vec::new();

    let s = format!(
        "At {}-{}: {:?}",
        gridloc.grid_id,
        gridloc.coord,
        occ.to_array()
    );

    children.push(ui.draghandle(UiMessage::PartInfoHandle));
    children.push(ui.text(s));

    for (layer, part_id) in occ.iter() {
        children.push(ui.sep());

        let part = ok_or_continue!(app.world.parts.try_get(part_id));
        let part_local = part.region.to_local(gridloc.coord);
        let proto = ok_or_continue!(app.world.prototypes.try_get(part.prototype));

        children.push(ui.text(proto.name.clone()));

        if let Some(texture) = app.assets.part_textures.get(&proto.name) {
            children.push(ui.sprite(proto.name.clone(), texture));
        }

        if let Ok(cpu) = app.world.computers.try_get(part_id) {
            children.push(ui.button(
                UiMessage::SetComputerOnOff(part_id, !cpu.on),
                if cpu.on { "Turn Off" } else { "Turn On" },
            ));
            children.push(ui.button(UiMessage::SetComputerDrift(part_id), "Set to Drift"));
            let info = computer_info_str(cpu);
            children.push(ui.text(info));
        }
        if let Ok(thruster) = app.world.thrusters.try_get(part_id) {
            let s = format!("{:#?}", thruster);
            children.push(ui.button(
                UiMessage::SetThrusterOnOff(part_id, !thruster.is_on),
                if thruster.is_on {
                    "Turn Off"
                } else {
                    "Turn On"
                },
            ));
            children.push(ui.text(s));
        }
        if let Ok(light) = app.world.lights.try_get(part_id) {
            let s = format!("{:#?}", light);
            children.push(ui.text(s));
        }
        if let Ok(mac) = app.world.machines.try_get(part_id) {
            children.push(ui.progress_bar(mac.progress()));
            children.push(ui.button(
                UiMessage::SetMachineOnOff(part_id, !mac.enabled),
                if mac.enabled { "Turn Off" } else { "Turn On" },
            ));
        }
        if let Ok(inv) = app.world.inventories.try_get(part_id) {
            for (idx, slot) in inv.slots().enumerate() {
                let line = slot_info_str(idx, slot);
                children.push(ui.text(line));
                children.push(ui.progress_bar(slot.fill_percentage()));
            }
        }

        if app.client.alt_mode {
            let mut s = format!("Part ID: {}", part_id);
            s += &format!("\nPart local coord: {}", part_local);

            s += &format!(
                "\nRegion: {:?} {} {:?}",
                layer,
                part.region.bottom_left(),
                part.region.rot()
            );

            s += &format!(
                "\nPrototype: {} {} {:?}",
                proto.name,
                proto.mass,
                proto.classification()
            );
            children.push(ui.text(s));
        }
    }

    let node = Node::column(600, children);
    tree.add_layout(node, app.part_info_origin);
}

fn make_gui(font: &Font, app: &ClientApp) -> Tree<UiMessage> {
    let builder = UiBuilder::new(font);

    let mut tree = Tree::new();

    match app.state {
        AppState::MainMenu => {
            tree.add_layout(make_main_menu(&builder), app.menu_origin);
        }
        AppState::InWorld => {
            make_world_ui(&builder, app, &mut tree);
            make_part_info_gui(&builder, &app, &mut tree);
        }
        AppState::ListSaves => {
            let mut saves = list_saves_in_dir("saves/");
            saves.sort();
            let saves: Vec<_> = saves
                .into_iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            tree.add_layout(make_savefiles_menu(&builder, &saves), app.menu_origin);
        }
    }

    tree
}

const UI_FONT_SIZE: i32 = 18;

fn draw_node(
    state: &UiInteractionState,
    d: &mut RaylibDrawHandle,
    node: &Node<UiMessage>,
    font: &Font,
    assets: &Assets,
) {
    if !node.is_visible() {
        return;
    }

    let color = node_color(node);
    let aabb = node.aabb();

    let is_clicked = state.active() == node.on_click();
    let is_hovered = state.hot() == node.on_click();

    match node.kind() {
        NodeType::Button(text, _msg) => {
            let scale = if is_clicked { 0.95 } else { 1.0 };
            let color = if is_clicked {
                color.lerp(Color::BLACK, 0.2)
            } else if is_hovered {
                color.lerp(Color::WHITE, 0.3)
            } else {
                color
            };
            let aabb = aabb.scale_about_center(scale);
            draw_ui_aabb(d, aabb, color, true);

            let p = glam_to_raylib(aabb.center);
            let font_size = (UI_FONT_SIZE as f32 * scale) as i32;
            draw_text_centered(d, font, &text, p, font_size, Color::BLACK);
        }
        NodeType::Text(text) => {
            draw_ui_aabb(d, aabb, Color::new(30, 30, 30, 200), true);
            let p = glam_to_raylib(aabb.center);
            draw_text_centered(d, font, &text, p, UI_FONT_SIZE, Color::WHITE);
        }
        NodeType::DragHandle(_) => {
            let color = if is_clicked {
                color.lerp(Color::BLACK, 0.2)
            } else if is_hovered {
                color.lerp(Color::WHITE, 0.3)
            } else {
                color
            };
            draw_ui_aabb(d, aabb, color, true);
        }
        NodeType::Image(name) => {
            let p = aabb.lower();
            if let Some(texture) = assets.part_textures.get(name) {
                let source_rec =
                    Rectangle::new(0.0, 0.0, texture.width as f32, texture.height as f32);
                let dest_rec = Rectangle::new(p.x, p.y, aabb.span.x, aabb.span.y);
                draw_ui_aabb(d, aabb, Color::new(90, 90, 90, 255), false);
                d.draw_texture_pro(
                    texture,
                    source_rec,
                    dest_rec,
                    Vector2::zero(),
                    0.0,
                    Color::WHITE,
                );
            }
        }
        NodeType::ProgressBar(val) => {
            let dims = Vec2::new(aabb.span.x * *val, aabb.span.y);
            let bar_aabb = AABB::from_arbitrary(aabb.lower(), aabb.lower() + dims);
            draw_ui_aabb(d, aabb, Color::new(40, 40, 40, 255), true);
            draw_ui_aabb(d, aabb, Color::new(10, 10, 10, 255), false);
            draw_ui_aabb(d, bar_aabb, color, true);
        }
        _ => {
            draw_ui_aabb(d, aabb, color, true);
        }
    };
}

fn draw_gui(
    state: &UiInteractionState,
    d: &mut RaylibDrawHandle,
    gui: &Tree<UiMessage>,
    font: &Font,
    assets: &Assets,
) {
    for node in gui.iter() {
        draw_node(state, d, node, font, assets);
    }
}

fn server_thread(args: Args) {
    if !args.run_server {
        return;
    }

    let server = match HeadlessServerApp::new(args.save_file, args.server_port, 10) {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to start server: {e:?}");
            return;
        }
    };

    server.spin_forever();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let server_args = args.clone();

    let _join = std::thread::spawn(move || server_thread(server_args));

    info!("{:?}", args);

    let mut main_app = ClientApp::new(args, log::LevelFilter::Info)?;

    let audio = raylib::audio::RaylibAudio::init_audio_device()?;
    let mut active_sounds = Vec::new();

    while !main_app.handle.window_should_close() && !main_app.should_exit {
        if !main_app.update_timer.tick() {
            continue;
        }

        main_app.update();

        // AND DRAW IT ALL

        let font = main_app.assets.consolas.as_ref().unwrap();

        let ui = make_gui(font, &main_app);

        if main_app
            .client
            .input
            .just_pressed_debounced(rdev::Key::KeyP)
        {
            println!("{}", ui);
            println!("{:?}", main_app.ui_state);
        }

        let scrp = main_app.client.mouse_screen_position;

        if let Some(c) = main_app.ui_state.update(&ui, scrp, &main_app.client.input) {
            main_app.on_ui_event(c);
        }

        let font = main_app.assets.consolas.as_ref().unwrap();

        let timers = DebugTimers::default();

        // CONSTRUCT IMMEDIATE-MODE GUI

        main_app
            .handle
            .draw(&main_app.thread, |mut d: RaylibDrawHandle<'_>| {
                d.clear_background(Color::BLACK);

                if matches!(main_app.state, AppState::InWorld) {
                    draw_world(&main_app.world, &main_app.client, &main_app.assets, &mut d);
                }

                draw_gui(&main_app.ui_state, &mut d, &ui, font, &main_app.assets);

                imgui::lame_old_imgui_entrypoint(
                    &mut d,
                    &mut main_app.client,
                    &main_app.world,
                    &mut main_app.sounds,
                );

                draw_mouse_screen_position(&mut d, main_app.client.mouse_screen_position);

                if main_app.show_debug_text {
                    draw_debug_info(
                        &main_app.world,
                        &main_app.client,
                        &main_app.assets,
                        &timers,
                        &main_app.node,
                        &main_app.update_timer,
                        &main_app.server_ping_timer,
                        &main_app.server_telemetry_timer,
                        &main_app.username,
                        main_app.client.player_id,
                        &mut d,
                    );
                }

                draw_terminal(
                    &mut d,
                    &main_app.terminal,
                    &main_app.assets,
                    Color::BLACK.alpha(0.8),
                );
            });

        handle_sounds(&main_app.sounds, &audio, &mut active_sounds);

        main_app.sounds.clear();

        if main_app.client.input.just_pressed(rdev::Key::KeyC)
            && main_app.client.input.is_key_pressed(rdev::Key::ControlLeft)
        {
            break;
        }

        main_app.client.input.on_frame_boundary();
    }

    info!("Done.");

    Ok(())
}
