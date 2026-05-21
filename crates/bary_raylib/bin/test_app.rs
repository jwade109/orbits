use bary_raylib::{
    Client, ClientMessage, ServerMessage, Transaction, cmd::CommandPrompt, utils::BasicApp,
};
use raylib::prelude::*;
use std::collections::BTreeMap;

fn main() {
    let mut app = BasicApp::new("Test app");

    let mut cmd = CommandPrompt::new();

    let mut client = Client::new(127, 0, 0, 1, 5000);

    let mut grids = BTreeMap::new();

    let font = app
        .handle
        .load_font(&app.thread, "assets/fonts/FiraCode-Bold.ttf")
        .unwrap();

    while app.frame() {
        // app.fixed_50_fps(|| info!("FRAME"));

        for msg in client.update() {
            if let ServerMessage::GridPos(name, pos) = msg {
                grids.insert(name.clone(), pos);
            }
        }

        app.handle.draw(&app.thread, |mut d| {
            d.clear_background(Color::BLACK);
            d.draw_text("Test App", 100, 100, 24, Color::WHITE);
            // d.draw_text(&format!("{:#?}", app.input), 100, 150, 24, Color::GRAY);

            for t in app.input.events() {
                if let Some(action) = cmd.on_event(t) {
                    let tr = Transaction::new(0, action);
                    client.send_message(ClientMessage::Transaction(tr));
                }
            }

            d.draw_text(
                &format!("{:#?}", client.stats()),
                500,
                100,
                24,
                Color::WHITE,
            );

            d.draw_text(&cmd.bg_text(), 100, 140, 28, Color::GRAY.alpha(0.4));
            d.draw_text(&cmd.fg_text(), 100, 140, 28, Color::WHITE);

            let mut y = 500;
            let size = 20;

            for (name, pos) in &grids {
                let x = pos.translation.x as i32 + d.get_render_width() / 2;
                let y = pos.translation.y as i32 + d.get_render_height() / 2;
                d.draw_circle(x, y, 3.0, Color::RED);

                d.draw_text_ex(
                    &font,
                    name,
                    Vector2::new(x as f32, y as f32),
                    16.0,
                    0.0,
                    Color::RED.alpha(0.5),
                );
            }

            d.draw_text_ex(
                &font,
                "Server Messages",
                Vector2::new(100.0, y as f32),
                size as f32,
                0.0,
                Color::WHITE.alpha(0.3),
            );
            y += size + 4;
            for (idx, msg) in client.history() {
                let l1 = format!("{}", idx);
                let l2 = format!("{:?}", msg).to_uppercase();

                d.draw_text_ex(
                    &font,
                    &l1,
                    Vector2::new(100.0, y as f32),
                    size as f32,
                    0.0,
                    Color::WHITE.alpha(0.3),
                );
                d.draw_text_ex(
                    &font,
                    &l2,
                    Vector2::new(150.0, y as f32),
                    size as f32,
                    0.0,
                    Color::WHITE.alpha(0.3),
                );
                y += size;
            }
        });
    }
}
