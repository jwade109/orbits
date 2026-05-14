use crate::client::{ClientSpecificInfo, DebugInfo};
use crate::cmd::{CommandPrompt, cmd_handle_input_event};
use crate::multiplayer::*;
use crate::sim::{World, process_event};
use crate::sounds::SoundEffects;
use crate::world_builder::WorldBuilder;
use bary_core::prelude::*;
use bary_factory::*;
use bary_sim::*;
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
        .blueprint(("pollux", 0))
        .blueprint(("pollux", 2))
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .blueprint("foundation")
        .blueprint("miner")
        .blueprint("icecream")
        .spawn(("pollux", 0), "", (30.0, 0.0, 0.0))
        .spawn(("pollux", 2), "", (0.0, 0.0, 0.0))
        .insert_source((19, 7), Item::Magnesium)
        .insert_source((20, 7), Item::Iron)
        .insert_source((21, 7), Item::Titanium)
        .set_recipe((21, 7), RecipeListing::TitaniumLattice)
        .insert_source((22, 11), Item::Water)
        .insert_pipe((22, 11), (22, 10))
        .set_recipe((27, 7), RecipeListing::WaterElectrolysis)
        .insert_pipe((17, 10), (15, 10))
        .insert_pipe((27, 7), (34, 7))
        .insert_pipe((26, 7), (25, 6))
        .spawn("remora", "", (10.0, 30.0, 0.1))
        .spawn("miner", "", (-9.0, 12.0, -0.3))
        .spawn("remora", "", (-7.0, 23.0, 0.7))
        .spawn("bellerophon", "", (130.0, 50.0, 0.1))
        .command(SetSpeed(10))
        .command(Ping(Vec2::ZERO))
        .command(Ping(Vec2::splat(10.0)))
        .asteroid((-80.0, 30.0, 0.1), 20.0, 391)
        .asteroid((60.0, 300.0, 0.7), 50.0, 2384)
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
