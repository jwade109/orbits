use bary_core::prelude::*;
use bary_raylib::app::*;
use bary_raylib::assets::*;
use bary_raylib::imgui;
use bary_raylib::render::*;
use bary_raylib::sim::*;
use bary_raylib::sounds::SoundEffects;
use bary_raylib::tests::is_world_consistent;
use bary_raylib::utils::raylib_to_glam;
use bary_raylib::*;
use log::*;
use raylib::prelude::*;

// use std::thread;
// use std::time::Duration;
// use steamworks::{LobbyChatMsg, LobbyEnter, PersonaStateChange};

fn draw_debug_info(app: &App, assets: &Assets, timers: &DebugTimers, d: &mut RaylibDrawHandle) {
    let world = &app.world;
    let client = &app.client;

    let size = size_in_bytes(world);
    let mut s = String::new();

    let consist = is_world_consistent(world);
    s += &format!("\nOK: {consist:?}");

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

    s += &format!("\nC {} {} fps", client.ticks, d.get_fps());
    s += &format!("\nMemory: {:0.3} kb", size as f64 / 1000.0);
    s += &format!("\nZoom: {:0.3}", client.camera.zoom);
    s += &format!("\nUpdates: {}", world.grid_acceleration_updates);
    s += &format!("\nPipes: {}", world.pipes.len());

    if let Some(free) = client.viewport.free() {
        s += &format!("\nTerrain: {:?}", free.hovered_chunk);
    }

    let total = timers.total();

    s += &format!("\ntotal\n{}", fmt_time(total, total));

    for timer in timers.timers.iter() {
        let time = fmt_time(*timer.1, total);
        s += &format!("\n{}\n{}", timer.0, time);
    }

    // s += &format!("\nMOUSE {:?}", client.mouse_screen_position);
    // s += &format!("\nHOVER {:?}", client.selection_info.hovered);
    // s += &format!("\nSLCT {:?}", client.selection_info.selected);
    // s += &format!("\nPRT {:?}", &world.particles.len());
    // s += &format!("\nBP {:?}", &world.blueprints);
    // s += &format!("\nPROTO {:?}", &world.prototypes);
    // s += &format!("\nPART {:?}", &world.parts);
    // s += &format!("\nGRID {:?}", &world.grids);
    // s += &format!("\nTHR {:?}", &world.thrusters);
    // s += &format!("\nCPU {:?}", &world.computers);
    // s += &format!("\nLIT {:?}", &world.lights);
    // s += &format!("\nE {:?}", &world.spawner);

    if let Some(font) = &assets.lato_regular {
        d.draw_text_ex(
            &font,
            &s,
            Vector2::new(12.0, 12.0),
            16.0,
            0.0,
            Color::WHITE.alpha(0.4),
        );
    }
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

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .unwrap();

    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .log_level(TraceLogLevel::LOG_WARNING)
        .msaa_4x()
        .resizable()
        .build();

    rl.set_target_fps(120);
    rl.maximize_window();
    rl.set_exit_key(None);
    rl.hide_cursor();

    let audio = raylib::audio::RaylibAudio::init_audio_device().unwrap();

    let mut app = new_app(false);

    let mut assets = Assets::default();

    load_assets(&mut assets, &mut rl, &thread);

    let mut active_sounds = Vec::new();

    while !rl.window_should_close() {
        // HANDLE INPUTS FROM RDEV LISTENER THREAD

        let mut rdev_events = Vec::new();
        while let Some(e) = app.input_queue.pop() {
            let focused = rl.is_window_focused();
            app.client.input.process_rdev_event(&e, focused);
            rdev_events.push(e);
        }

        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        app.client.mouse_screen_position = rl
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(rl.get_mouse_position()));
        app.client.screen_dims =
            Vec2::new(rl.get_screen_width() as f32, rl.get_screen_height() as f32);

        // GET COMMANDS FROM THE MULTIPLAYER SERVER

        while let Some(n) = app.incoming_network_queue.pop() {
            if let ServerMessage::Transaction(tr) = n {
                apply_action(&mut app.world, tr.action);
            }
        }

        // RUN PRE-PHYSICS, PHYSICS, AND POST-PHYSICS UPDATES

        let mut sounds = SoundEffects::new();
        let mut actions = Vec::new();

        let mut timers =
            app.runner
                .update(&mut app.world, &mut app.client, &mut sounds, &mut actions);

        // CONSTRUCT IMMEDIATE-MODE GUI

        let gui = {
            let _timer = timers.scope("imgui");

            imgui::imgui_pass(&mut app.client, &mut app.world, &mut sounds)
        };

        // HANDLE RDEV EVENTS (DEPRECATED - USE INPUTSTATE)

        for e in rdev_events {
            app.process_event(e, &mut sounds, &mut actions, gui.is_hovering_gui());
        }

        // EMIT ACTIONS TO OTHER MULTIPLAYER CLIENTS

        for msg in actions {
            let transaction = Transaction::new(app.world.ticks, msg);
            app.outgoing_network_queue.push(transaction);
        }

        // AND DRAW IT ALL

        rl.draw(&thread, |mut d: RaylibDrawHandle<'_>| {
            d.clear_background(Color::BLACK);

            draw_world(&app.world, &app.client, &assets, &gui, &mut d);

            imgui::lame_old_imgui_entrypoint(&mut d, &mut app, &mut sounds, &assets);

            draw_mouse_screen_position(&mut d, app.client.mouse_screen_position);

            draw_debug_info(&app, &assets, &timers, &mut d);
        });

        handle_sounds(sounds, &audio, &mut active_sounds);

        if app.client.input.just_pressed(rdev::Key::KeyC)
            && app.client.input.is_key_pressed(rdev::Key::ControlLeft)
        {
            break;
        }

        app.client.input.on_frame_boundary();
    }

    info!("Done.");
}
