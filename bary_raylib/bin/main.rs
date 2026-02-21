use bary_raylib::{scenarios::dev_world, world::*};
use crossbeam_queue::SegQueue;
use raylib::prelude::*;
use std::{sync::Arc, thread};

use rdev::listen;

fn draw_debug_info(world: &World, d: &mut RaylibDrawHandle) {
    let mut s = String::new();

    s += &format!("{:?}", d.get_fps());

    s += &format!("\nUpdate: {:?}", world.timers.update);
    s += &format!("\nRender: {:?}", world.timers.render);

    s += &format!("\n{:?}", d.is_cursor_on_screen());
    s += &format!("\n{:?}", d.get_mouse_position());
    s += &format!("\n{:?}", d.get_mouse_delta());
    s += &format!("\nCAM {:?}", &world.camera);
    s += &format!("\nINP {:?}", &world.input);
    s += &format!("\nPRT {:?}", &world.particles.len());
    s += &format!("\nBP {:?}", &world.blueprints);
    s += &format!("\nPART {:?}", &world.prototypes);
    s += &format!("\nGRID {:?}", &world.grids);
    s += &format!("\nTHR {:?}", &world.thrusters);
    s += &format!("\nCPU {:?}", &world.computers);
    s += &format!("\nLGT {:?}", &world.lights);
    s += &format!("\nE {:?}", &world.counter);

    for e in &world.event_queue {
        s += &format!("\n{:?}", e);
    }

    d.draw_text(&s, 12, 12, 20, Color::WHITE);
}

fn main() {
    let os = std::env::consts::OS;
    println!("{}", os);

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

    rl.set_target_fps(144);
    rl.maximize_window();

    let mut shader = rl.load_shader(&thread, None, Some("assets/shaders/distortion.fs"));

    let mut world = dev_world("assets/").unwrap();

    load_assets(&mut world, &mut rl, &thread);

    while !rl.window_should_close() {
        while let Some(e) = input_queue.pop() {
            push_event(&mut world, e);
        }

        let w = rl.get_screen_width();
        let h = rl.get_screen_height();

        update_world(&mut world, Vector2::new(w as f32, h as f32));

        let time = rl.get_time();
        shader.set_shader_value(1, time as f32);

        rl.draw(&thread, |mut d: RaylibDrawHandle<'_>| {
            let start = std::time::Instant::now();
            d.clear_background(Color::BLACK);
            draw_world(&world, &mut d);
            let end = std::time::Instant::now();
            world.timers.render = end - start;
            draw_debug_info(&world, &mut d);
        });
    }
}
