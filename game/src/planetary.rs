use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::time::Duration;

use starling::core::*;
use starling::examples::*;
use starling::orbit::*;
use starling::planning::*;
use starling::propagator::*;

use crate::debug::*;

pub struct PlanetaryPlugin;

impl Plugin for PlanetaryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(Update, (draw_orbital_system, keyboard_input, handle_zoom));
        app.add_systems(
            FixedUpdate,
            (propagate_system, update_sim_time, drop_expired_entities),
        );
        app.add_systems(
            Update,
            (
                log_system_info,
                update_camera,
                draw_event_markers,
                process_commands,
            ),
        );
    }
}

#[derive(Component, Clone, Copy)]
struct Expires {
    expiry: Duration,
}

#[derive(Component, Clone)]
struct EventMarker {
    pos: Vec2,
    offset: Vec2,
    relative_to: Option<ObjectId>,
}

fn draw_event_markers(
    mut gizmos: Gizmos,
    mut query: Query<(&EventMarker, &mut Transform)>,
    state: Res<PlanetaryState>,
) {
    let frame = state.system.current_frame();

    for (event, mut tf) in query.iter_mut() {
        let relpos = match event.relative_to.map(|id| frame.lookup(id)).flatten() {
            Some((_, pv, _)) => pv.pos,
            None => Vec2::ZERO,
        };

        let p = relpos + event.pos;
        tf.translation = (p + event.offset).extend(0.0);
        draw_square(&mut gizmos, p, 20.0, WHITE);
        gizmos.line_2d(p, p + event.offset, WHITE);
    }
}

fn drop_expired_entities(
    time: Res<Time>,
    mut commands: Commands,
    query: Query<(Entity, &Expires)>,
) {
    let t = time.elapsed();
    for (e, exp) in query.iter() {
        if exp.expiry < t {
            commands.entity(e).despawn();
        }
    }
}

fn draw_orbit(origin: Vec2, orb: Orbit, gizmos: &mut Gizmos, alpha: f32, base_color: Srgba) {
    if orb.eccentricity >= 1.0 {
        let n_points = 30;
        let theta_inf = f32::acos(-1.0 / orb.eccentricity);
        let points: Vec<_> = (-n_points..n_points)
            .map(|i| 0.98 * theta_inf * i as f32 / n_points as f32)
            .map(|t| origin + orb.position_at(t))
            .collect();
        gizmos.linestrip_2d(points, Srgba { alpha: 0.05, ..RED })
    }

    let color = Srgba {
        alpha,
        ..base_color
    };

    // {
    //     let root = orb.pos() + origin;
    //     let t1 = root + orb.normal() * 60.0;
    //     let t2 = root + orb.tangent() * 60.0;
    //     let t3 = root + orb.vel() * 3.0;
    //     gizmos.line_2d(root, t1, GREEN);
    //     gizmos.line_2d(root, t2, GREEN);
    //     gizmos.line_2d(root, t3, PURPLE);
    // }

    let b = orb.semi_major_axis * (1.0 - orb.eccentricity.powi(2)).sqrt();
    let center: Vec2 = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
    let iso = Isometry2d::new(center, orb.arg_periapsis.into());
    gizmos
        .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), color)
        .resolution(orb.semi_major_axis.clamp(3.0, 200.0) as u32);
}

#[derive(Resource)]
struct PlanetaryState {
    sim_time: Duration,
    sim_speed: f32,
    show_orbits: bool,
    show_potential_field: bool,
    show_gravity_field: bool,
    show_primary_body: bool,
    paused: bool,
    system: OrbitalSystem,
    backup: Option<OrbitalSystem>,
    focus_object: ObjectId,
    follow_object: bool,
    target_scale: f32,
}

impl Default for PlanetaryState {
    fn default() -> Self {
        PlanetaryState {
            sim_time: Duration::default(),
            sim_speed: 1.0,
            show_orbits: true,
            show_potential_field: false,
            show_gravity_field: false,
            show_primary_body: false,
            paused: false,
            system: default_example(),
            focus_object: ObjectId(0),
            backup: None,
            follow_object: false,
            target_scale: 4.0,
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(PlanetaryState::default());
}

fn draw_x(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let dx = Vec2::new(size, 0.0);
    let dy = Vec2::new(0.0, size);
    gizmos.line_2d(p - dx, p + dx, color);
    gizmos.line_2d(p - dy, p + dy, color);
}

fn draw_square(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos.rect_2d(
        Isometry2d::from_translation(p),
        Vec2::new(size, size),
        color,
    );
}

fn draw_circle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos.circle_2d(Isometry2d::from_translation(p), size, color);
}

fn draw_orbital_frame(mut gizmos: &mut Gizmos, frame: &OrbitalFrame) {
    for (_, pv, body) in frame.objects.iter() {
        if let Some(b) = body {
            draw_circle(&mut gizmos, pv.pos, b.radius, WHITE);
            draw_circle(&mut gizmos, pv.pos, b.radius * 0.95, WHITE);
            draw_circle(&mut gizmos, pv.pos, b.soi, ORANGE);
        } else {
            draw_square(&mut gizmos, pv.pos, 10.0, WHITE);
        }
    }
}

fn draw_orbital_system(mut gizmos: Gizmos, state: Res<PlanetaryState>) {
    gizmos.grid_2d(
        Isometry2d::default(),
        (100, 100).into(),
        (500.0, 500.0).into(),
        Srgba {
            alpha: 0.02,
            ..GRAY
        },
    );

    let frame = state.system.current_frame();

    {
        for obj in state.system.objects.iter() {
            let dy = 3.0;
            let y = (obj.id.0 - state.focus_object.0) as f32 * dy;
            for h in obj.history.0.iter() {
                let dt = h.epoch().as_secs_f32() - state.sim_time.as_secs_f32();
                let p = Vec2::new(dt * 100.0, y);
                let color = match obj.id == state.focus_object {
                    true => WHITE,
                    false => RED,
                };
                gizmos.line_2d(p, p + Vec2::Y * dy * 0.6, color);
            }
            if let Some(_) = frame.lookup(obj.id) {
                let dt = frame.epoch.as_secs_f32() - state.sim_time.as_secs_f32();
                let p = Vec2::new(dt * 100.0, y);
                gizmos.line_2d(p, p + Vec2::Y * dy * 0.7, ORANGE);
            }
        }
    }

    draw_orbital_frame(&mut gizmos, &frame);

    {
        let p = frame.barycenter();
        draw_circle(&mut gizmos, p, 6.0, PURPLE);
        draw_x(&mut gizmos, p, 8.0, PURPLE);
    }

    if let Some((_, pv, _)) = frame.lookup(state.focus_object) {
        draw_x(
            &mut gizmos,
            pv.pos,
            5000.0,
            Srgba {
                alpha: 0.05,
                ..GRAY
            },
        );

        // let (pos, hit) = get_future_pvs(&state.system, state.focus_object, 1000);
        // for (i, p) in pos.iter().enumerate() {
        //     if i + 1 == pos.len() && hit {
        //         draw_x(&mut gizmos, p.pos, 20.0, ORANGE);
        //     } else {
        //         draw_square(&mut gizmos, p.pos, 5.0, ORANGE);
        //     }
        // }
    }

    let mut lattice = vec![]; // generate_square_lattice(Vec2::ZERO, 10000, 200);

    for (_, pv, body) in frame.objects.iter() {
        if let Some(b) = body {
            let minilat = generate_circular_log_lattice(pv.pos, b.radius + 5.0, b.soi * 2.0);
            lattice.extend(minilat);
        }
    }

    let gravity = lattice.iter().map(|p| frame.gravity_at(*p));
    let potential = lattice.iter().map(|p| frame.potential_at(*p));
    let primary = lattice.iter().map(|p| frame.primary_body_at(*p, None));
    let max_potential = frame.potential_at((500.0, 500.0).into());

    if state.show_gravity_field {
        for (grav, p) in gravity.zip(&lattice) {
            let a = grav.angle_to(Vec2::X);
            let r = 0.5 * a.cos() + 0.5;
            let g = 0.5 * a.sin() + 0.5;
            let color = Srgba {
                red: r,
                green: g,
                blue: 1.0,
                alpha: 0.8,
            };
            draw_square(&mut gizmos, *p, 10.0, color);
            gizmos.line_2d(*p, p + grav.normalize() * 15.0, color);
        }
    }
    if state.show_potential_field {
        for (pot, p) in potential.zip(&lattice) {
            let r = (((pot / max_potential * 5.0) as u32) as f32 / 5.0).sqrt();
            let color = Srgba {
                red: r,
                green: 0.0,
                blue: 1.0 - r,
                alpha: 0.7,
            };
            draw_square(&mut gizmos, *p, 20.0, color);
        }
    }
    if state.show_primary_body {
        for (prim, p) in primary.zip(&lattice) {
            if let (Some(pr), Some((_, pv, _))) =
                (prim.clone(), prim.map(|o| frame.lookup(o)).flatten())
            {
                let r = 0.5 * (pv.pos.x / 1000.0).cos() + 0.5;
                let g = 0.5 * (pv.pos.y / 1000.0).sin() + 0.5;
                let ObjectId(id) = pr;
                let b = 0.5 * (id as f32).cos() + 0.5;
                let color = Srgba {
                    red: r,
                    green: g,
                    blue: b,
                    alpha: 1.0,
                };
                draw_square(&mut gizmos, *p, 10.0, color);
            }
        }
    }
}

fn update_sim_time(time: Res<Time>, mut config: ResMut<PlanetaryState>) {
    if config.paused {
        return;
    }
    let sp = config.sim_speed;
    config.sim_time += Duration::from_nanos((time.delta().as_nanos() as f32 * sp) as u64);
}

fn propagate_system(mut commands: Commands, time: Res<Time>, mut state: ResMut<PlanetaryState>) {
    let e = state.sim_time;
    let events = state.system.propagate_to(e);
    let expiry = time.elapsed() + Duration::from_secs(5);
    for evt in events.iter() {
        commands.spawn(make_event_marker(evt.1, expiry));
    }
}

fn log_system_info(state: Res<PlanetaryState>, mut evt: EventWriter<DebugLog>) {
    let frame = state.system.current_frame();

    send_log(
        &mut evt,
        &format!("Epoch: {:0.2}", frame.epoch.as_secs_f32()),
    );
    send_log(&mut evt, &format!("{} objects", frame.objects.len()));
    if state.paused {
        send_log(&mut evt, "Paused");
    }
    send_log(&mut evt, &format!("Sim speed: {:0.2}", state.sim_speed));
    send_log(&mut evt, &format!("Show (o)orbits: {}", state.show_orbits));
    send_log(
        &mut evt,
        &format!("Show (p)otential: {}", state.show_potential_field),
    );
    send_log(
        &mut evt,
        &format!("Show (g)ravity: {}", state.show_gravity_field),
    );
    send_log(
        &mut evt,
        &format!("Show primary (b)ody: {}", state.show_primary_body),
    );
    send_log(
        &mut evt,
        &format!("Tracked object: {:?}", state.focus_object),
    );
    send_log(
        &mut evt,
        &format!("Follow tracked: {:?}", state.follow_object),
    );

    if let Some(obj) = frame.lookup(state.focus_object) {
        send_log(&mut evt, &format!("{:#?}", obj));
    }
}

fn make_event_marker(
    event: OrbitalEvent,
    expiry: Duration,
) -> (EventMarker, Expires, Text2d, Transform, TextFont, Anchor) {
    let (text, pos, rel) = match event {
        OrbitalEvent::NumericalError(ego) => {
            let ObjectId(a) = ego;
            (format!("NUMERROR({})", a), Vec2::ZERO, None)
        }
        OrbitalEvent::Collision(pos, ego, obj) => {
            let ObjectId(a) = ego;
            let ObjectId(b) = obj.unwrap_or(ObjectId(-1));
            (format!("COLLISION({}, {})", a, b), pos, obj)
        }
        OrbitalEvent::Escaped(pos, ego) => {
            let ObjectId(a) = ego;
            (format!("ESCAPED({})", a), pos, None)
        }
        OrbitalEvent::LookupFailure(ego) => {
            let ObjectId(a) = ego;
            (format!("LOOKUPFAILURE({})", a), Vec2::ZERO, None)
        }
    };
    let offset = randvec(300.0, 500.0);
    let font = TextFont {
        font_size: 25.0,
        ..default()
    };

    let anchor = if offset.y > 0.0 {
        Anchor::BottomCenter
    } else {
        Anchor::TopCenter
    };

    dbg!(event);

    (
        EventMarker {
            pos,
            offset,
            relative_to: rel,
        },
        Expires { expiry },
        Text2d(text.to_string()),
        Transform::from_translation((pos + offset).extend(0.0)),
        font,
        anchor,
    )
}

fn keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<PlanetaryState>,
    mut exit: ResMut<Events<bevy::app::AppExit>>,
    cstate: Res<CommandsState>,
) {
    if cstate.active {
        return;
    }

    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Period => {
                config.sim_speed = f32::clamp(config.sim_speed * 10.0, 0.0, 1000.0);
            }
            KeyCode::Comma => {
                config.sim_speed = f32::clamp(config.sim_speed / 10.0, 0.0, 2000.0);
            }
            KeyCode::KeyF => {
                config.follow_object = !config.follow_object;
            }
            KeyCode::Equal => {
                config.target_scale /= 1.5;
            }
            KeyCode::Minus => {
                config.target_scale *= 1.5;
            }
            _ => (),
        }
    }

    // let mut process_arrow_key = |key: KeyCode| {
    //     let dv = 0.1
    //         * match key {
    //             KeyCode::ArrowLeft => -Vec2::X,
    //             KeyCode::ArrowRight => Vec2::X,
    //             KeyCode::ArrowUp => Vec2::Y,
    //             KeyCode::ArrowDown => -Vec2::Y,
    //             _ => return,
    //         };

    //     let id = config.focus_object;
    //     let frame = config.system.frame(config.system.epoch);
    //     let obj = config.system.lookup_mut(id);
    //     let pvo = frame.lookup(id);
    //     if let Some((obj, (_, mut pv, _))) = obj.zip(pvo) {
    //         if let Propagator::Fixed(_, _) = obj.prop {
    //             return;
    //         }
    //         pv.vel += dv;
    //         obj.prop = NBodyPropagator::new(obj.prop.epoch(), pv.pos, pv.vel).into();
    //     }
    // };

    // if keys.pressed(KeyCode::ArrowLeft) {
    //     process_arrow_key(KeyCode::ArrowLeft);
    // }

    // if keys.pressed(KeyCode::ArrowRight) {
    //     process_arrow_key(KeyCode::ArrowRight);
    // }

    // if keys.pressed(KeyCode::ArrowUp) {
    //     process_arrow_key(KeyCode::ArrowUp);
    // }

    // if keys.pressed(KeyCode::ArrowDown) {
    //     process_arrow_key(KeyCode::ArrowDown);
    // }

    if keys.just_pressed(KeyCode::KeyM) || keys.all_pressed([KeyCode::KeyM, KeyCode::ShiftLeft]) {
        let ObjectId(max) = config.system.max_id().unwrap_or(ObjectId(0));
        let ObjectId(mut id) = config.focus_object;
        id += 1;
        while !config.system.has_object(ObjectId(id)) && id < max {
            id += 1
        }
        config.focus_object = ObjectId(id.min(max));
    }
    if keys.just_pressed(KeyCode::KeyN) || keys.all_pressed([KeyCode::KeyN, KeyCode::ShiftLeft]) {
        let ObjectId(min) = config.system.min_id().unwrap_or(ObjectId(0));
        let ObjectId(mut id) = config.focus_object;
        id -= 1;
        while !config.system.has_object(ObjectId(id)) && id > min {
            id -= 1;
        }
        config.focus_object = ObjectId(id.max(min));
    }
    if keys.just_pressed(KeyCode::Space) {
        config.paused = !config.paused;
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(bevy::app::AppExit::Success);
    }
}

fn load_new_scenario(state: &mut PlanetaryState, new_system: OrbitalSystem) {
    state.backup = Some(new_system.clone());
    state.system = new_system;
    state.sim_time = Duration::default();
}

fn on_command(
    commands: &mut Commands,
    time: &Res<Time>,
    state: &mut PlanetaryState,
    cmd: &Vec<String>,
) {
    let starts_with = |s: &'static str| -> bool { cmd.first() == Some(&s.to_string()) };

    if starts_with("load") {
        let system = match cmd.get(1).map(|s| s.as_str()) {
            Some("earth") => earth_moon_example_one(),
            Some("moon") => patched_conics_scenario(),
            Some("jupiter") => sun_jupiter_lagrange(),
            Some("stability") => n_body_stability(),
            Some("playground") => playground(),
            _ => {
                return;
            }
        };
        load_new_scenario(state, system);
    } else if starts_with("toggle") {
        match cmd.get(1).map(|s| s.as_str()) {
            Some("gravity") => {
                state.show_gravity_field = !state.show_gravity_field;
            }
            Some("potential") => {
                state.show_potential_field = !state.show_potential_field;
            }
            Some("primary") => {
                state.show_primary_body = !state.show_primary_body;
            }
            Some("orbit") => {
                state.show_orbits = !state.show_orbits;
            }
            _ => {
                return;
            }
        }
    } else if starts_with("restore") {
        if let Some(sys) = &state.backup {
            state.system = sys.clone();
            state.sim_time = state.system.epoch;
        }
    } else if starts_with("save") {
        state.backup = Some(state.system.clone());
    }
}

fn process_commands(
    mut commands: Commands,
    time: Res<Time>,
    mut evts: EventReader<DebugCommand>,
    mut state: ResMut<PlanetaryState>,
) {
    for DebugCommand(cmd) in evts.read() {
        on_command(&mut commands, &time, &mut state, cmd);
    }
}

fn handle_zoom(state: Res<PlanetaryState>, mut tf: Query<&mut Transform, With<Camera>>) {
    let mut transform = tf.single_mut();
    let ds = (state.target_scale - transform.scale) * 0.2;
    transform.scale += ds;
}

fn update_camera(mut query: Query<&mut Transform, With<Camera>>, state: Res<PlanetaryState>) {
    let mut tf = query.single_mut();

    if !state.follow_object {
        *tf = tf.with_translation(Vec3::ZERO);
        return;
    }

    let frame = state.system.current_frame();

    if let Some((_, pv, _)) = frame.lookup(state.focus_object) {
        *tf = tf.with_translation(pv.pos.extend(0.0));
    }
}
