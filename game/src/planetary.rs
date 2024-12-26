use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::time::Duration;

use starling::core::*;
use starling::examples::*;
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
    for (event, mut tf) in query.iter_mut() {
        if let Some(pv) = state
            .system
            .transform_from_id(event.relative_to, state.system.epoch)
        {
            let p = pv.pos + event.pos;
            tf.translation = (p + event.offset).extend(0.0);
            draw_square(&mut gizmos, p, 20.0, WHITE);
            gizmos.line_2d(p, p + event.offset, WHITE);
        }
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

    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.periapsis()),
        4.0,
        Srgba { alpha, ..RED },
    );

    if orb.eccentricity < 1.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(origin + orb.apoapsis()),
            4.0,
            Srgba { alpha, ..WHITE },
        );
    }
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
    primary_object: ObjectId,
    secondary_object: ObjectId,
    follow_object: bool,
    target_scale: f32,
    camera_easing: Vec2,
    camera_switch: bool,
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
            primary_object: ObjectId(10),
            secondary_object: ObjectId(20),
            backup: None,
            follow_object: false,
            target_scale: 4.0,
            camera_easing: Vec2::ZERO,
            camera_switch: false,
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

fn draw_orbital_system(mut gizmos: Gizmos, state: Res<PlanetaryState>) {
    let stamp = state.system.epoch;

    // gizmos.grid_2d(
    //     Isometry2d::from_translation(Vec2::ZERO),
    //     (100, 100).into(),
    //     (500.0, 500.0).into(),
    //     Srgba {
    //         alpha: 0.003,
    //         ..GRAY
    //     },
    // );

    {
        let (b, _) = state.system.barycenter();
        gizmos.circle_2d(Isometry2d::from_translation(b), 6.0, PURPLE);
        draw_x(&mut gizmos, b, 8.0, PURPLE);
    }

    if let Some(p) = state
        .system
        .transform_from_id(Some(state.primary_object), state.system.epoch)
    {
        draw_square(
            &mut gizmos,
            p.pos,
            80.0,
            Srgba {
                alpha: 0.3,
                ..ORANGE
            },
        );
    }

    if let Some(p) = state
        .system
        .transform_from_id(Some(state.secondary_object), state.system.epoch)
    {
        draw_square(&mut gizmos, p.pos, 75.0, Srgba { alpha: 0.3, ..BLUE });
    }

    {
        let start = state.system.epoch;
        let end = start + Duration::from_secs(100);
        let pos: Vec<_> =
            get_future_positions(&state.system, state.primary_object, start, end, 500)
                .iter()
                .map(|pvs| pvs.pv.pos)
                .collect();
        gizmos.linestrip_2d(pos, ORANGE);
        let pos: Vec<_> =
            get_future_positions(&state.system, state.secondary_object, start, end, 500)
                .iter()
                .map(|pvs| pvs.pv.pos)
                .collect();
        gizmos.linestrip_2d(pos, BLUE);

        if let Some(dist) = get_approach_info(
            &state.system,
            state.primary_object,
            state.secondary_object,
            start,
            end,
            500.0,
        ) {
            for a in dist.iter() {
                draw_circle(&mut gizmos, a.0.pv.pos, 200.0, ORANGE);
                draw_circle(&mut gizmos, a.0.pv.pos, 30.0, ORANGE);
                draw_circle(&mut gizmos, a.1.pv.pos, 200.0, BLUE);
                draw_circle(&mut gizmos, a.1.pv.pos, 30.0, BLUE);
            }
        }
    }

    for object in state.system.objects.iter() {
        if let Some((body, pv)) = object.body.zip(object.prop.pv_at(stamp)) {
            let iso = Isometry2d::from_translation(pv.pos);
            gizmos.circle_2d(iso, body.radius, WHITE);
            gizmos.circle_2d(
                iso,
                body.soi,
                Srgba {
                    alpha: 0.3,
                    ..ORANGE
                },
            );
        }
    }

    for object in state.system.objects.iter() {
        let pv = match (
            &object.prop,
            state.system.global_transform(&object.prop, stamp),
        ) {
            (Propagator::Fixed(_, _), Some(pv)) => {
                draw_x(&mut gizmos, pv.pos, 14.0, RED);
                Some(pv)
            }
            (Propagator::Kepler(k), Some(pv)) => {
                if let Some(parent) = state.system.lookup(k.primary) {
                    let color: Srgba = ORANGE;
                    draw_square(&mut gizmos, pv.pos, 9.0, color);
                    if state.show_orbits || state.primary_object == object.id {
                        if let Some(parent_pv) = state.system.global_transform(&parent.prop, stamp)
                        {
                            draw_orbit(parent_pv.pos, k.orbit, &mut gizmos, 0.2, GRAY);
                        }
                    }
                }
                Some(pv)
            }
            (_, None) => None,
        };

        if let Some(p) = pv {
            let s = 250.0;
            let lower = (p.pos / s).floor() * s;
            let upper = lower + Vec2::new(s, s);
            let iso = Isometry2d::from_translation((upper + lower) / 2.0);
            gizmos.rect_2d(
                iso,
                (s, s).into(),
                Srgba {
                    alpha: 0.02,
                    ..GRAY
                },
            );
        }
    }

    let mut lattice = vec![]; // generate_square_lattice(Vec2::ZERO, 10000, 200);

    for obj in state.system.objects.iter() {
        if let Some(body) = obj.body {
            if let Some(center) = state.system.global_transform(&obj.prop, stamp) {
                let minilat =
                    generate_circular_log_lattice(center.pos, body.radius + 5.0, body.soi * 2.0);
                lattice.extend(minilat);
            }
        }
    }

    let gravity = lattice.iter().map(|p| state.system.gravity_at(*p));
    let potential = lattice.iter().map(|p| state.system.potential_at(*p));
    let primary = lattice
        .iter()
        .map(|p| state.system.primary_body_at(*p, None));
    let max_potential = state.system.potential_at((500.0, 500.0).into());

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
            if let (Some(pr), Some(d)) = (
                prim.clone(),
                prim.map(|o| state.system.global_transform(&o.prop, stamp))
                    .flatten(),
            ) {
                let r = 0.5 * (d.pos.x / 1000.0).cos() + 0.5;
                let g = 0.5 * (d.pos.y / 1000.0).sin() + 0.5;
                let ObjectId(id) = pr.id;
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
    state.system.epoch = state.sim_time;
    // while state.system.epoch < state.sim_time {
    //     let events = state.system.step();
    //     let expiry = time.elapsed() + Duration::from_secs(5);
    //     for evt in events.iter() {
    //         commands.spawn(make_event_marker(evt.1, expiry));
    //     }
    // }
}

fn log_system_info(state: Res<PlanetaryState>, mut evt: EventWriter<DebugLog>) {
    send_log(
        &mut evt,
        &format!("Epoch: {:0.2}", state.system.epoch.as_secs_f32()),
    );
    send_log(&mut evt, &format!("Scale: {:0.3}", state.target_scale));
    send_log(&mut evt, &format!("{} objects", state.system.objects.len()));
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
    send_log(&mut evt, &format!("Units: {:#?}", state.system.units));
    send_log(&mut evt, &format!("Primary: {:?}", state.primary_object));
    send_log(
        &mut evt,
        &format!("Secondary: {:?}", state.secondary_object),
    );
    send_log(
        &mut evt,
        &format!("Follow tracked: {:?}", state.follow_object),
    );

    if let Some(obj) = state.system.lookup_ref(state.primary_object) {
        send_log(&mut evt, &format!("{:#?}", obj));
    }

    if let Some((pri, sec)) = state
        .system
        .lookup_ref(state.primary_object)
        .zip(state.system.lookup_ref(state.secondary_object))
    {
        match (pri.prop, sec.prop) {
            (Propagator::Kepler(k1), Propagator::Kepler(k2)) => {
                let p1 = k1.orbit.period();
                let p2 = k2.orbit.period();
                let syn = synodic_period(p1, p2);
                send_log(&mut evt, &format!("Synodic period: {:?}", syn));
            }
            _ => (),
        }
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
                config.camera_switch = true;
            }
            KeyCode::Equal => {
                config.target_scale /= 1.5;
            }
            KeyCode::Minus => {
                config.target_scale *= 1.5;
            }
            KeyCode::KeyS => {
                config.paused = true;
                config.system.epoch += Duration::from_millis(10);
                config.sim_time = config.system.epoch;
            }
            _ => (),
        }
    }

    if keys.just_pressed(KeyCode::KeyM) || keys.all_pressed([KeyCode::KeyM, KeyCode::ShiftLeft]) {
        let ObjectId(max) = config.system.max_id().unwrap_or(ObjectId(0));
        let ObjectId(mut id) = config.primary_object;
        id += 1;
        while !config.system.has_object(ObjectId(id)) && id < max {
            id += 1
        }
        config.primary_object = ObjectId(id.min(max));
    }
    if keys.just_pressed(KeyCode::KeyN) || keys.all_pressed([KeyCode::KeyN, KeyCode::ShiftLeft]) {
        let ObjectId(min) = config.system.min_id().unwrap_or(ObjectId(0));
        let ObjectId(mut id) = config.primary_object;
        id -= 1;
        while !config.system.has_object(ObjectId(id)) && id > min {
            id -= 1;
        }
        config.primary_object = ObjectId(id.max(min));
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
    } else if starts_with("primary") {
        if let Some(n) = cmd.get(1).map(|s| s.parse::<i64>().ok()).flatten() {
            state.primary_object = ObjectId(n)
        }
    } else if starts_with("secondary") {
        if let Some(n) = cmd.get(1).map(|s| s.parse::<i64>().ok()).flatten() {
            state.secondary_object = ObjectId(n)
        }
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

fn update_camera(
    mut query: Query<&mut Transform, With<Camera>>,
    mut state: ResMut<PlanetaryState>,
) {
    let mut tf = query.single_mut();

    if state.camera_switch {
        state.follow_object = !state.follow_object;
    }

    let current_pos = tf.translation.xy();

    let target_pos = if state.follow_object {
        state
            .system
            .lookup(state.primary_object)
            .map(|o| state.system.global_transform(&o.prop, state.system.epoch))
            .flatten()
            .map(|p| p.pos)
            .unwrap_or(Vec2::ZERO)
    } else {
        Vec2::ZERO
    };

    if state.camera_switch {
        state.camera_easing = current_pos - target_pos;
    }

    state.camera_switch = false;

    *tf = tf.with_translation((target_pos + state.camera_easing).extend(0.0));
    state.camera_easing *= 0.85;
}
