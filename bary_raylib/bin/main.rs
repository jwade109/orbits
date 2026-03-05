use bary_core::prelude::*;
use bary_raylib::app::new_app;
use bary_raylib::draw;
use bary_raylib::multiplayer::*;
use bary_raylib::systems::*;
use bary_raylib::ui;
use bary_raylib::utils::raylib_to_glam;
use bary_raylib::world::*;
use bary_raylib::world_builder::WorldBuilder;
use log::*;
use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use steamworks::{LobbyChatMsg, LobbyEnter, PersonaStateChange};

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

    s += &format!(
        "\n{} {:0.1}",
        world.ticks,
        apparent_elapsed_time(world).as_secs_f64()
    );
    s += &format!("\nMemory: {:0.3} kb", size as f64 / 1000.0);
    s += &format!("\nZoom: {:0.3}", world.camera.zoom);

    s += &format!("\nMOUSE {:?}", world.mouse_screen_position);
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

    // rl.set_target_fps(240);
    // rl.maximize_window();
    rl.set_exit_key(None);

    let mut shader = rl.load_shader(&thread, None, Some("assets/shaders/distortion.fs"));

    let audio = raylib::audio::RaylibAudio::init_audio_device().unwrap();

    let mut app = new_app(false);

    let mut assets = Assets::default();

    load_assets(&mut assets, &mut rl, &thread);

    rl.hide_cursor();

    let mut active_sounds = Vec::new();

    while !rl.window_should_close() {
        // _ = client.as_ref().map(|c| c.run_callbacks());

        let loop_start = std::time::Instant::now();

        while let Some(e) = app.input_queue.pop() {
            // process release events even if the window isn't focused!
            let is_release = match e.event_type {
                rdev::EventType::ButtonRelease(_) => true,
                rdev::EventType::KeyRelease(_) => true,
                _ => false,
            };

            if is_release || rl.is_window_focused() {
                push_event(&mut app.runner.world, e);
            }
        }

        while let Some(n) = app.incoming_network_queue.pop() {
            if let ServerMessage::Transaction(tr) = n {
                apply_transaction(&mut app.runner.world, &mut app.input, tr);
            }
        }

        let w = rl.get_screen_width();
        let h = rl.get_screen_height();

        let screen_dims = Vec2::new(w as f32, h as f32);

        let mouse = rl
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(rl.get_mouse_position()));

        app.runner.world.screen_dims = screen_dims;
        app.runner.world.mouse_screen_position = mouse;

        let (outgoing_messages, sounds) = app.runner.update(&mut app.input);

        for msg in outgoing_messages {
            let transaction = Transaction::new(app.runner.world.ticks, msg);
            app.outgoing_network_queue.push(transaction);
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

            sound.set_volume(0.4);
            sound.play();

            active_sounds.push(sound);
        }

        let time = rl.get_time();
        shader.set_shader_value(1, time as f32);

        ui::update_ui_state(&mut app.ui_state, app.runner.world.mouse_screen_position);

        rl.draw(&thread, |mut d: RaylibDrawHandle<'_>| {
            let start = std::time::Instant::now();
            d.clear_background(Color::BLACK);

            draw::draw_world(&app.runner.world, &assets, &mut d);

            ui::draw_ui(&mut d, &app.runner.world, &app.ui_state, &assets);

            draw::draw_mouse_screen_position(&mut d, app.runner.world.mouse_screen_position);

            let end = std::time::Instant::now();
            app.runner.world.timers.render = end - start;
            app.runner.world.timers.total = end - loop_start;
            draw_debug_info(&app.runner.world, &assets, &mut d);
        });
    }
}
