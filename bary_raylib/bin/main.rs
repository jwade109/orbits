use bary_raylib::{scenarios::dev_world, world::*};
use crossbeam_queue::SegQueue;
use raylib::prelude::*;
use std::{sync::Arc, thread};
use steamworks::{Client, PersonaStateChange};

use rdev::listen;

fn draw_debug_info(world: &World, d: &mut RaylibDrawHandle) {
    let mut s = String::new();

    s += &format!("{:?}", d.get_fps());

    let fmt_time = |d: std::time::Duration| {
        format!(
            "    {:09} ns\n    {:05.04} ms",
            d.as_nanos(),
            d.as_nanos() as f64 / 1000000.0
        )
    };

    s += &format!("\nU\n{}", fmt_time(world.timers.update));
    s += &format!("\nR\n{}", fmt_time(world.timers.render));
    s += &format!("\nT\n{}", fmt_time(world.timers.total));

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
    s += &format!("\nUP {}", &world.grids_to_update.len());
    s += &format!("\nE {:?}", &world.spawner);

    for e in &world.event_queue {
        s += &format!("\n{:?}", e);
    }

    if let Some(font) = &world.lato_regular {
        d.draw_text_ex(&font, &s, Vector2::new(12.0, 12.0), 20.0, 0.0, Color::WHITE);
    }
}

fn main() {
    let os = std::env::consts::OS;
    println!("{}", os);

    let client = Client::init().unwrap();

    let _cb = client.register_callback(|p: PersonaStateChange| {
        println!("Got callback: {:?}", p);
    });

    let mm = client.matchmaking();

    mm.create_lobby(steamworks::LobbyType::FriendsOnly, 12, |r| match r {
        Ok(id) => {
            println!("Created new lobby: {:?}", id);
        }
        Err(e) => {
            println!("Failed to create lobby: {:?}", e);
        }
    });

    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .log_level(TraceLogLevel::LOG_WARNING)
        .msaa_4x()
        .resizable()
        // .vsync()
        .build();

    let input_queue = Arc::new(SegQueue::new());

    let thread_copy = input_queue.clone();

    let _input_thread = thread::spawn(|| {
        if let Err(error) = listen(move |e| thread_copy.push(e)) {
            println!("Error: {:?}", error)
        }
    });

    rl.set_target_fps(60);
    rl.maximize_window();
    rl.set_exit_key(None);

    let mut shader = rl.load_shader(&thread, None, Some("assets/shaders/distortion.fs"));

    let mut world = dev_world("assets/").unwrap();

    load_assets(&mut world, &mut rl, &thread);

    rl.hide_cursor();

    while !rl.window_should_close() {
        client.run_callbacks();

        let loop_start = std::time::Instant::now();

        while let Some(e) = input_queue.pop() {
            push_event(&mut world, e);
        }

        let w = rl.get_screen_width();
        let h = rl.get_screen_height();

        let screen_dims = Vector2::new(w as f32, h as f32);

        let mouse = rl.is_cursor_on_screen().then(|| rl.get_mouse_position());

        update_world(&mut world, screen_dims, mouse);

        // let time = rl.get_time();
        // shader.set_shader_value(1, time as f32);

        rl.draw(&thread, |mut d: RaylibDrawHandle<'_>| {
            let start = std::time::Instant::now();
            d.clear_background(Color::BLACK);

            draw_world(&world, &mut d);

            let end = std::time::Instant::now();
            world.timers.render = end - start;
            world.timers.total = end - loop_start;
            draw_debug_info(&world, &mut d);
        });
    }
}
