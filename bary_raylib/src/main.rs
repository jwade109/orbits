use crate::world::*;
use bary_core::prelude::*;
use raylib::prelude::*;

mod tests;
mod world;

fn draw_gui(world: &World, d: &mut RaylibDrawHandle) {
    let s = format!(
        "{} FPS\n{:?}\n{:?}\n{:?}\n{:#?}",
        d.get_fps(),
        d.is_cursor_on_screen(),
        d.get_mouse_position(),
        d.get_mouse_delta(),
        &world.spacecraft
    );

    d.clear_background(Color::BLACK);
    d.draw_text(&s, 12, 12, 20, Color::WHITE);
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .vsync()
        .msaa_4x()
        .resizable()
        .build();

    rl.maximize_window();

    let texture = rl
        .load_texture(&thread, "assets/parts/cargo/skin.png")
        .unwrap();

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

    let mut world = World::test_scene(texture);

    while !rl.window_should_close() {
        update_world(&mut world);
        let mut d = rl.begin_drawing(&thread);
        draw_world(&world, &mut d);
        draw_gui(&world, &mut d);

        for (bp, offset) in &bps {
            draw_blueprint(bp, *offset, &mut d);
        }
    }
}
