use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::{MessageQueue, new_message_queue};
use bary_orbital::Asteroid;
use glfw::*;
use rend::*;
use std::{collections::BTreeMap, thread::JoinHandle, time::Instant};
mod rend_app;
use crate::rend_app::*;
use crate::tweens::{AnimationStates, Tween};
use crate::viewport::Viewport;
use crate::world::*;
mod tweens;
mod viewport;
mod world;

fn draw_spider(cmd: &mut RenderCommands, spider: &Spider, view: &Viewport) {
    let s = view.world_to_screen(spider.pose.translation.as_dvec2());

    cmd.circle(s)
        .radius(view.zoom() * 3.0)
        .inner_radius(view.zoom() * 1.5)
        .color(Color::ORANGE.alpha(0.2));

    cmd.circle(s)
        .radius(view.zoom() * 2.7)
        .inner_radius(view.zoom() * 1.8)
        .color(Color::ORANGE.alpha(0.2));

    for leg in &spider.legs {
        if leg.state == LegState::Retracted {
            continue;
        }
        let e = view.world_to_screen(leg.foot_position);
        cmd.line(s, e).thickness(0.5 * view.zoom());
    }

    cmd.circle(s)
        .diameter(view.zoom())
        .color(Color::BLUE.alpha(0.7));

    cmd.isometry(view.w2s_iso(spider.pose), view.meters(2.0));

    for x in linspace(10.0, 400.0, 4) {
        let p = Isometry2d::new(Vec2::splat(x), x / 300.0);
        cmd.rect(p)
            .dims(DVec2::new(50.0, 20.0))
            .color(Color::BLUE.alpha(0.5));
        cmd.isometry(p, 50.0);

        let w = view.w2s_iso(p);
        cmd.rect(w)
            .dims(DVec2::new(view.meters(5.0), view.meters(2.0)))
            .color(Color::BLUE.alpha(0.5));
        cmd.isometry(w, view.meters(5.0));
    }
}

fn draw_asteroid(cmd: &mut RenderCommands, ast: &Asteroid, view: &Viewport) {
    let mut draw_outline = |scale: f64, thickness: f64| {
        let points = linspace_f64(0.0, 2.0 * PI_64, 100)
            .iter()
            .map(|theta| {
                let r = ast.radius_at(*theta as f32) as f64;
                let x = r * theta.cos() * scale;
                let y = r * theta.sin() * scale;
                view.world_to_screen((x, y))
            })
            .collect();

        cmd.linestring(points).thickness(thickness);
    };

    draw_outline(1.0, 10.0);
    draw_outline(0.9, 9.0);
    draw_outline(0.8, 8.0);
    draw_outline(0.7, 7.0);
    draw_outline(0.6, 6.0);
}

fn draw_button(
    cmd: &mut RenderCommands,
    text: &str,
    p: DVec2,
    mouse: DVec2,
    input: &InputState,
) -> (DVec2, bool) {
    let padding = DVec2::splat(15.0);
    let (tcmd, extent) = cmd.text(p + padding, text, 32.0);
    let full_extent = extent + padding * 2.0;
    let aabb = AABB::from_arbitrary(p.as_vec2(), (p + full_extent).as_vec2());
    let contains = aabb.contains(mouse.as_vec2());
    let alpha = contains as u8 as f64 * 0.2 + 0.9;
    cmd.rect(p)
        .dims(full_extent)
        .color(Color::BLUE.alpha(alpha));
    cmd.apply(tcmd);
    (
        full_extent,
        input.just_pressed(rdev::Button::Left) && contains,
    )
}

fn draw_grid_lines(cmd: &mut RenderCommands, view: &Viewport) {
    let n_lines = 500;
    let spacing = 10;

    for x in -n_lines..=n_lines {
        let x = x * spacing;
        let s = DVec2::new(x as f64, -10000.0);
        let e = DVec2::new(x as f64, 10000.0);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY)
            .thickness(3.0);
        let s = DVec2::new(-10000.0, x as f64);
        let e = DVec2::new(10000.0, x as f64);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY)
            .thickness(3.0);
    }
}

fn draw_font_ui(
    cmd: &mut RenderCommands,
    mouse: DVec2,
    input: &InputState,
    new_font_id: &mut usize,
) {
    let fonts = cmd.fonts.clone();

    let mut p = DVec2::new(30.0, 300.0);
    for (font_id, font) in fonts {
        let text = format!("{} {}", font_id, font.name);
        let (e, clicked) = draw_button(cmd, &text, p, mouse, input);
        if clicked {
            *new_font_id = font_id;
        }
        p.y += e.y + 15.0;
    }
}

fn draw_world(
    cmd: &mut RenderCommands,
    input: &InputState,
    world: &mut World,
    screen_width: DVec2,
    mouse: DVec2,
    anim: &AnimationStates,
) {
    let cam = &world.camera;

    let view = Viewport::new(world.camera, screen_width);

    draw_grid_lines(cmd, &view);
    draw_asteroid(cmd, &world.asteroid, &view);

    draw_font_ui(cmd, mouse, input, &mut world.current_font_id);

    cmd.current_font_id = world.current_font_id;

    {
        let pw = DVec2::splat(0.0);

        let chars = "abcdefghijklmnopqrstuvwxyz";

        let n = (world.ticks / 100) as usize % chars.len();
        let c = chars.chars().nth(n).unwrap();

        let mut lines = vec![
            chars.to_uppercase().to_string(),
            chars.to_string(),
            format!("{} ticks", world.ticks),
            c.to_string(),
        ];

        for (id, num, tween, state) in anim.animations() {
            lines.push(format!("{} {} {:?} {:0.2}", id, num, tween, state));
        }

        let text = lines.join("\n");

        {
            let iso = Isometry2d::from_pos(pw.as_vec2());
            let iso = view.w2s_iso(iso);
            let (text_cmd, extent) = cmd.text(iso, &text, view.meters(2.0));
            cmd.rect(iso).dims(extent).color(Color::LIGHT_BLUE);
            cmd.apply(text_cmd);
        }

        {
            let p = DVec2::new(20.0, 20.0);
            let (text_cmd, extent) = cmd.text(p, &text, 32.0);
            cmd.rect(p).dims(extent).color(Color::GREEN.alpha(0.2));
            cmd.apply(text_cmd);
        }
    }

    let lclicked = input.is_key_pressed(rdev::Button::Left);
    let rclicked = input.is_key_pressed(rdev::Button::Right);

    let mouse_world = view.screen_to_world(mouse);

    for (i, spider) in world.spiders.iter().enumerate() {
        draw_spider(cmd, spider, &view);

        let s = spider.pose.translation.as_dvec2();
        let mouseover = s.distance(mouse_world) < 3.0;
        let t1 = anim.anim(("spider_select", i), Tween::Exponential, 0.04, mouseover);
        let t2 = anim.anim(
            ("spider_click", i),
            Tween::Exponential,
            0.04,
            mouseover && lclicked,
        );
        let t3 = anim.anim(
            ("spider_rclick", i),
            Tween::Exponential,
            0.04,
            mouseover && rclicked,
        );
        let t1 = 0.6 + t1 * 0.4;

        let color = Color::ORANGE.mix(Color::GREEN, t3);

        cmd.circle(cam.world_to_screen(s, screen_width))
            .inner_radius(100.0 * t1 + 50.0 - 23.0 * t2)
            .radius(180.0 * t1 + 23.0 * t2)
            .color(color);
    }

    cmd.isometry(view.w2s_iso(world.camera.isometry), 50.0);
    cmd.circle(mouse).diameter(11.0).color(Color::RED);
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
        self.input_state.on_frame_boundary();

        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();

        self.animations.update(dt);
        while let Some(event) = self.input_queue.pop() {
            self.input_state
                .process_rdev_event(&event, self.rs.window.is_focused());
        }

        update_world(&mut self.world, dt as f64);
        process_input(&mut self.world, &self.input_state, dt as f64);

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

        draw_world(
            &mut cmd,
            &self.input_state,
            &mut self.world,
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
