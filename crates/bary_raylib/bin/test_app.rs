use bary_raylib::{render::draw_terminal, *};
use bary_terminal::*;
use raylib::prelude::*;
use std::collections::BTreeMap;

fn main() {
    let mut app = utils::BasicApp::new("Test app", TraceLogLevel::LOG_INFO);

    let mut terminal = Terminal::with_commands(all_commands());

    let mut client = Client::new(127, 0, 0, 1, 5000);

    let mut server_stats = ServerStatistics::default();

    let mut grids = BTreeMap::new();

    let mut assets = assets::Assets::default();

    assets::load_assets(&mut assets, &mut app.handle, &app.thread);

    while app.frame() {
        for msg in client.update() {
            let s = format!("{:?}", msg);
            let k = if let MessageKind::Text(text) = &msg.kind {
                text.clone()
            } else {
                format!("{:?}", msg.kind)
            };
            let t = format!("[{}] {}", msg.source, k);
            terminal.log_debug(s);

            match msg.level {
                MessageLevel::Command => {
                    terminal.log_info(t);
                }
                MessageLevel::Response => {
                    terminal.log_info(t);
                }
                _ => (),
            }

            if let MessageKind::GridPos(name, pos) = &msg.kind {
                grids.insert(name.clone(), *pos);
            }

            if let MessageKind::ServerStatistics(stats) = msg.kind {
                server_stats = stats;
            }
        }

        let mut should_exit = false;

        for t in app.input.events() {
            if let Some(action) = terminal.on_event(t) {
                match action {
                    Action::Say(s) => {
                        client.send_command(MessageKind::Text(s));
                    }
                    Action::Ping => {
                        client.send_command(MessageKind::Ping);
                    }
                    Action::Clear => {
                        terminal.clear();
                    }
                    Action::SetSimSpeed(speed) => {
                        client.send_message(Message::command(
                            "client",
                            MessageKind::SetSimSpeed(speed),
                        ));
                    }
                    Action::FindGridByName(name) => {
                        client.send_message(Message::command(
                            "client",
                            MessageKind::FindGridByName(name),
                        ));
                    }
                    Action::Exit => {
                        should_exit = true;
                    }
                    Action::ListGrids => {
                        client.send_command(MessageKind::ListGrids);
                    }
                    Action::ListProtos => {
                        client.send_command(MessageKind::ListProtos);
                    }
                    Action::ListParts => {
                        client.send_command(MessageKind::ListParts);
                    }
                    Action::ListThrusters => {
                        client.send_command(MessageKind::ListThrusters);
                    }
                    Action::ListComputers => {
                        client.send_command(MessageKind::ListComputers);
                    }
                    Action::ServerConnect => {
                        client.reconnect();
                    }
                    Action::ServerDisconnect => {
                        client.disconnect();
                    }
                    Action::RequestServerStatistics => {
                        client.send_command(MessageKind::RequestServerStatistics);
                    }
                    Action::SetWaypoint(id, iso) => {
                        client.send_command(MessageKind::SetWaypoint(id, iso));
                    }
                    _ => (),
                }
            }
        }

        if should_exit {
            app.exit();
        }

        let focused = app.handle.is_window_focused();

        app.handle.draw(&app.thread, |mut d| {
            d.clear_background(Color::new(10, 10, 10, 255));

            d.draw_text(
                &format!("{}\n{:#?}", focused, client.stats()),
                100,
                100,
                18,
                Color::WHITE,
            );

            d.draw_text(&format!("{:#?}", server_stats), 600, 100, 18, Color::WHITE);

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
