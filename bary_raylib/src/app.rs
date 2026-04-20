use bary_core::prelude::Vec2;
use raylib::RaylibHandle;

use crate::client::{ClientSpecificInfo, DebugInfo};
use crate::cmd::prompt::{CommandPrompt, cmd_handle_input_event};
use crate::imgui::ImGui;
use crate::multiplayer::*;
use crate::sim::{World, process_event, spawn_stars};
use crate::sounds::SoundEffects;
use crate::world_builder::WorldBuilder;
use std::thread::JoinHandle;
use std::time::Duration;

fn network_thread(incoming: MessageQueue<ServerMessage>, outgoing: MessageQueue<Transaction>) {
    let mut client = Client::new();
    let dur = Duration::from_millis(50);

    loop {
        let msgs = client.update();
        for msg in msgs {
            incoming.push(msg);
        }

        while let Some(out) = outgoing.pop() {
            client.send_message(ClientMessage::Transaction(out));
        }

        std::thread::sleep(dur);
    }
}

pub struct App {
    pub client: ClientSpecificInfo,
    pub world: World,
    pub runner: WorldRunner,
    pub debug: DebugInfo,

    pub _network_thread: JoinHandle<()>,
    pub incoming_network_queue: MessageQueue<ServerMessage>,
    pub outgoing_network_queue: MessageQueue<Transaction>,

    pub _input_thread: JoinHandle<()>,
    pub input_queue: MessageQueue<rdev::Event>,

    pub cmd: CommandPrompt,
}

impl App {
    pub fn process_event(
        &mut self,
        e: rdev::Event,
        sounds: &mut SoundEffects,
        actions: &mut Vec<Action>,
        on_gui: bool,
    ) {
        cmd_handle_input_event(&mut self.cmd, &e);

        if !self.cmd.is_focused() {
            process_event(
                &mut self.world,
                &mut self.client,
                &e,
                sounds,
                actions,
                on_gui,
            );
        }
    }
}

pub fn new_app(multiplayer: bool) -> App {
    use WorldAction::*;

    let mut world = WorldBuilder::new()
        .assets()
        .blueprint(("pollux", 1))
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .blueprint("foundation")
        .blueprint("miner")
        .blueprint("icecream")
        .spawn("pollux", "", (0.0, 0.0, 0.0))
        .spawn("remora", "", (10.0, 30.0, 0.1))
        .spawn("miner", "", (-9.0, 12.0, -0.3))
        .spawn("remora", "", (-7.0, 23.0, 0.7))
        .spawn("bellerophon", "", (130.0, 50.0, 0.1))
        .command(SetSpeed(1))
        .command(Ping(Vec2::ZERO))
        .command(Ping(Vec2::splat(10.0)))
        .build();

    let stars = spawn_stars(&mut world.spawner);
    world.stars = stars;

    let incoming_network_queue = new_message_queue();
    let incoming_network_queue_copy = incoming_network_queue.clone();

    let outgoing_network_queue = new_message_queue();
    let outgoing_network_queue_copy = outgoing_network_queue.clone();

    let _network_thread = std::thread::spawn(move || {
        if multiplayer {
            network_thread(incoming_network_queue_copy, outgoing_network_queue_copy);
        }
    });

    let input_queue = new_message_queue();
    let thread_copy = input_queue.clone();
    let _input_thread = std::thread::spawn(|| {
        if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
            println!("Error: {:?}", error)
        }
    });

    App {
        world,
        client: ClientSpecificInfo::new(),
        runner: WorldRunner::new(),
        debug: DebugInfo::default(),
        _network_thread,
        incoming_network_queue,
        outgoing_network_queue,
        _input_thread,
        input_queue,
        cmd: CommandPrompt::new(),
    }
}
