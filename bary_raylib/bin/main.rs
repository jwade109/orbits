use bary_core::prelude::*;
use bary_raylib::world::*;
use raylib::prelude::*;
use std::thread;

use rdev::{Event, listen};

fn callback(event: Event) {
    match event.event_type {
        rdev::EventType::MouseMove { .. } => return,
        _ => (),
    };

    println!("INPUT {:?}", event);
}

fn draw_debug_info(world: &World, d: &mut RaylibDrawHandle, text: &str) {
    let s = format!(
        "{} FPS\n{:?}\n{:?}\n{:?}\n{:?}\n{:#?}",
        d.get_fps(),
        d.is_cursor_on_screen(),
        d.get_mouse_position(),
        d.get_mouse_delta(),
        text,
        &world.spacecraft
    );

    d.draw_text(&s, 12, 12, 20, Color::WHITE);
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .msaa_4x()
        .resizable()
        .vsync()
        .build();

    let _input_thread = thread::spawn(|| {
        if let Err(error) = listen(callback) {
            println!("Error: {:?}", error)
        }
    });

    rl.set_target_fps(60);

    rl.maximize_window();

    let mut shader = rl.load_shader(&thread, None, Some("assets/shaders/distortion.fs"));

    let vehicles = [
        ("pollux", Vec2::new(900.0, 300.0)),
        ("bellerophon", Vec2::new(700.0, 600.0)),
        ("remora", Vec2::new(800.0, 800.0)),
        ("remora", Vec2::new(1400.0, 1100.0)),
        ("spacestation", Vec2::new(1700.0, 800.0)),
    ];

    let mut bps = Vec::new();

    let parts = load_parts_from_dir("assets/parts/").expect("Parts dir");

    for (v, pos) in vehicles {
        let path = format!("assets/vehicles/{}.vehicle", v);
        let bp = load_vehicle(path, &parts).expect("Vehicle dir");
        bps.push((bp, pos));
    }

    let mut world = World::test_scene();

    let text = String::new();

    while !rl.window_should_close() {
        while let Some(x) = rl.get_char_pressed() {
            dbg!(x);
        }

        update_world(&mut world);

        let time = rl.get_time();
        shader.set_shader_value(1, time as f32);

        rl.draw(&thread, |mut d: RaylibDrawHandle<'_>| {
            d.clear_background(Color::BLACK);
            d.draw_shader_mode(&mut shader, |mut d| {
                draw_world(&world, &mut d);

                d.draw_rectangle(100, 100, 300, 200, Color::RED);

                for (bp, offset) in &bps {
                    draw_blueprint(bp, *offset, &mut d);
                }
            });
            draw_debug_info(&world, &mut d, &text);
        });
    }
}
