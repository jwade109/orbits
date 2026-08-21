use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::{MessageQueue, new_message_queue};
use bary_sim::Camera;
use glfw::*;
use rend::*;
use std::{collections::BTreeMap, thread::JoinHandle, time::Instant};
mod rend_app;
use crate::rend_app::*;
use crate::tweens::{AnimationStates, Tween};
use crate::world::*;
mod tweens;
mod world;

fn to_glm(p: bary_core::prelude::Vec2) -> DVec2 {
    DVec2::new(p.x.into(), p.y.into())
}

fn draw_spider(cmd: &mut RenderCommands, spider: &Spider, cam: &Camera, screen_width: Vec2) {
    let s = to_glm(cam.world_to_screen(spider.pose.translation, screen_width));

    cmd.circle(s.x, s.y)
        .radius(cam.zoom as f64 * 3.0)
        .inner_radius(cam.zoom as f64 * 1.5)
        .color(Color::ORANGE.alpha(0.2));

    cmd.circle(s.x, s.y)
        .radius(cam.zoom as f64 * 2.7)
        .inner_radius(cam.zoom as f64 * 1.8)
        .color(Color::ORANGE.alpha(0.2));

    for leg in &spider.legs {
        if leg.state == LegState::Retracted {
            continue;
        }
        let e = to_glm(cam.world_to_screen(leg.foot_position, screen_width));
        cmd.line(s, e).thickness(0.5 * cam.zoom as f64);
    }

    let r = cmd
        .circle(s.x, s.y)
        .diameter(cam.zoom as f64)
        .color(Color::BLUE);
}

fn draw_world(
    cmd: &mut RenderCommands,
    input: &InputState,
    world: &World,
    screen_width: Vec2,
    mouse: DVec2,
    anim: &AnimationStates,
) {
    let cam = &world.camera;

    {
        let n_lines = 500;
        let spacing = 10;

        for x in -n_lines..=n_lines {
            let x = x * spacing;
            let s = Vec2::new(x as f32, -10000.0);
            let e = Vec2::new(x as f32, 10000.0);
            cmd.line(
                to_glm(cam.world_to_screen(s, screen_width)),
                to_glm(cam.world_to_screen(e, screen_width)),
            )
            .color(Color::GRAY)
            .thickness(3.0);
            let s = Vec2::new(-10000.0, x as f32);
            let e = Vec2::new(10000.0, x as f32);
            cmd.line(
                to_glm(cam.world_to_screen(s, screen_width)),
                to_glm(cam.world_to_screen(e, screen_width)),
            )
            .color(Color::GRAY)
            .thickness(3.0);
        }
    }

    {
        let pw = Vec2::ZERO;
        let ps = world.camera.world_to_screen(pw, screen_width);

        let mut lines = vec!["SPIDERBOI".to_string(), format!("{} ticks", world.ticks)];

        for (id, num, tween, state) in anim.animations() {
            lines.push(format!("{} {} {:?} {:0.2}", id, num, tween, state));
        }

        let text = lines.join("\n");
        let qs = cmd.text(to_glm(ps), &text, 2.0 * cam.zoom as f64);

        cmd.text(DVec2::new(20.0, 20.0), text, 32.0);

        cmd.frame(to_glm(ps), to_glm(qs.as_vec2()), 0.3 * cam.zoom as f64);
    }

    let clicked = input.is_key_pressed(rdev::Button::Left);

    let mouse_world = cam.screen_to_world(mouse.as_vec2(), screen_width);

    for (i, spider) in world.spiders.iter().enumerate() {
        draw_spider(cmd, spider, cam, screen_width);

        let s = spider.pose.translation;
        let mouseover = s.distance(mouse_world) < 3.0;
        let t1 = anim.anim(("spider_select", i), Tween::Exponential, 0.26, mouseover);
        let t2 = anim.anim(
            ("spider_click", i),
            Tween::Exponential,
            0.20,
            mouseover && clicked,
        );
        let t1 = 0.6 + t1 * 0.4;

        cmd.circle_new(cam.world_to_screen(s, screen_width).into())
            .inner_radius(100.0 * t1 + 50.0 - 23.0 * t2)
            .radius(180.0 * t1 + 23.0 * t2)
            .color(Color::ORANGE);
    }

    cmd.circle_new(mouse).diameter(20.0).color(Color::RED);
}

struct SpiderApp<'a> {
    last: Instant,
    world: World,
    animations: AnimationStates,
    input_state: InputState,
    rs: RenderState<'a>,
    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,
    should_exit: bool,
}

impl<'a> SpiderApp<'a> {
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

impl<'a> RendApp for SpiderApp<'a> {
    fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();

        self.animations.update(dt);
        while let Some(event) = self.input_queue.pop() {
            self.input_state
                .process_rdev_event(&event, self.rs.window.is_focused());
        }

        update_world(&mut self.world, dt as f32);
        process_input(&mut self.world, &self.input_state);

        if self.input_state.is_key_pressed(rdev::Key::Escape) {
            self.should_exit = true;
        }

        self.input_state.on_frame_boundary();

        self.last = now;
    }

    fn emit_render_commands(&self) -> RenderCommands {
        let font_info: BTreeMap<usize, FontInfo> = self
            .rs
            .resources
            .fonts
            .iter()
            .map(|(id, (font, _sprite))| (*id, font.clone()))
            .collect();
        let mut cmd = RenderCommands::new(font_info);
        let (width, height) = self.rs.window.get_size();
        let dims = Vec2::new(width as f32, height as f32);
        let mouse = DVec2::new(
            self.rs.window.get_cursor_pos().0,
            self.rs.window.get_cursor_pos().1,
        );
        draw_world(
            &mut cmd,
            &self.input_state,
            &self.world,
            dims,
            mouse,
            &self.animations,
        );
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

    run(glfw, events, SpiderApp::new(&mut window).await);
}

fn main() {
    pollster::block_on(init());
}
