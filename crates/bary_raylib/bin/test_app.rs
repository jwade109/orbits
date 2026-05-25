use bary_ipc::*;
use bary_raylib::{render::draw_terminal, *};
use bary_terminal::*;
use clap::Parser;
use raylib::prelude::*;
use std::collections::BTreeMap;

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    server_addr: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut app = utils::BasicApp::new("Test app", TraceLogLevel::LOG_INFO);
    let mut client = ClientNode::with_str_addr(&args.server_addr)?;
    let mut server_stats = ServerStatistics::default();
    let mut grids = BTreeMap::new();
    let mut assets = assets::Assets::default();

    let mut cmds = all_commands();
    cmds.extend(client_request_blob_commands());

    let mut terminal = Terminal::with_commands(cmds);

    assets::load_assets(&mut assets, &mut app.handle, &app.thread);

    while app.frame() {
        for msg in client.update() {
            let s = format!("{:?}", msg);
            let k = if let MessageKind::Text(text) = &msg.kind {
                text.clone()
            } else {
                format!("{:?}", msg.kind)
            };
            let t = format!("[{:?}] {}", msg.source, k);
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
            if let Some(cmd) = terminal.on_event(t) {
                match cmd {
                    TermCmd::Say(s) => {
                        client.send_command(MessageKind::Text(s));
                    }
                    TermCmd::Ping => {
                        client.send_command(MessageKind::Ping);
                    }
                    TermCmd::Clear => {
                        terminal.clear();
                    }
                    TermCmd::SetSimSpeed(speed) => {
                        client.send_command(MessageKind::SetSimSpeed(speed));
                    }
                    TermCmd::FindGridByName(name) => {
                        client.send_command(MessageKind::FindGridByName(name));
                    }
                    TermCmd::Exit => {
                        should_exit = true;
                    }
                    TermCmd::ListGrids => {
                        client.send_command(MessageKind::ListGrids);
                    }
                    TermCmd::ListProtos => {
                        client.send_command(MessageKind::ListProtos);
                    }
                    TermCmd::ListParts => {
                        client.send_command(MessageKind::ListParts);
                    }
                    TermCmd::ListThrusters => {
                        client.send_command(MessageKind::ListThrusters);
                    }
                    TermCmd::ListComputers => {
                        client.send_command(MessageKind::ListComputers);
                    }
                    TermCmd::ServerConnect => {
                        client.reconnect();
                    }
                    TermCmd::ServerDisconnect => {
                        client.disconnect();
                    }
                    TermCmd::RequestServerStatistics => {
                        client.send_command(MessageKind::RequestServerStatistics);
                    }
                    TermCmd::SetWaypoint(id, iso) => {
                        client.send_command(MessageKind::SetWaypoint(id, iso));
                    }
                    TermCmd::ClientReqBlob(table) => {
                        client.send_command(MessageKind::ClientBlobRequest(table));
                    }
                    _ => terminal.log_error(format!("Unsupported: {:?}", cmd)),
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

    Ok(())
}
