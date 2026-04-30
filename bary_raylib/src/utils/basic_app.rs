use crate::constants::TICKS_PER_SECOND;
use crate::multiplayer::{MessageQueue, new_message_queue};
use crate::sim::consume_rdev_event_into_input_state;
use crate::utils::InputState;
use raylib::prelude::*;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct BasicApp {
    pub handle: RaylibHandle,
    pub thread: RaylibThread,
    pub input: InputState,
    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,
    pub this_frame: Instant,
    pub last_frame: Instant,

    pub next_fixed: Instant,
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

        let now = Instant::now();

        Self {
            handle,
            thread,
            input,
            input_queue,
            _input_thread,
            this_frame: now,
            last_frame: now,
            next_fixed: now,
        }
    }

    pub fn fixed_50_fps(&mut self, mut func: impl FnMut()) {
        let now = Instant::now();
        if now > self.next_fixed {
            self.next_fixed += Duration::from_millis(1000 / TICKS_PER_SECOND);
            func();
        }
    }

    pub fn frame(&mut self) -> bool {
        let now = Instant::now();
        self.last_frame = self.this_frame;
        self.this_frame = now;
        self.input.on_frame_boundary();
        while let Some(e) = self.input_queue.pop() {
            let focused = self.handle.is_window_focused();
            consume_rdev_event_into_input_state(&mut self.input, &e, focused);
        }

        if self.input.is_key_pressed(rdev::Key::ControlLeft)
            && self.input.just_pressed(rdev::Key::KeyC)
        {
            return false;
        }

        !self.handle.window_should_close()
    }
}
