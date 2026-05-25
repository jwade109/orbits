use std::thread::JoinHandle;
use std::time::Duration;

use bary_core::prelude::*;
use bary_ipc::*;
use bary_raylib::TermCmd;
use bary_raylib::WorldRunner;
use bary_raylib::assets::*;
use bary_raylib::imgui;
use bary_raylib::render::*;
use bary_raylib::sim::*;
use bary_raylib::sounds::SoundEffects;
use bary_raylib::tests::is_world_consistent;
use bary_raylib::utils::WallTimer;
use bary_raylib::utils::raylib_to_glam;
use bary_raylib::*;
use bary_sim::DebugInfo;
use bary_sim::WorldDelta;
use bary_terminal::Terminal;
use clap::Parser;
use log::*;
use raylib::prelude::*;
use serde::Deserialize;

pub struct App {
    client: ClientSpecificInfo,
    world: World,
    runner: WorldRunner,
    debug: DebugInfo,

    incoming_network_queue: MessageQueue<Message>,

    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,

    terminal: Terminal<TermCmd>,

    handle: RaylibHandle,
    thread: RaylibThread,
    assets: Assets,
    node: ClientNode,
    server_ping_wall_timer: WallTimer,

    should_exit: bool,
}

impl App {
    fn new(
        server_addr: &str,
        log_level: log::LevelFilter,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let incoming_network_queue = new_message_queue();

        let input_queue = new_message_queue();
        let thread_copy = input_queue.clone();
        let _input_thread = std::thread::spawn(|| {
            if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
                println!("Error: {:?}", error)
            }
        });

        let world = World::empty();

        let mut cmds = all_commands();
        cmds.extend(client_request_blob_commands());

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

        handle.set_target_fps(120);
        handle.maximize_window();
        handle.set_exit_key(None);
        handle.hide_cursor();

        let mut assets = Assets::default();
        load_assets(&mut assets, &mut handle, &thread);

        let node = ClientNode::with_str_addr(server_addr)?;

        let server_ping_wall_timer = WallTimer::with_dur(Duration::from_millis(500));

        Ok(App {
            world,
            client: ClientSpecificInfo::new(),
            runner: WorldRunner::new(),
            debug: DebugInfo::default(),
            incoming_network_queue,
            _input_thread,
            input_queue,
            terminal: Terminal::with_commands(cmds),
            handle,
            thread,
            assets,
            node,
            server_ping_wall_timer,
            should_exit: false,
        })
    }

    #[must_use]
    pub fn process_event(
        &mut self,
        e: rdev::Event,
        sounds: &mut SoundEffects,
        actions: &mut Vec<TermCmd>,
        on_gui: bool,
    ) -> Option<TermCmd> {
        let cmd = self.terminal.on_event(&e);

        if !self.terminal.is_focused() {
            process_event(
                &mut self.world,
                &mut self.client,
                &e,
                sounds,
                actions,
                on_gui,
            );
        }

        cmd
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
            (MessageLevel::Response, MessageKind::BlobResponse(blob)) => {
                self.on_rcv_blob(blob);
            }
            (MessageLevel::Response, MessageKind::MultiBlobResponse(blobs)) => {
                for blob in blobs {
                    self.on_rcv_blob(blob);
                }
            }
            _ => (),
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

    fn apply_world_delta(&mut self, delta: WorldDelta) {
        let s = format!("{delta:?} => OK");
        match self.world.apply(delta) {
            Ok(()) => self.terminal.log_info(s),
            Err(e) => self.terminal.log_error(format!("Failed to apply: {e:?}")),
        }
    }

    pub fn on_terminal_cmd(&mut self, cmd: TermCmd) {
        self.terminal.log_info(format!("{cmd:?}"));

        match cmd {
            TermCmd::World(delta) => {
                self.apply_world_delta(delta);
            }
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
                self.node.send_command(MessageKind::SetSimSpeed(speed));
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
                self.node.send_command(MessageKind::ClientBlobRequestAll);
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
    d: &mut RaylibDrawHandle,
) {
    let size = size_in_bytes(world);
    let mut s = "Barycenter Client".to_string();

    // let consist = is_world_consistent(world);
    // s += &format!("\nOK: {:?}", consist.is_ok());

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
        world.tick_rate,
        world.ticks,
        apparent_elapsed_time(world).as_secs_f64(),
        apparent_datetime(world).format("%b %d %Y %I:%M:%S %p"),
    );

    s += &format!("\nFPS:       {}", d.get_fps());
    s += &format!("\nMemory:    {:0.3} KB", size as f64 / 1000.0);
    s += &format!("\nZoom:      {:0.3}", client.camera.zoom);

    s += "\n";

    s += &format!("\nConnected: {}", node.is_connected());
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

    s += "\n";

    if let Some(free) = client.viewport.free() {
        s += &format!("\nTerrain: {:?}", free.hovered_chunk);
    }

    let total = timers.total();

    s += &format!("\ntotal\n{}", fmt_time(total, total));

    for timer in timers.timers.iter() {
        let time = fmt_time(*timer.1, total);
        s += &format!("\n{}\n{}", timer.0, time);
    }

    d.draw_text_ex(
        assets.consolas.as_ref().unwrap(),
        &s,
        Vector2::new(10.0, 10.0),
        16.0,
        0.0,
        Color::ORANGE,
    );
}

fn handle_sounds<'a>(
    sounds: SoundEffects,
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
    server_addr: String,
}

struct MainApp {
    app: App,
}

impl MainApp {
    fn new(server_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            app: App::new(server_addr, log::LevelFilter::Info)?,
        })
    }

    fn update(&mut self) {
        // HANDLE MESSAGES FROM NETWORK NODE

        for msg in self.app.node.update() {
            self.app.on_rcv_server_msg(msg);
        }

        // HANDLE INPUTS FROM RDEV LISTENER THREAD

        while let Some(e) = self.app.input_queue.pop() {
            let focused = self.app.handle.is_window_focused();
            self.app.client.input.process_rdev_event(&e, focused);
        }

        if self.app.server_ping_wall_timer.tick() {
            self.app.node.send_telemetry(MessageKind::Ping)
        }

        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        self.app.client.mouse_screen_position = self
            .app
            .handle
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(self.app.handle.get_mouse_position()));
        self.app.client.screen_dims = Vec2::new(
            self.app.handle.get_screen_width() as f32,
            self.app.handle.get_screen_height() as f32,
        );

        // GET COMMANDS FROM THE MULTIPLAYER SERVER

        while let Some(n) = self.app.incoming_network_queue.pop() {
            info!("Got message: {n:?}")
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut main_app = MainApp::new(&args.server_addr)?;

    let audio = raylib::audio::RaylibAudio::init_audio_device()?;
    let mut active_sounds = Vec::new();

    while !main_app.app.handle.window_should_close() && !main_app.app.should_exit {
        main_app.update();

        // RUN PRE-PHYSICS, PHYSICS, AND POST-PHYSICS UPDATES

        let mut sounds = SoundEffects::new();
        let mut actions = Vec::new();

        let mut timers = main_app.app.runner.update(
            &mut main_app.app.world,
            &mut main_app.app.client,
            &mut sounds,
            &mut actions,
        );

        // CONSTRUCT IMMEDIATE-MODE GUI

        let gui = {
            let _timer = timers.scope("imgui");

            imgui::imgui_pass(
                &mut main_app.app.client,
                &mut main_app.app.world,
                &mut sounds,
            )
        };

        // HANDLE RDEV EVENTS (DEPRECATED - USE INPUTSTATE)

        let events: Vec<_> = main_app.app.client.input.events().cloned().collect();
        for e in events {
            let cmd = main_app.app.process_event(
                e.clone(),
                &mut sounds,
                &mut actions,
                gui.is_hovering_gui(),
            );

            if let Some(cmd) = cmd {
                main_app.app.on_terminal_cmd(cmd);
            }
        }

        // EMIT ACTIONS TO OTHER MULTIPLAYER CLIENTS

        for msg in actions {
            warn!("Would emit action: {msg:?}");
        }

        // AND DRAW IT ALL

        main_app
            .app
            .handle
            .draw(&main_app.app.thread, |mut d: RaylibDrawHandle<'_>| {
                d.clear_background(Color::BLACK);

                draw_world(
                    &main_app.app.world,
                    &main_app.app.client,
                    &main_app.app.assets,
                    &gui,
                    &mut d,
                );

                imgui::lame_old_imgui_entrypoint(
                    &mut d,
                    &mut main_app.app.client,
                    &mut main_app.app.world,
                    &main_app.app.terminal,
                    &mut sounds,
                    &main_app.app.assets,
                );

                draw_mouse_screen_position(&mut d, main_app.app.client.mouse_screen_position);

                draw_debug_info(
                    &main_app.app.world,
                    &main_app.app.client,
                    &main_app.app.assets,
                    &timers,
                    &main_app.app.node,
                    &mut d,
                );
            });

        handle_sounds(sounds, &audio, &mut active_sounds);

        if main_app.app.client.input.just_pressed(rdev::Key::KeyC)
            && main_app
                .app
                .client
                .input
                .is_key_pressed(rdev::Key::ControlLeft)
        {
            break;
        }

        main_app.app.client.input.on_frame_boundary();
    }

    info!("Done.");

    Ok(())
}
