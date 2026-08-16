mod camera;
mod game_objects;

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use glfw::{Action, ClientApiHint, Key, WindowHint, fail_on_errors};
use glm::*;
use rend::*;

use crate::camera::*;
use crate::game_objects::*;

fn make_commands(
    commands: &mut RenderCommands,
    frames: u32,
    font_index: i32,
    font_size: f64,
    time: f64,
) {
    let fonts = commands.fonts.keys().collect::<Vec<_>>();
    let font_id = *fonts[(font_index % fonts.len() as i32) as usize];

    let font = commands.fonts.get(&font_id).unwrap();

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

    commands.paragraph(*fonts[0], 40.0, 200.0, 100.0, &info, None);
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
    let quad_id = rs.spawn_mesh(make_quad(&rs.renderer.device));
    let cube_id = rs.spawn_mesh(make_cube(
        &rs.renderer.device,
        Vec4::new(1.0, 0.6, 0.6, 0.4),
    ));
    let tetra_id = rs.spawn_mesh(make_tetrahedron(&rs.renderer.device));
    let nine_gon_id = rs.spawn_mesh(make_n_gon(&rs.renderer.device, 9));

    rs.load_font("cambria");
    rs.load_font("consolas");
    rs.load_font("garamond");
    rs.load_font("arial");
    rs.load_font("calibri");

    let mut world = World::new();

    for x in [-100, 0, 100] {
        for z in [-100, 0, 100] {
            let id = rs.spawn_ground_plane(x, z, 100);
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

async fn run() {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::NoApi));
    let (mut window, events) = glfw
        .create_window(800, 600, "It's WGPU time.", glfw::WindowMode::Windowed)
        .unwrap();

    let mut rs = RenderState::new(&mut window).await;

    rs.window.set_framebuffer_size_polling(true);
    rs.window.set_key_polling(true);
    rs.window.set_mouse_button_polling(true);
    rs.window.set_pos_polling(true);

    let mut keys_pressed = HashSet::new();

    let mut world = make_world(&mut rs);

    let mut font_index = 0i32;
    let mut font_size = 48.0f64;
    let mut target_font_size = 48.0f64;

    let font_info: BTreeMap<usize, FontInfo> = rs
        .fonts
        .iter()
        .map(|(id, (font, _sprite))| (*id, font.clone()))
        .collect();

    let start = Instant::now();

    // let mut paused = false;
    let mut view_selector = ViewSelector::World3d;
    let mut draw_wireframes = false;

    let mut frames = 0;

    while !rs.window.should_close() {
        glfw.poll_events();

        rs.update(world.camera.to_projection_matrix(&rs.window), world.time);

        font_size += (target_font_size - font_size) * 0.03;

        let mut commands = RenderCommands::new(font_info.clone());

        let now = Instant::now();

        make_commands(
            &mut commands,
            frames,
            font_index,
            font_size,
            (now - start).as_secs_f64(),
        );

        let ctrls = make_camera_controls(&keys_pressed);

        world.update(16.67 / 1000.0, &ctrls);

        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(key, _, Action::Press, _) => {
                    keys_pressed.insert(key);
                }
                glfw::WindowEvent::Key(key, _, Action::Release, _) => {
                    keys_pressed.remove(&key);
                }
                _ => (),
            }

            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    rs.window.set_should_close(true)
                }
                // glfw::WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                //     paused ^= true;
                // }
                glfw::WindowEvent::Key(Key::Z, _, Action::Press, _) => {
                    draw_wireframes ^= true;
                }
                glfw::WindowEvent::Key(Key::Right, _, Action::Press, _) => {
                    view_selector = enum_iterator::next_cycle(&view_selector);
                }
                glfw::WindowEvent::Key(Key::Left, _, Action::Press, _) => {
                    view_selector = enum_iterator::previous_cycle(&view_selector);
                }

                glfw::WindowEvent::Key(Key::M, _, Action::Press, _) => {
                    font_index += 1;
                    font_index = font_index.clamp(0, rs.fonts.len() as i32 - 1);
                }
                glfw::WindowEvent::Key(Key::N, _, Action::Press, _) => {
                    font_index -= 1;
                    font_index = font_index.clamp(0, rs.fonts.len() as i32 - 1);
                }

                glfw::WindowEvent::Key(Key::L, _, Action::Press, _) => {
                    target_font_size *= 1.1;
                    target_font_size = target_font_size.clamp(10.0, 250.0);
                }
                glfw::WindowEvent::Key(Key::K, _, Action::Press, _) => {
                    target_font_size /= 1.1;
                    target_font_size = target_font_size.clamp(10.0, 250.0);
                }

                //Window was moved
                glfw::WindowEvent::Pos(..) => {
                    rs.update_surface();
                    rs.resize(rs.window.get_size());
                }

                //Window was resized
                glfw::WindowEvent::FramebufferSize(width, height) => {
                    rs.update_surface();
                    rs.resize((width, height));
                }
                _ => {}
            }
        }

        frames += 1;

        match rs.render(
            view_selector,
            draw_wireframes,
            &world.quads,
            &commands,
            world.time,
        ) {
            Ok(Some(drawable)) => {
                drawable.present();
            }
            Ok(None) => (),
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                rs.update_surface();
                rs.resize(rs.window.get_size());
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn main() {
    pollster::block_on(run());
}
