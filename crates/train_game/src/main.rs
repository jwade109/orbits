use crate::draw::draw_world;
use crate::event_bus::EventBus;
use crate::rend_app::*;
use crate::tweens::AnimationStates;
use crate::world::*;
use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::{MessageQueue, new_message_queue};
use glfw::*;
use rend::*;
use std::{collections::BTreeMap, thread::JoinHandle, time::Instant};

mod persistence;
mod bezier;
mod draw;
mod event_bus;
mod railcar;
mod rend_app;
mod track;
mod tweens;
mod viewport;
mod world;

struct TrainApp<'a> {
    last: Instant,
    world: World,
    animations: AnimationStates,
    input_state: InputState,
    rs: RenderState<'a>,
    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,
    should_exit: bool,
}

impl<'a> TrainApp<'a> {
    async fn new(window: &'a mut glfw::Window) -> Self {
        let mut rs = RenderState::new(window).await;
        let input_queue = new_message_queue();
        let thread_copy = input_queue.clone();

        let _input_thread = std::thread::spawn(|| {
            if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
                println!("Error: {:?}", error)
            }
        });

        rs.resources.load_font(&rs.renderer, "consolas");
        rs.resources.load_font(&rs.renderer, "cambria");
        rs.resources.load_font(&rs.renderer, "garamond");
        rs.resources.load_font(&rs.renderer, "arial");
        rs.resources.load_font(&rs.renderer, "calibri");
        rs.resources.load_font(&rs.renderer, "verdana");
        rs.resources.load_font(&rs.renderer, "impact");
        rs.resources.load_font(&rs.renderer, "courier_new");

        rs.window.set_framebuffer_size_polling(true);
        rs.window.set_key_polling(true);
        rs.window.set_mouse_button_polling(true);
        rs.window.set_pos_polling(true);

        Self {
            last: Instant::now(),
            rs,
            world: make_world(),
            animations: AnimationStates::new(),
            input_state: InputState::default(),
            _input_thread,
            input_queue,
            should_exit: false,
        }
    }
}

impl<'a> RendApp for TrainApp<'a> {
    fn update(&mut self) {
        self.input_state.on_frame_boundary();

        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();

        self.animations.update(dt);
        while let Some(event) = self.input_queue.pop() {
            self.input_state
                .process_rdev_event(&event, self.rs.window.is_focused());
        }

        let (width, height) = self.rs.window.get_size();
        let screen_width = DVec2::new(width as f64, height as f64);
        let mouse = DVec2::new(
            self.rs.window.get_cursor_pos().0,
            self.rs.window.get_cursor_pos().1,
        );

        let mouse = mouse.with_y(height as f64 - mouse.y);

        update_world(&mut self.world, dt as f64, mouse, screen_width);
        process_input(
            &mut self.world,
            &self.input_state,
            dt as f64,
            mouse,
            screen_width,
        );

        if self.input_state.is_key_pressed(rdev::Key::Escape) {
            self.should_exit = true;
        }

        self.last = now;
    }

    fn emit_render_commands(&mut self) -> RenderCommands {
        let font_info: BTreeMap<usize, FontInfo> = self
            .rs
            .resources
            .fonts
            .iter()
            .map(|(id, (font, _sprite))| (*id, font.clone()))
            .collect();
        let mut cmd = RenderCommands::new(font_info);
        cmd.current_font_id = self.world.current_font_id;
        let (width, height) = self.rs.window.get_size();
        let dims = DVec2::new(width as f64, height as f64);
        let mouse = DVec2::new(
            self.rs.window.get_cursor_pos().0,
            self.rs.window.get_cursor_pos().1,
        );

        let mouse = mouse.with_y(height as f64 - mouse.y);
        let mut event_bus = EventBus::new();

        draw_world(
            &mut cmd,
            &mut event_bus,
            &self.input_state,
            &self.world,
            dims,
            mouse,
            &self.animations,
        );

        if let Some(font_id) = event_bus.new_font_id() {
            self.world.current_font_id = font_id;
        }

        cmd
    }

    fn on_event(&mut self, _event: &glfw::WindowEvent) {}

    fn render(&mut self, commands: &RenderCommands) {
        match self.rs.render(&commands) {
            Ok(Some(drawable)) => {
                drawable.present();
            }
            Ok(None) => (),
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.rs.update_surface();
                self.rs.resize(self.rs.window.get_size());
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }

    fn should_close(&self) -> bool {
        // todo!()
        self.rs.window.should_close() || self.should_exit
    }
}

fn run(
    mut glfw: glfw::Glfw,
    events: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
    mut app: impl RendApp,
) {
    while !app.should_close() {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            app.on_event(&event);
        }

        app.update();

        let commands = app.emit_render_commands();
        app.render(&commands);
    }
}

async fn init() {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::NoApi));
    let (mut window, events) = glfw
        .create_window(1200, 950, "It's WGPU time.", glfw::WindowMode::Windowed)
        .unwrap();

    window.maximize();

    run(glfw, events, TrainApp::new(&mut window).await);
}

fn main() {
    pollster::block_on(init());
}
