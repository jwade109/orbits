use crate::input_state::InputState;
use crate::multiplayer::*;
use crate::ui::UiState;
use crate::world_builder::WorldBuilder;
use std::thread::JoinHandle;
use std::time::Duration;
use crate::cmd::prompt::CommandPrompt;

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
    pub runner: WorldRunner,
    pub ui_state: UiState,
    pub input: InputState,

    pub _network_thread: JoinHandle<()>,
    pub incoming_network_queue: MessageQueue<ServerMessage>,
    pub outgoing_network_queue: MessageQueue<Transaction>,

    pub _input_thread: JoinHandle<()>,
    pub input_queue: MessageQueue<rdev::Event>,

    pub cmd: CommandPrompt,
}

pub fn new_app(multiplayer: bool) -> App {
    let world = WorldBuilder::new()
        .assets()
        .blueprint("pollux")
        .blueprint("bellerophon")
        .blueprint("remora")
        .blueprint("spacestation")
        .spawn("pollux", (0.0, 0.0, 0.0))
        .spawn("remora", (10.0, 30.0, 0.1))
        .spawn("remora", (-9.0, 12.0, -0.3))
        .spawn("remora", (-7.0, 23.0, 0.7))
        .spawn("bellerophon", (130.0, 50.0, 0.1))
        .waypoint("pollux", (300.0, 200.0, 0.3))
        .waypoint("remora", (3000.0, 7000.0, 0.0))
        .build();

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

    let ui_state = UiState::new();

    // let grid_id = find::grid_by_name(&world.grids, "pollux").unwrap();

    // ui_state.track_grid_info(grid_id);

    App {
        runner: WorldRunner::new(world),
        ui_state,
        input: InputState::default(),
        _network_thread,
        incoming_network_queue,
        outgoing_network_queue,
        _input_thread,
        input_queue,
        cmd: CommandPrompt::new(),
    }
}
