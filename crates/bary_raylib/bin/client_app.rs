use std::thread::JoinHandle;
use std::time::Duration;

use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::*;
use bary_raylib::assets::*;
use bary_raylib::headless_server::HeadlessServerApp;
use bary_raylib::imgui;
use bary_raylib::render::*;
use bary_raylib::sim::*;
use bary_raylib::sounds::SoundEffects;
use bary_raylib::utils::ActionSet;
use bary_raylib::utils::Application;
use bary_raylib::utils::glam_to_raylib;
use bary_raylib::utils::raylib_to_glam;
use bary_raylib::*;
use bary_sim::*;
use bary_terminal::Terminal;
use bary_ui::*;
use clap::Parser;
use log::*;
use raylib::prelude::*;
use serde::Deserialize;

pub struct ClientApp {
    client: ClientSpecificInfo,
    world: World,
    #[allow(unused)]
    debug: DebugInfo,

    incoming_network_queue: MessageQueue<Message>,

    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,

    terminal: Terminal<TermCmd>,

    handle: RaylibHandle,
    thread: RaylibThread,
    assets: Assets,
    node: ClientNode,

    update_timer: WallTimer,
    server_ping_timer: WallTimer,
    server_telemetry_timer: WallTimer,

    should_exit: bool,
}

impl ClientApp {
    fn new(args: Args, log_level: log::LevelFilter) -> Result<Self, Box<dyn std::error::Error>> {
        let incoming_network_queue = new_message_queue();

        let input_queue = new_message_queue();
        let thread_copy = input_queue.clone();
        let _input_thread = std::thread::spawn(|| {
            if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
                println!("Error: {:?}", error)
            }
        });

        let world = load_world(&args.save_file)?;

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
        let server_telemetry_timer = WallTimer::with_dur(Duration::from_millis(1000));

        Ok(ClientApp {
            world,
            client: ClientSpecificInfo::new(),
            debug: DebugInfo::default(),
            incoming_network_queue,
            _input_thread,
            input_queue,
            terminal,
            handle,
            thread,
            assets,
            node,
            update_timer,
            server_ping_timer,
            server_telemetry_timer,
            should_exit: false,
        })
    }

    fn update(&mut self) {
        // HANDLE MESSAGES FROM NETWORK NODE

        for msg in self.node.update() {
            self.on_rcv_server_msg(msg);
        }

        // HANDLE INPUTS FROM RDEV LISTENER THREAD

        while let Some(e) = self.input_queue.pop() {
            let focused = self.handle.is_window_focused();
            self.client.input.process_rdev_event(&e, focused);
        }

        if self.server_ping_timer.tick() {
            self.node.send_telemetry(MessageKind::Ping)
        }

        if self.server_telemetry_timer.tick() {
            let tlm = ClientTelemetry {
                ticks: self.world.ticks,
                grid_transforms: self
                    .world
                    .grids
                    .iter()
                    .map(|(id, grid)| (*id, grid.name.clone(), grid.particle_location))
                    .collect(),
            };
            self.node.send_telemetry(MessageKind::ClientTelemetry(tlm));
        }

        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        self.client.mouse_screen_position = self
            .handle
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(self.handle.get_mouse_position()));
        self.client.screen_dims = Vec2::new(
            self.handle.get_screen_width() as f32,
            self.handle.get_screen_height() as f32,
        );

        // GET COMMANDS FROM THE MULTIPLAYER SERVER

        while let Some(n) = self.incoming_network_queue.pop() {
            info!("Got message: {n:?}")
        }
    }

    pub fn process_event(&mut self, sounds: &mut SoundEffects, on_gui: bool) {
        if !self.terminal.is_active() {
            process_event(&mut self.world, &mut self.client, sounds, on_gui);
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
            (MessageLevel::Telemetry, MessageKind::Driver { ticks, deltas }) => {
                self.on_driver_packet(ticks, deltas);
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
            _ => (),
        }
    }

    fn on_driver_packet(&mut self, ticks: u64, deltas: Vec<WorldDelta>) {
        while self.world.ticks + 1 < ticks {
            update_world(&mut self.world);
        }

        for delta in deltas {
            if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
                error!("Failed to apply delta {:?}: {:?}", delta, e);
            }
        }

        update_world(&mut self.world);
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
        client.tick_rate,
        world.ticks,
        apparent_elapsed_time(world.ticks).as_secs_f64(),
        apparent_datetime(world.ticks).format("%b %d %Y %I:%M:%S %p"),
    );

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
    #[arg(short, default_value = "saves/scenario_a")]
    save_file: String,
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

fn draw_gui<T>(d: &mut RaylibDrawHandle, gui: &Tree<T>, mouse_pos: Option<Vec2>, assets: &Assets) {
    let Some(font) = assets.consolas.as_ref() else {
        return;
    };

    for root in gui.layouts() {
        draw_ui_aabb(d, root.aabb(), Color::ORANGE.alpha(0.5), false);
        for node in root.iter() {
            if !node.is_visible() {
                continue;
            }

            let aabb = node.aabb();

            let [r, g, b, a] = node.color_u8();
            let color = Color::new(r, g, b, a);

            let is_hovered = mouse_pos.map(|p| aabb.contains(p)).unwrap_or(false);
            let color = if is_hovered {
                color.lerp(Color::WHITE, 0.5)
            } else {
                color
            };

            if node.is_leaf() {
                draw_ui_aabb(d, node.aabb(), color, true);
            } else {
                draw_ui_aabb(d, node.aabb(), Color::new(20, 20, 20, 230), true);
            }

            if let Some(text) = node.text_content() {
                let p = glam_to_raylib(aabb.center);
                draw_text_centered(d, font, text, p, 16, Color::WHITE);
            }
        }
    }
}

fn server_thread(args: Args) {
    if !args.run_server {
        return;
    }

    let server = match HeadlessServerApp::new(&args.save_file, args.server_port, 10) {
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

        // RUN PRE-PHYSICS, PHYSICS, AND POST-PHYSICS UPDATES

        let mut sounds = SoundEffects::new();

        let deltas = pre_simulation_update(&main_app.world, &mut main_app.client);

        let post_delta = post_simulation_update(
            &main_app.world,
            &mut main_app.client,
            &mut sounds,
            main_app.terminal.is_focused(),
        );

        for delta in deltas.into_iter().chain(post_delta) {
            main_app.node.send_command(MessageKind::RequestDelta(delta));
        }

        let mut timers = DebugTimers::default();

        // CONSTRUCT IMMEDIATE-MODE GUI

        let gui = {
            let _timer = timers.scope("imgui");

            imgui::imgui_pass(&mut main_app.client, &main_app.world, &mut sounds)
        };

        // HANDLE RDEV EVENTS (DEPRECATED - USE INPUTSTATE)

        let cmds = main_app.terminal.handle_input(&main_app.client.input);

        for cmd in cmds {
            main_app.on_terminal_cmd(cmd);
        }

        main_app.process_event(&mut sounds, gui.is_hovering_gui());

        // AND DRAW IT ALL

        main_app
            .handle
            .draw(&main_app.thread, |mut d: RaylibDrawHandle<'_>| {
                d.clear_background(Color::BLACK);

                draw_world(
                    &main_app.world,
                    &main_app.client,
                    &main_app.assets,
                    &gui,
                    &mut d,
                );

                if main_app.client.input.is_key_pressed(rdev::Key::BackSlash) {
                    let gui = generate_assets_ui(
                        &main_app.assets,
                        &main_app.client.input,
                        d.get_render_width() as f32,
                        d.get_render_height() as f32,
                    );

                    draw_gui(
                        &mut d,
                        &gui,
                        main_app.client.mouse_screen_position,
                        &main_app.assets,
                    );
                }

                imgui::lame_old_imgui_entrypoint(
                    &mut d,
                    &mut main_app.client,
                    &main_app.world,
                    &mut sounds,
                    &main_app.assets,
                );

                draw_mouse_screen_position(&mut d, main_app.client.mouse_screen_position);

                draw_debug_info(
                    &main_app.world,
                    &main_app.client,
                    &main_app.assets,
                    &timers,
                    &main_app.node,
                    &main_app.update_timer,
                    &main_app.server_ping_timer,
                    &main_app.server_telemetry_timer,
                    &mut d,
                );

                draw_terminal(
                    &mut d,
                    &main_app.terminal,
                    &main_app.assets,
                    Color::BLACK.alpha(0.8),
                );
            });

        handle_sounds(sounds, &audio, &mut active_sounds);

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
