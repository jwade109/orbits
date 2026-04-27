use crate::multiplayer::{MessageQueue, new_message_queue};
use crate::sim::consume_rdev_event_into_input_state;
use crate::utils::InputState;
use raylib::prelude::*;
use std::thread::JoinHandle;

pub struct BasicApp {
    pub handle: RaylibHandle,
    pub thread: RaylibThread,
    pub input: InputState,
    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,
}

impl BasicApp {
    pub fn new(title: &str) -> Self {
        let (mut handle, thread) = raylib::init()
            .size(1080, 700)
            .title(title)
            .msaa_4x()
            .resizable()
            .build();

        handle.set_target_fps(120);
        handle.maximize_window();
        handle.set_exit_key(None);

        let input = InputState::default();

        let input_queue = new_message_queue();
        let thread_copy = input_queue.clone();
        let _input_thread = std::thread::spawn(|| {
            if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
                println!("Error: {:?}", error)
            }
        });

        Self {
            handle,
            thread,
            input,
            input_queue,
            _input_thread,
        }
    }

    pub fn update_inputs(&mut self) {
        self.input.on_frame_boundary();
        let mut rdev_events = Vec::new();
        while let Some(e) = self.input_queue.pop() {
            let focused = self.handle.is_window_focused();
            consume_rdev_event_into_input_state(&mut self.input, &e, focused);
            rdev_events.push(e);
        }
    }
}
