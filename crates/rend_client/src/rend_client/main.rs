mod camera;
mod world;
use bary_ui::example_layout;
use glfw::{Action, ClientApiHint, Key, WindowHint, fail_on_errors};
use glm::*;
use rend::*;
use std::collections::{BTreeMap, HashSet};
mod rend_app;

use crate::camera::*;
use crate::rend_app::*;
use crate::world::*;

fn make_commands(
    commands: &mut RenderCommands,
    frames: u32,
    font_name: &str,
    font_size: f64,
    time: f64,
    width: f32,
    height: f32,
) {
    let layout = example_layout(width, height);

    for node in layout.iter() {
        let aabb = node.aabb();
        let pos = Vec2d::new(aabb.lower().x as f64, aabb.lower().y as f64);
        let dims = Vec2d::new(aabb.span.x as f64, aabb.span.y as f64);
        commands.rect(pos, dims, 0.0, Color::GRAY.alpha(0.2));
    }

    let (font_id, font) = commands
        .fonts
        .iter()
        .find_map(|(id, f)| {
            if f.name == font_name {
                Some((*id, f))
            } else {
                None
            }
        })
        .unwrap();

    let info = format!("({} frames) {} {:0.2} px", frames, font.name, font_size);

    let text = "Saturn is the sixth planet from the Sun and the \
        second largest in the Solar System, after Jupiter. It is a gas giant, \
        with an average radius of about 9 times that of Earth. It has an \
        eighth of the average density of Earth, but is over 95 times more \
        massive. Even though Saturn is almost as big as Jupiter, Saturn has \
        less than a third of its mass. Saturn orbits the Sun at a distance \
        of 9.59 AU (1,434 million km), with an orbital period of 29.45 years.\
        \n\n\
        Saturn's interior is thought to be composed of a rocky core, surrounded \
        by a deep layer of metallic hydrogen, an intermediate layer of liquid \
        hydrogen and liquid helium, and an outer layer of gas. Saturn has a \
        pale yellow hue, due to ammonia crystals in its upper atmosphere. An \
        electrical current in the metallic hydrogen layer is thought to give \
        rise to Saturn's planetary magnetic field, which is weaker than Earth's, \
        but has a magnetic moment 580 times that of Earth because of Saturn's \
        greater size. Saturn's magnetic field strength is about a twentieth \
        that of Jupiter.[27] The outer atmosphere is generally bland and \
        lacking in contrast, although long-lived features can appear. Wind \
        speeds on Saturn can reach 1,800 kilometres per hour (1,100 miles \
        per hour).\
        \n\n\
        The planet has a bright and extensive system of rings, composed mainly \
        of ice particles, with a smaller amount of rocky debris and dust. At \
        least 293 moons orbit the planet, of which 63 are officially named; \
        these do not include the hundreds of moonlets in the rings. Titan, \
        Saturn's largest moon and the second largest in the Solar System, is \
        larger (but less massive) than the planet Mercury and is the only moon \
        in the Solar System that has a substantial atmosphere.[28]";

    let layout_width = 1700.0;

    commands.paragraph(font_id, 40.0, 200.0, 100.0, &info, None);
    commands.paragraph(font_id, font_size, 200.0, 200.0, &text, Some(layout_width));

    // commands.rect(
    //     Vec2d::new(150.0, 100.0),
    //     Vec2d::new(7.0, 1000.0),
    //     0.0,
    //     Color::WHITE,
    // );
    // commands.rect(
    //     Vec2d::new(200.0, 180.0),
    //     Vec2d::new(layout_width, 7.0),
    //     0.0,
    //     Color::GRAY,
    // );
    // commands.rect(
    //     Vec2d::new(0.0, 0.0),
    //     Vec2d::new(layout_width + 500.0, 4000.0),
    //     0.0,
    //     Color::gray(0.0, 0.7),
    // );

    {
        commands
            .circle(700.0, 500.0)
            .diameter(320.0)
            .color(Color::BLACK);
        commands
            .line(Vec2d::new(700.0, 500.0), Vec2d::new(1200.0, 900.0))
            .color(Color::BLACK)
            .thickness(32.0);
        for i in 0..20 {
            let a = i as f64 / 4.0 + time;
            let r1 = 155.0;
            let r2 = 225.0 + 50.0 * a.sin();
            let start = Vec2d::new(700.0, 500.0) + Vec2d::new(a.cos(), a.sin()) * r1;
            let end = Vec2d::new(700.0, 500.0) + Vec2d::new(a.cos(), a.sin()) * r2;
            commands.line(start, end);
        }
    }

    commands
        .circle(700.0, 500.0)
        .diameter(300.0)
        .color(Color::RED);
    commands
        .circle(700.0, 500.0)
        .diameter(112.0)
        .color(Color::WHITE);
    commands
        .line(Vec2d::new(700.0, 500.0), Vec2d::new(1200.0, 900.0))
        .color(Color::WHITE)
        .thickness(18.0);
    commands
        .line(Vec2d::new(700.0, 500.0), Vec2d::new(1200.0, 900.0))
        .color(Color::GREEN)
        .thickness(12.0);

    // commands
    //     .circle(700.0, 500.0)
    //     .diameter(100.0)
    //     .color(Color::BLUE);
    // commands
    //     .circle(1800.0, 700.0)
    //     .radius(500.0)
    //     .color(Color::BROWN);
    // commands
    //     .circle(1800.0, 700.0)
    //     .radius(490.0)
    //     .color(Color::RED);
    // commands
    //     .circle(1800.0, 700.0)
    //     .radius(120.0)
    //     .color(Color::ORANGE);
    // commands
    //     .circle(1800.0, 700.0)
    //     .radius(60.0)
    //     .color(Color::WHITE);
}

fn make_world(rs: &mut RenderState) -> World {
    let quad_id = rs.resources.spawn_mesh(make_quad(&rs.renderer.device));

    let cube_id = rs.resources.spawn_mesh(make_cube(
        &rs.renderer.device,
        Vec4::new(1.0, 0.6, 0.6, 0.4),
    ));

    let tetra_id = rs
        .resources
        .spawn_mesh(make_tetrahedron(&rs.renderer.device));
    let nine_gon_id = rs.resources.spawn_mesh(make_n_gon(&rs.renderer.device, 9));

    rs.resources.load_font(&rs.renderer, "cambria");
    rs.resources.load_font(&rs.renderer, "consolas");
    rs.resources.load_font(&rs.renderer, "garamond");
    rs.resources.load_font(&rs.renderer, "arial");
    rs.resources.load_font(&rs.renderer, "calibri");

    let mut world = World::new();

    for x in [-100, 0, 100] {
        for z in [-100, 0, 100] {
            let id = rs
                .resources
                .spawn_ground_plane(&rs.renderer.device, x, z, 100);
            world.ground_plane(x, z, id);
        }
    }

    world.quads.push(MeshObject {
        position: Vec3::new(0.0, 6.0, -9.0),
        angle: 0.0,
        vel: 0.0,
        should_animate: false,
        mesh_id: nine_gon_id,
    });
    world.quads.push(MeshObject {
        position: Vec3::new(0.0, 4.0, -5.6),
        angle: 0.0,
        vel: 0.0,
        should_animate: false,
        mesh_id: nine_gon_id,
    });
    world.quads.push(MeshObject {
        position: Vec3::new(0.0, 5.0, 0.0),
        angle: 0.0,
        vel: 0.01,
        should_animate: false,
        mesh_id: tetra_id,
    });

    for i in 0..20 {
        let a = i as f32 / 5.0;
        let z = i as f32 * 1.0 - 10.0;
        world.quads.push(MeshObject {
            position: Vec3::new(4.5, 3.0, z),
            angle: a,
            vel: 0.0,
            should_animate: false,
            mesh_id: quad_id,
        });
    }

    for i in (0..200).step_by(14) {
        let a = i as f32 / 6.0;
        let r = 3.0 + i as f32 / 8.0;
        let x = a.cos() * r;
        let z = a.sin() * r;
        world.quads.push(MeshObject {
            position: Vec3::new(x as f32, 0.0, z as f32),
            angle: i as f32 * 0.4,
            vel: 1.0,
            should_animate: true,
            mesh_id: cube_id,
        });
    }

    world
}

fn make_camera_controls(keys: &HashSet<Key>) -> CameraControls {
    let mut ctrls = CameraControls::default();
    if keys.contains(&Key::Space) {
        ctrls.y_axis = CamDir::Positive
    }
    if keys.contains(&Key::LeftShift) {
        ctrls.y_axis = CamDir::Negative
    }
    if keys.contains(&Key::A) {
        ctrls.x_axis = CamDir::Negative;
    }
    if keys.contains(&Key::D) {
        ctrls.x_axis = CamDir::Positive;
    }
    if keys.contains(&Key::W) {
        ctrls.z_axis = CamDir::Positive;
    }
    if keys.contains(&Key::S) {
        ctrls.z_axis = CamDir::Negative;
    }
    ctrls
}

struct DemoApp<'a> {
    keys_pressed: HashSet<Key>,
    world: World,
    rs: RenderState<'a>,
}

impl<'a> DemoApp<'a> {
    async fn new(window: &'a mut glfw::Window) -> Self {
        let mut rs = RenderState::new(window).await;
        let world = make_world(&mut rs);

        rs.window.set_framebuffer_size_polling(true);
        rs.window.set_key_polling(true);
        rs.window.set_mouse_button_polling(true);
        rs.window.set_pos_polling(true);

        Self {
            keys_pressed: HashSet::new(),
            world,
            rs,
        }
    }
}

impl<'a> RendApp for DemoApp<'a> {
    fn update(&mut self) {
        let ctrls = make_camera_controls(&self.keys_pressed);
        self.world.update(16.67 / 1000.0, &ctrls);

        self.rs.update(
            self.world.camera.to_projection_matrix(&self.rs.window),
            self.world.time,
        );
    }

    fn emit_render_commands(&self) -> RenderCommands {
        let font_info: BTreeMap<usize, FontInfo> = self
            .rs
            .resources
            .fonts
            .iter()
            .map(|(id, (font, _sprite))| (*id, font.clone()))
            .collect();

        let mut commands = RenderCommands::new(font_info.clone());

        let (width, height) = self.rs.window.get_size();
        make_commands(
            &mut commands,
            0,
            "Consolas",
            48.0,
            self.world.time as f64,
            width as f32,
            height as f32,
        );

        commands
    }

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

    fn on_event(&mut self, event: &glfw::WindowEvent) {
        match event {
            glfw::WindowEvent::Key(key, _, Action::Press, _) => {
                self.keys_pressed.insert(*key);
            }
            glfw::WindowEvent::Key(key, _, Action::Release, _) => {
                self.keys_pressed.remove(key);
            }
            _ => (),
        }

        match event {
            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                self.rs.window.set_should_close(true)
            }

            //Window was moved
            glfw::WindowEvent::Pos(..) => {
                self.rs.update_surface();
                self.rs.resize(self.rs.window.get_size());
            }

            //Window was resized
            glfw::WindowEvent::FramebufferSize(width, height) => {
                self.rs.update_surface();
                self.rs.resize((*width, *height));
            }
            _ => {}
        }
    }

    fn should_close(&self) -> bool {
        self.rs.window.should_close()
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

    run(glfw, events, DemoApp::new(&mut window).await);
}

fn main() {
    pollster::block_on(init());
}
