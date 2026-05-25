use crate::sim::*;
use crate::sounds::SoundEffects;
use crate::*;
use bary_ipc::*;
use bary_sim::*;
use bary_terminal::Terminal;
use std::thread::JoinHandle;
use std::time::Duration;

fn network_thread(incoming: MessageQueue<Message>, outgoing: MessageQueue<Message>) {
    let mut client = ClientNode::new(127, 0, 0, 1, 5000);
    let dur = Duration::from_millis(50);

    loop {
        let msgs = client.update();
        for msg in msgs {
            incoming.push(msg);
        }

        while let Some(out) = outgoing.pop() {
            client.send_message(out);
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
    pub incoming_network_queue: MessageQueue<Message>,
    pub outgoing_network_queue: MessageQueue<Message>,

    pub _input_thread: JoinHandle<()>,
    pub input_queue: MessageQueue<rdev::Event>,

    pub cmd: Terminal<TermCmd>,
}

impl App {
    pub fn process_event(
        &mut self,
        e: rdev::Event,
        sounds: &mut SoundEffects,
        actions: &mut Vec<TermCmd>,
        on_gui: bool,
    ) {
        self.cmd.on_event(&e);

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

    let world = World::empty();

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
        cmd: Terminal::with_commands(all_commands()),
    }
}
