use bary_raylib::{render::draw_terminal, *};
use bary_terminal::*;
use raylib::prelude::*;
use std::collections::BTreeMap;

fn main() {
    let mut app = utils::BasicApp::new("Test app");

    let mut terminal = Terminal::with_commands(all_commands());

    let mut client = Client::new(127, 0, 0, 1, 5000);

    let mut grids = BTreeMap::new();

    let mut assets = assets::Assets::default();

    assets::load_assets(&mut assets, &mut app.handle, &app.thread);

    while app.frame() {
        // app.fixed_50_fps(|| info!("FRAME"));

        for msg in client.update() {
            let s = format!("{:?}", msg);
            terminal.log_debug(s);
            if let ServerMessage::GridPos(name, pos) = msg {
                grids.insert(name.clone(), pos);
            }
        }

        app.handle.draw(&app.thread, |mut d| {
            d.clear_background(Color::new(10, 10, 10, 255));

            for t in app.input.events() {
                if let Some(action) = terminal.on_event(t) {
                    let tr = Transaction::new(0, action);
                    client.send_message(ClientMessage::Transaction(tr));
                }
            }

            d.draw_text(
                &format!("{:#?}", client.stats()),
                100,
                100,
                24,
                Color::WHITE,
            );

            for (name, pos) in &grids {
                let x = pos.translation.x as i32 + d.get_render_width() / 2;
                let y = pos.translation.y as i32 + d.get_render_height() / 2;
                d.draw_circle(x, y, 3.0, Color::RED);

                d.draw_text_ex(
                    assets.lato_regular.as_ref().unwrap(),
                    name,
                    Vector2::new(x as f32, y as f32),
                    16.0,
                    0.0,
                    Color::RED.alpha(0.5),
                );
            }

            draw_terminal(&mut d, &terminal, &assets);
        });
    }
}
