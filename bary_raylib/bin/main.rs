use bary_core::prelude::*;
use bary_raylib::draw::draw_world;
use bary_raylib::multiplayer::*;
use bary_raylib::systems::*;
use bary_raylib::utils::raylib_to_glam;
use bary_raylib::world::*;
use bary_raylib::world_builder::WorldBuilder;
use log::*;
use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use steamworks::{LobbyChatMsg, LobbyEnter, PersonaStateChange};

use rdev::listen;

fn draw_debug_info(world: &World, assets: &Assets, d: &mut RaylibDrawHandle) {
    let size = size_in_bytes(world);
    let mut s = String::new();

    s += &format!("{:?}", d.get_fps());

    let fmt_time = |d: std::time::Duration, t: std::time::Duration| {
        let p = d.as_secs_f64() / t.as_secs_f64();
        format!(
            "    {:09} ns\n    {:05.04} ms\n    {:3.1}%",
            d.as_nanos(),
            d.as_nanos() as f64 / 1000000.0,
            p * 100.0
        )
    };

    s += &format!("\nU\n{}", fmt_time(world.timers.update, world.timers.total));
    s += &format!("\nR\n{}", fmt_time(world.timers.render, world.timers.total));
    s += &format!("\nT\n{}", fmt_time(world.timers.total, world.timers.total));

    s += &format!("\n{}", world.ticks);
    s += &format!("\nMemory: {:0.3} kb", size as f64 / 1000.0);
    s += &format!("\nZoom: {:0.3}", world.camera.zoom);

    s += &format!("\nMOUSE {:?}", world.mouse_screen_position);
    s += &format!("\nINP {:?}", &world.input);
    s += &format!("\nPRT {:?}", &world.particles.len());
    s += &format!("\nBP {:?}", &world.blueprints);
    s += &format!("\nPROTO {:?}", &world.prototypes);
    s += &format!("\nPART {:?}", &world.parts);
    s += &format!("\nGRID {:?}", &world.grids);
    s += &format!("\nTHR {:?}", &world.thrusters);
    s += &format!("\nCPU {:?}", &world.computers);
    s += &format!("\nLIT {:?}", &world.lights);
    s += &format!("\nE {:?}", &world.spawner);

    for e in &world.event_queue {
        s += &format!("\n{:?}", e);
    }

    if let Some(font) = &assets.lato_regular {
        d.draw_text_ex(&font, &s, Vector2::new(12.0, 12.0), 16.0, 0.0, Color::WHITE);
    }
}

fn network_thread(incoming: MessageQueue<ServerMessage>, outgoing: MessageQueue<Transaction>) {
    let mut client = Client::new();
    let dur = Duration::from_millis(50);

    loop {
        let msgs = client.update();
        for msg in msgs {
            incoming.push(msg);
        }

        while let Some(out) = outgoing.pop() {
            client.send_message(ClientMessage::Transaction(out));
        }

        std::thread::sleep(dur);
    }
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .unwrap();

    let os = std::env::consts::OS;
    println!("{}", os);

    // let client = steamworks::Client::init();

    // if let Ok(client) = &client {
    //     let _cb = client.register_callback(|p: PersonaStateChange| {
    //         println!("Got callback: {:?}", p);
    //     });

    //     let _cb2 = client.register_callback(|p: LobbyChatMsg| {
    //         println!("Got callback: {:?}", p);
    //     });

    //     let _cb3 = client.register_callback(|p: LobbyEnter| {
    //         println!("Got callback: {:?}", p);
    //     });

    //     let mm = client.matchmaking();

    //     mm.create_lobby(steamworks::LobbyType::FriendsOnly, 12, |r| match r {
    //         Ok(id) => {
    //             println!("Created new lobby: {:?}", id);
    //         }
    //         Err(e) => {
    //             println!("Failed to create lobby: {:?}", e);
    //         }
    //     });
    // }

    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .log_level(TraceLogLevel::LOG_INFO)
        .msaa_4x()
        .resizable()
        // .vsync()
        .build();

    let input_queue = new_message_queue();
    let thread_copy = input_queue.clone();
    let _input_thread = thread::spawn(|| {
        if let Err(error) = listen(move |e| thread_copy.push(e)) {
            println!("Error: {:?}", error)
        }
    });

    let incoming_network_queue = new_message_queue();
    let incoming_network_queue_copy = incoming_network_queue.clone();

    let outgoing_network_queue = new_message_queue();
    let outgoing_network_queue_copy = outgoing_network_queue.clone();

    let _network_thread = thread::spawn(|| {
        network_thread(incoming_network_queue_copy, outgoing_network_queue_copy);
    });

    rl.set_target_fps(240);
    // rl.maximize_window();
    rl.set_exit_key(None);

    let mut shader = rl.load_shader(&thread, None, Some("assets/shaders/distortion.fs"));

    let audio = raylib::audio::RaylibAudio::init_audio_device().unwrap();

    let world = WorldBuilder::new()
        .assets("assets/")
        .blueprint("pollux")
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .spawn("pollux", Isometry2d::IDENTITY)
        .spawn("remora", (10.0, 30.0, 0.1))
        .spawn("remora", (-9.0, 12.0, -0.3))
        .spawn("remora", (-7.0, 23.0, 0.7))
        .spawn("bellerophon", (130.0, 50.0, 0.1))
        .build();

    let mut runner = WorldRunner::new(world);

    let mut assets = Assets::default();

    load_assets(&mut assets, &mut rl, &thread);

    rl.hide_cursor();

    let mut active_sounds = Vec::new();

    while !rl.window_should_close() {
        // _ = client.as_ref().map(|c| c.run_callbacks());

        let loop_start = std::time::Instant::now();

        while let Some(e) = input_queue.pop() {
            // process release events even if the window isn't focused!
            let is_release = match e.event_type {
                rdev::EventType::ButtonRelease(_) => true,
                rdev::EventType::KeyRelease(_) => true,
                _ => false,
            };

            if is_release || rl.is_window_focused() {
                push_event(&mut runner.world, e);
            }
        }

        while let Some(n) = incoming_network_queue.pop() {
            if let ServerMessage::Transaction(tr) = n {
                apply_transaction(&mut runner.world, tr);
            }
        }

        let w = rl.get_screen_width();
        let h = rl.get_screen_height();

        let screen_dims = Vec2::new(w as f32, h as f32);

        let mouse = rl
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(rl.get_mouse_position()));

        runner.world.screen_dims = screen_dims;
        runner.world.mouse_screen_position = mouse;

        let (outgoing_messages, sounds) = runner.update();

        for msg in outgoing_messages {
            let transaction = Transaction::new(runner.world.ticks, msg);
            outgoing_network_queue.push(transaction);
        }

        for sound in sounds.effects {
            info!("Sound: {:?}", sound);
            let path = sound.to_path();
            let sound = match audio.new_sound(path) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to load sound: {}", e);
                    continue;
                }
            };

            sound.play();

            active_sounds.push(sound);
        }

        let time = rl.get_time();
        shader.set_shader_value(1, time as f32);

        rl.draw(&thread, |mut d: RaylibDrawHandle<'_>| {
            let start = std::time::Instant::now();
            d.clear_background(Color::BLACK);

            draw_world(&runner.world, &assets, &mut d);

            let end = std::time::Instant::now();
            runner.world.timers.render = end - start;
            runner.world.timers.total = end - loop_start;
            draw_debug_info(&runner.world, &assets, &mut d);
        });
    }
}
