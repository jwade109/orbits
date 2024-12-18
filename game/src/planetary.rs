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
        app.add_systems(Update, (draw_orbital_system, keyboard_input, scroll_events));
        app.add_systems(
            FixedUpdate,
            (propagate_system, update_sim_time, drop_expired_entities),
        );
        app.add_systems(Update, (log_system_info, update_camera, draw_event_markers));
    }
}

#[derive(Debug, Resource, Default)]
struct SimTime(Duration);

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
        if let Some(pv) = state.system.transform_from_id(event.relative_to) {
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
        .resolution(orb.semi_major_axis.clamp(3.0, 40.0) as u32);

    // let line_start = origin + orb.pos().normalize() * (orb.body.radius + 5.0);
    // gizmos.line_2d(line_start, origin + orb.pos(), color);

    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.periapsis()),
        2.0,
        Srgba { alpha, ..RED },
    );

    if orb.eccentricity < 1.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(origin + orb.apoapsis()),
            2.0,
            Srgba { alpha, ..WHITE },
        );
    }
}

#[derive(Resource)]
struct PlanetaryState {
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
}

impl Default for PlanetaryState {
    fn default() -> Self {
        PlanetaryState {
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
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(PlanetaryState::default());
    commands.insert_resource(SimTime::default());
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

fn draw_orbital_system(mut gizmos: Gizmos, state: Res<PlanetaryState>) {
    {
        let b = state.system.barycenter();
        gizmos.circle_2d(Isometry2d::from_translation(b), 6.0, PURPLE);
        draw_x(&mut gizmos, b, 8.0, PURPLE);
    }

    {
        let (pos, abridged) = get_future_positions(&state.system, state.focus_object, 2000);
        if abridged && !pos.is_empty() {
            draw_x(&mut gizmos, *pos.last().unwrap(), 16.0, ORANGE);
        }
        gizmos.linestrip_2d(pos, ORANGE);
    }

    if let Some(p) = state.system.transform_from_id(Some(state.focus_object)) {
        let s = 400.0;
        let color = Srgba {
            alpha: 0.02,
            ..ORANGE
        };
        let d1 = Vec2::new(s, s);
        let d2 = Vec2::new(-s, s);
        gizmos.line_2d(p.pos - d1, p.pos + d1, color);
        gizmos.line_2d(p.pos - d2, p.pos + d2, color);
    }

    for object in state.system.objects.iter() {
        if let Some(body) = object.body {
            let iso = Isometry2d::from_translation(object.prop.pos());
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
        match (&object.prop, state.system.global_transform(&object.prop)) {
            (Propagator::Fixed(_, _), Some(pv)) => {
                draw_x(&mut gizmos, pv.pos, 14.0, RED);
            }
            (Propagator::NBody(nb), Some(pv)) => {
                draw_square(&mut gizmos, nb.pos, 9.0, WHITE);
                if state.show_orbits || state.focus_object == object.id {
                    let parent_object = state.system.primary_body_at(pv.pos, Some(object.id));
                    let parent_pv: Option<PV> = parent_object
                        .clone()
                        .map(|o| state.system.global_transform(&o.prop))
                        .flatten();
                    let parent_body = parent_object.map(|o| o.body).flatten();

                    if let (Some(parent_pv), Some(parent_body)) = (parent_pv, parent_body) {
                        let rpos: Vec2 = pv.pos - parent_pv.pos;
                        let rvel = pv.vel - parent_pv.vel;
                        let orb: Orbit = Orbit::from_pv(rpos, rvel, parent_body);
                        draw_orbit(parent_pv.pos, orb, &mut gizmos, 0.2, GRAY);
                    }
                }
            }
            (Propagator::Kepler(k), Some(pv)) => {
                if let Some(parent) = state.system.lookup(k.primary) {
                    let color: Srgba = ORANGE;
                    draw_square(&mut gizmos, pv.pos, 9.0, color);
                    if state.show_orbits || state.focus_object == object.id {
                        if let Some(parent_pv) = state.system.global_transform(&parent.prop) {
                            draw_orbit(parent_pv.pos, k.orbit, &mut gizmos, 0.2, GRAY);
                        }
                    }
                }
            }
            (_, None) => (),
        }
    }

    let mut lattice = vec![]; // generate_square_lattice(Vec2::ZERO, 10000, 200);

    for obj in state.system.objects.iter() {
        if let Some(body) = obj.body {
            if let Some(center) = state.system.global_transform(&obj.prop) {
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
                prim.map(|o| state.system.global_transform(&o.prop))
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

fn update_sim_time(time: Res<Time>, mut simtime: ResMut<SimTime>, config: Res<PlanetaryState>) {
    if config.paused {
        return;
    }
    let SimTime(dur) = simtime.as_mut();
    *dur = *dur + Duration::from_nanos((time.delta().as_nanos() as f32 * config.sim_speed) as u64);
}

fn propagate_system(
    mut commands: Commands,
    time: Res<Time>,
    simtime: Res<SimTime>,
    mut state: ResMut<PlanetaryState>,
) {
    let SimTime(t) = *simtime;

    while state.system.epoch < t {
        let events = state.system.step();
        let expiry = time.elapsed() + Duration::from_secs(5);
        for evt in events.iter() {
            commands.spawn(make_event_marker(evt.1, expiry));
        }
    }
}

fn log_system_info(state: Res<PlanetaryState>, mut evt: EventWriter<DebugLog>) {
    send_log(
        &mut evt,
        &format!("Epoch: {:0.2}", state.system.epoch.as_secs_f32()),
    );
    send_log(&mut evt, &format!("Iteration: {}", state.system.iter));
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
    send_log(
        &mut evt,
        &format!("Tracked object: {:?}", state.focus_object),
    );
    send_log(
        &mut evt,
        &format!("Follow tracked: {:?}", state.follow_object),
    );

    if let Some(obj) = state.system.lookup_ref(state.focus_object) {
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
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<PlanetaryState>,
    mut simtime: ResMut<SimTime>,
    time: Res<Time>,
    mut exit: ResMut<Events<bevy::app::AppExit>>,
) {
    for key in keys.get_pressed() {
        match key {
            KeyCode::ArrowDown => {
                config.sim_speed = f32::clamp(config.sim_speed - 0.01, 0.0, 1200.0);
            }
            KeyCode::ArrowUp => {
                config.sim_speed = f32::clamp(config.sim_speed + 0.01, 0.0, 1200.0);
            }
            KeyCode::ArrowLeft => {
                config.sim_speed = f32::clamp(config.sim_speed - 1.0, 0.0, 1200.0);
            }
            KeyCode::ArrowRight => {
                config.sim_speed = f32::clamp(config.sim_speed + 1.0, 0.0, 1200.0);
            }
            KeyCode::Period => {
                config.sim_speed = 1.0;
            }
            _ => (),
        }
    }

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
    if keys.just_pressed(KeyCode::KeyO) {
        config.show_orbits = !config.show_orbits;
    }
    if keys.just_pressed(KeyCode::KeyG) {
        config.show_gravity_field = !config.show_gravity_field;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        config.show_potential_field = !config.show_potential_field;
    }
    if keys.just_pressed(KeyCode::KeyB) {
        config.show_primary_body = !config.show_primary_body;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        config.follow_object = !config.follow_object;
    }
    if keys.just_pressed(KeyCode::Space) {
        config.paused = !config.paused;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        config.paused = true;
        let e = config.system.epoch + Duration::from_millis(100);
        *simtime = SimTime(e);
        let events = config.system.step();
        let expiry = time.elapsed() + Duration::from_secs(5);
        for evt in events.iter() {
            commands.spawn(make_event_marker(evt.1, expiry));
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        config.backup = None;
        config.system = default_example();
        *simtime = SimTime(config.system.epoch);
    }
    if keys.just_pressed(KeyCode::KeyJ) {
        config.backup = None;
        config.system = sun_jupiter_lagrange();
        *simtime = SimTime(config.system.epoch);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        config.backup = None;
        config.system = earth_moon_example_one();
        *simtime = SimTime(config.system.epoch);
    }
    if keys.just_pressed(KeyCode::KeyU) {
        config.backup = None;
        config.system = n_body_stability();
        *simtime = SimTime(config.system.epoch);
    }
    if keys.just_pressed(KeyCode::KeyY) {
        config.backup = None;
        config.system = patched_conics_scenario();
        *simtime = SimTime(config.system.epoch);
    }
    if keys.just_pressed(KeyCode::KeyK) {
        config.backup = Some(config.system.clone());
    }
    if keys.just_pressed(KeyCode::KeyL) {
        if let Some(sys) = &config.backup {
            config.system = sys.clone();
            *simtime = SimTime(config.system.epoch);
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.send(bevy::app::AppExit::Success);
    }
}

fn scroll_events(
    mut evr_scroll: EventReader<MouseWheel>,
    mut transforms: Query<&mut Transform, With<Camera>>,
) {
    use bevy::input::mouse::MouseScrollUnit;

    let mut transform = transforms.single_mut();

    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                if ev.y > 0.0 {
                    transform.scale /= 1.1;
                } else {
                    transform.scale *= 1.1;
                }
            }
            _ => (),
        }
    }
}

fn update_camera(mut query: Query<&mut Transform, With<Camera>>, state: Res<PlanetaryState>) {
    let mut tf = query.single_mut();

    if !state.follow_object {
        *tf = tf.with_translation(Vec3::ZERO);
        return;
    }

    if let Some(p) = state
        .system
        .lookup(state.focus_object)
        .map(|o| state.system.global_transform(&o.prop))
        .flatten()
    {
        *tf = tf.with_translation(p.pos.extend(0.0));
    }
}
