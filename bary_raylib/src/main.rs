use raylib::prelude::*;

pub fn load_parts_from_dir_3(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    root_dir: &'static str,
) -> Vec<Texture2D> {
    let mut ret = Vec::new();

    for file in std::fs::read_dir(root_dir).unwrap() {
        let Ok(file) = file else {
            continue;
        };
        let data_path = file.path().join("skin.png");
        let Ok(t) = rl.load_texture(&thread, &data_path.to_str().unwrap()) else {
            continue;
        };
        ret.push(t);
    }

    ret
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .vsync()
        .build();

    let textures = load_parts_from_dir_3(&mut rl, &thread, &"assets/parts/");

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        d.draw_text("What's up", 12, 12, 24, Color::WHITE);
        d.draw_circle(120, 140, 70.0, Color::REBECCAPURPLE);

        let mut p = 0;

        for t in &textures {
            d.draw_texture(t, p, p, Color::WHITE);
            p += 20;
        }
    }
}
