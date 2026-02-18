use bary_raylib::{scenarios::dev_world, world::*};
use crossbeam_queue::SegQueue;
use raylib::prelude::*;
use std::{sync::Arc, thread};

use rdev::listen;

fn draw_debug_info(world: &World, d: &mut RaylibDrawHandle, text: &str) {
    let mut s = format!(
        "{} FPS\n{:?}\n{:?}\n{:?}\n{:?}\nSnapping: {}\n{:#?}\n{:#?}\nParticles: {:#?}\nBlueprints: {:#?}\nParts: {:#?}\nGrids: {:#?}\nThrusters: {:#?}\nComputers: {:#?}",
        d.get_fps(),
        d.is_cursor_on_screen(),
        d.get_mouse_position(),
        d.get_mouse_delta(),
        text,
        &world.snap_camera_to_local_planet,
        &world.camera,
        &world.input,
        &world.ring_particles,
        &world.blueprints,
        &world.prototypes,
        &world.grids,
        &world.thrusters,
        &world.computers,
    );

    for g in world.grids.values() {
        s += &format!("\n- {} {} {:?}", g.mass, g.parts.len(), g.isometry);
    }

    for e in &world.event_queue {
        s += &format!("\n{:?}", e);
    }

    d.draw_text(&s, 12, 12, 20, Color::WHITE);
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .log_level(TraceLogLevel::LOG_WARNING)
        .msaa_4x()
        .resizable()
        .vsync()
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

    let mut world = dev_world("assets/");

    load_assets(&mut world, &mut rl, &thread);

    let text = String::new();

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
            d.clear_background(Color::BLACK);

            d.draw_mode2D(world.camera, |mut d, _camera| {
                // d.draw_shader_mode(&mut shader, |mut d| {
                draw_world(&world, &mut d);
                // });
            });

            draw_debug_info(&world, &mut d, &text);
        });
    }
}
