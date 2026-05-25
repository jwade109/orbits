use bary_input::*;
use bary_ipc::{MessageQueue, new_message_queue};
use raylib::prelude::*;
use std::thread::JoinHandle;

pub trait Application: Sized {
    fn update(&mut self);

    fn draw(&mut self);

    fn should_exit(&self) -> bool;

    fn spin_forever(mut self) {
        while !self.should_exit() {
            self.spin_once();
        }
    }

    fn spin_once(&mut self) {
        self.update();
        self.draw();
    }
}

pub struct BasicApp {
    pub handle: RaylibHandle,
    pub thread: RaylibThread,
    pub input: InputState,
    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,
    should_exit: bool,
}

impl BasicApp {
    pub fn new(title: &str, level: TraceLogLevel) -> Self {
        let (mut handle, thread) = raylib::init()
            .size(1080, 700)
            .title(title)
            .msaa_4x()
            .resizable()
            .log_level(level)
            .build();

        simple_logger::SimpleLogger::new()
            .with_level(log::LevelFilter::Info)
            .env()
            .init()
            .unwrap();

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
            should_exit: false,
        }
    }

    pub fn exit(&mut self) {
        self.should_exit = true;
    }

    pub fn should_loop(&self) -> bool {
        if self.input.is_key_pressed(rdev::Key::ControlLeft)
            && self.input.just_pressed(rdev::Key::KeyC)
        {
            return false;
        }

        !self.should_exit && !self.handle.window_should_close()
    }

    pub fn frame(&mut self) -> bool {
        self.input.on_frame_boundary();
        while let Some(e) = self.input_queue.pop() {
            let focused = self.handle.is_window_focused();
            self.input.process_rdev_event(&e, focused);
        }
        self.should_loop()
    }
}
