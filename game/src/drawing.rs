#![allow(dead_code)]

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;

use starling::prelude::*;

use crate::camera_controls::CameraState;
use crate::mouse::MouseState;
use crate::planetary::{GameState, ShowOrbitsState};

pub struct GizmosPlugin;

impl Plugin for GizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_mouse_state, draw_game_state));
    }
}

fn alpha(color: Srgba, a: f32) -> Srgba {
    Srgba { alpha: a, ..color }
}

fn draw_cross(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let dx = Vec2::new(size, 0.0);
    let dy = Vec2::new(0.0, size);
    gizmos.line_2d(p - dx, p + dx, color);
    gizmos.line_2d(p - dy, p + dy, color);
}

fn draw_x(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size / 2.0;
    gizmos.line_2d(p + Vec2::new(-s, -s), p + Vec2::new(s, s), color);
    gizmos.line_2d(p + Vec2::new(s, -s), p + Vec2::new(-s, s), color);
}

fn draw_square(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos.rect_2d(
        Isometry2d::from_translation(p),
        Vec2::new(size, size),
        color,
    );
}

fn draw_diamond(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size / 2.0;
    let pts = [0.0, PI / 2.0, PI, -PI / 2.0, 0.0].map(|a| p + rotate(Vec2::X * s, a));
    gizmos.linestrip_2d(pts, color);
}

fn draw_circle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos
        .circle_2d(Isometry2d::from_translation(p), size, color)
        .resolution(200);
}

fn draw_velocity_vec(gizmos: &mut Gizmos, pv: PV, length: f32, color: Srgba) {
    let p1 = pv.pos;
    let p2 = pv.pos + pv.vel.normalize_or_zero() * length;
    gizmos.line_2d(p1, p2, color);
}

fn draw_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    gizmos.rect_2d(Isometry2d::from_translation(aabb.center), aabb.span, color);
}

fn draw_region(gizmos: &mut Gizmos, region: Region, color: Srgba) {
    match region {
        Region::AABB(aabb) => draw_aabb(gizmos, aabb, color),
        Region::OrbitRange(a, b) => {
            draw_circle(gizmos, Vec2::ZERO, a, color);
            draw_circle(gizmos, Vec2::ZERO, b, color);
        }
    }
}

fn draw_obb(gizmos: &mut Gizmos, obb: &OBB, color: Srgba) {
    draw_cross(gizmos, obb.0.center, 30.0, color);
    let mut corners = obb.corners().to_vec();
    corners.push(*corners.get(0).unwrap());
    gizmos.linestrip_2d(corners, color);
}

fn draw_orbit(gizmos: &mut Gizmos, orb: &SparseOrbit, origin: Vec2, color: Srgba) {
    if orb.will_escape() {
        let ta = if orb.is_hyperbolic() {
            let hrta = hyperbolic_range_ta(orb.ecc());
            linspace(-0.999 * hrta, 0.999 * hrta, 1000)
        } else {
            linspace(-PI, PI, 1000)
        };

        let points: Vec<_> = ta
            .iter()
            .filter_map(|t| {
                let p = orb.position_at(*t);
                if p.length() > orb.body.soi {
                    return None;
                }
                Some(origin + p)
            })
            .collect();
        gizmos.linestrip_2d(points, color);
    } else {
        let b = orb.semi_minor_axis();
        let center: Vec2 = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
        let iso = Isometry2d::new(center, orb.arg_periapsis.into());

        let res = orb.semi_major_axis.clamp(40.0, 300.0) as u32;

        gizmos
            .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), color)
            .resolution(res);
    }
}

fn draw_function(
    gizmos: &mut Gizmos,
    func: impl Fn(f32) -> f32,
    xmin: f32,
    xmax: f32,
    camera: &CameraState,
    color: Srgba,
) {
    let bounds = camera.game_bounds();
    let scale = camera.actual_scale;
    let xeval = linspace(xmin, xmax, 500);
    let yeval = apply(&xeval, func);
    let ymin = yeval.iter().min_by(|x, y| x.total_cmp(y)).unwrap();
    let ymax = yeval.iter().max_by(|x, y| x.total_cmp(y)).unwrap();
    let points = xeval
        .iter()
        .zip(&yeval)
        .map(|(x, y)| {
            let yscale = 2.0 * ((y - ymin) / (ymax - ymin)) - 1.0;
            bounds.center + Vec2::new(*x, yscale * 500.0) * scale
        })
        .collect::<Vec<_>>();
    let zero = xeval
        .iter()
        .map(|x| {
            let yscale = 2.0 * ((0.0 - ymin) / (ymax - ymin)) - 1.0;
            bounds.center + Vec2::new(*x, yscale * 500.0) * scale
        })
        .collect::<Vec<_>>();
    gizmos.linestrip_2d(zero, GRAY);
    gizmos.linestrip_2d(points, color);
}

fn draw_planets(gizmos: &mut Gizmos, planet: &PlanetarySystem, stamp: Nanotime, origin: Vec2) {
    draw_circle(gizmos, origin, planet.body.radius, alpha(GRAY, 0.1));
    for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
        draw_circle(gizmos, origin, planet.body.soi * ds, alpha(ORANGE, a));
    }

    for (orbit, pl) in &planet.subsystems {
        if let Some(pv) = orbit.pv(stamp).ok() {
            draw_orbit(gizmos, orbit, origin, alpha(GRAY, 0.4));
            draw_planets(gizmos, pl, stamp, origin + pv.pos)
        }
    }
}

fn draw_propagator(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    prop: &Propagator,
    stamp: Nanotime,
    scale: f32,
    with_event: bool,
    color: Srgba,
    duty_cycle: bool,
) -> Option<()> {
    let (_, parent_pv, _, _) = planets.lookup(prop.parent, stamp)?;

    draw_orbit(gizmos, &prop.orbit, parent_pv.pos, color);
    if with_event {
        if let Some((t, e)) = prop.stamped_event() {
            let pv_end = parent_pv + prop.pv(t)?;
            draw_event(gizmos, planets, &e, t, pv_end.pos, scale, duty_cycle);
        }
    }
    Some(())
}

fn draw_object(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    obj: &Orbiter,
    stamp: Nanotime,
    scale: f32,
    show_orbits: ShowOrbitsState,
    tracked: bool,
    duty_cycle: bool,
) -> Option<()> {
    let pv = obj.pv(stamp, planets)?;

    let size = (4.0 * scale).min(10.0);
    if duty_cycle && obj.will_collide() {
        draw_circle(gizmos, pv.pos, size + 10.0 * scale, RED);
        draw_circle(gizmos, pv.pos, size + 16.0 * scale, RED);
    } else if duty_cycle && obj.has_error() {
        draw_circle(gizmos, pv.pos, size + 10.0 * scale, YELLOW);
        draw_circle(gizmos, pv.pos, size + 16.0 * scale, YELLOW);
    } else if duty_cycle && obj.will_change() {
        draw_circle(gizmos, pv.pos, size + 7.0 * scale, TEAL);
    }

    let show_orbits = match show_orbits {
        ShowOrbitsState::All => true,
        ShowOrbitsState::Focus => tracked,
        ShowOrbitsState::None => false,
    };

    if tracked {
        for (i, prop) in obj.props().iter().enumerate() {
            let color = if i == 0 {
                alpha(WHITE, 0.02)
            } else {
                alpha(TEAL, (1.0 - i as f32 * 0.3).max(0.0))
            };
            if show_orbits {
                draw_propagator(
                    gizmos, planets, &prop, stamp, scale, true, color, duty_cycle,
                );
            }
        }
    } else {
        if show_orbits {
            let prop = obj.propagator_at(stamp)?;
            draw_propagator(
                gizmos,
                planets,
                prop,
                stamp,
                scale,
                false,
                alpha(GRAY, 0.02),
                duty_cycle,
            );
        }
    }
    Some(())
}

fn draw_scenario(
    gizmos: &mut Gizmos,
    scenario: &Scenario,
    stamp: Nanotime,
    scale: f32,
    show_orbits: ShowOrbitsState,
    track_list: &Vec<ObjectId>,
    duty_cycle: bool,
) {
    draw_planets(gizmos, &scenario.system, stamp, Vec2::ZERO);

    _ = scenario
        .ids()
        .into_iter()
        .filter_map(|id| {
            let obj = scenario.lup(id, stamp)?.orbiter()?;
            let is_tracked = track_list.contains(&obj.id());
            draw_object(
                gizmos,
                &scenario.system,
                obj,
                stamp,
                scale,
                show_orbits,
                is_tracked,
                duty_cycle,
            )
        })
        .collect::<Vec<_>>();
}

fn draw_scalar_field_cell(
    gizmos: &mut Gizmos,
    scalar_field: &impl Fn(Vec2) -> f32,
    center: Vec2,
    step: f32,
    levels: &[i32],
) {
    // draw_square(gizmos, center, step as f32, alpha(WHITE, 0.001));

    let bl = center + Vec2::new(-step / 2.0, -step / 2.0);
    let br = center + Vec2::new(step / 2.0, -step / 2.0);
    let tl = center + Vec2::new(-step / 2.0, step / 2.0);
    let tr = center + Vec2::new(step / 2.0, step / 2.0);

    let pot: Vec<(Vec2, f32)> = [bl, br, tr, tl]
        .iter()
        .map(|p| (*p, scalar_field(*p)))
        .collect();

    for level in levels {
        let mut pts = vec![];

        for i in 0..4 {
            let p1 = pot[i].0;
            let z1 = pot[i].1;
            let p2 = pot[(i + 1) % 4].0;
            let z2 = pot[(i + 1) % 4].1;

            let l = *level as f32;

            if z1 > l && z2 < l || z1 < l && z2 > l {
                let t = (l - z1) / (z2 - z1);
                let d = p1.lerp(p2, t);
                pts.push(d);
            }
        }

        gizmos.linestrip_2d(pts, alpha(RED, 0.03));
    }
}

fn draw_scalar_field(gizmos: &mut Gizmos, scalar_field: &impl Fn(Vec2) -> f32, levels: &[i32]) {
    let step = 250;
    for y in (-4000..=4000).step_by(step) {
        for x in (-4000..=4000).step_by(step) {
            let p = Vec2::new(x as f32, y as f32);
            draw_scalar_field_cell(gizmos, scalar_field, p, step as f32, levels);
        }
    }
}

fn draw_event_marker_at(
    gizmos: &mut Gizmos,
    event: &EventType,
    p: Vec2,
    scale: f32,
    duty_cycle: bool,
) {
    if !duty_cycle {
        match event {
            EventType::NumericalError => return,
            EventType::Collide(_) => return,
            _ => (),
        }
    }

    let color = match event {
        EventType::Collide(_) => {
            draw_x(gizmos, p, 40.0 * scale, RED);
            return;
        }
        EventType::NumericalError => YELLOW,
        EventType::Encounter(_) => GREEN,
        EventType::Escape(_) => TEAL,
        EventType::Impulse(_) => PURPLE,
    };

    draw_circle(gizmos, p, 15.0 * scale, alpha(color, 0.8));
    draw_circle(gizmos, p, 6.0 * scale, alpha(color, 0.8));
}

fn draw_event(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    event: &EventType,
    stamp: Nanotime,
    p: Vec2,
    scale: f32,
    duty_cycle: bool,
) -> Option<()> {
    if let EventType::Encounter(id) = event {
        let (body, pv, _, _) = planets.lookup(*id, stamp)?;
        draw_circle(gizmos, pv.pos, body.soi, alpha(ORANGE, 0.2));
    }
    draw_event_marker_at(gizmos, event, p, scale, duty_cycle);
    Some(())
}

fn draw_highlighted_objects(gizmos: &mut Gizmos, state: &GameState) {
    _ = state
        .highlighted()
        .into_iter()
        .filter_map(|id| {
            let pv = state.scenario.lup(id, state.sim_time)?.pv();
            draw_circle(gizmos, pv.pos, 20.0 * state.camera.actual_scale, GRAY);
            Some(())
        })
        .collect::<Vec<_>>();
}

fn draw_camera_controls(gizmos: &mut Gizmos, state: &GameState) {
    if let Some(p) = state.camera.mouse_pos() {
        draw_circle(gizmos, p, 3.0 * state.camera.actual_scale, RED);
    }

    if let Some(p) = state.camera.mouse_down_pos() {
        draw_circle(gizmos, p, 2.0 * state.camera.actual_scale, RED);
    }

    if let Some(a) = state.selection_region {
        draw_region(gizmos, a, RED);
    }
}

fn draw_controller(
    gizmos: &mut Gizmos,
    stamp: Nanotime,
    ctrl: &Controller,
    scale: f32,
) -> Option<()> {
    let plan = ctrl.plan()?;
    draw_maneuver_plan(gizmos, stamp, plan, scale);
    Some(())
}

fn draw_event_animation(
    gizmos: &mut Gizmos,
    scenario: &Scenario,
    id: ObjectId,
    stamp: Nanotime,
    scale: f32,
    duty_cycle: bool,
) -> Option<()> {
    let obj = scenario.lup(id, stamp)?.orbiter()?;
    let p = obj.props().last()?;
    let dt = Nanotime::secs(1);
    let mut t = stamp + dt;
    while t < p.end().unwrap_or(stamp + Nanotime::secs(30)) {
        let pv = obj.pv(t, &scenario.system)?;
        draw_diamond(gizmos, pv.pos, 11.0 * scale.min(1.0), alpha(WHITE, 0.6));
        t += dt;
    }
    for prop in obj.props() {
        if let Some((t, e)) = prop.stamped_event() {
            let pv = obj.pv(t, &scenario.system)?;
            draw_event_marker_at(gizmos, &e, pv.pos, scale, duty_cycle);
        }
    }
    if let Some(t) = p.end() {
        let pv = obj.pv(t, &scenario.system)?;
        draw_square(gizmos, pv.pos, 13.0 * scale.min(1.0), alpha(RED, 0.8));
    }
    Some(())
}

fn draw_maneuver_plan(
    gizmos: &mut Gizmos,
    stamp: Nanotime,
    plan: &ManeuverPlan,
    scale: f32,
) -> Option<()> {
    let color = match plan.kind() {
        ManeuverType::Direct => YELLOW,
        ManeuverType::Hohmann => TEAL,
        ManeuverType::Bielliptic => ORANGE,
    };

    let dist = 20.0;
    let mut t = stamp;
    let mut points = vec![];
    while t < plan.end() {
        let pv = plan.pv(t)?;
        let secs = dist / pv.vel.length();
        t += Nanotime::secs_f32(secs);
        points.push(pv.pos);
    }
    let pv = plan.pv(plan.end())?;
    points.push(pv.pos);
    gizmos.linestrip_2d(points, color);
    draw_diamond(gizmos, pv.pos, 10.0 * scale, color);

    Some(())
}

fn draw_scale_indicator(gizmos: &mut Gizmos, cam: &CameraState) {
    let width = 300.0;
    let center = Vec2::new(cam.window_dims.x / 2.0, cam.window_dims.y - 20.0);

    let p1 = center + Vec2::X * width;
    let p2 = center - Vec2::X * width;

    let b = cam.game_bounds();
    let c = cam.window_bounds();

    let u1 = c.map(b, p1);
    let u2 = c.map(b, p2);

    let map = |p: Vec2| c.map(b, p);

    let color = alpha(WHITE, 0.3);

    let mut draw_at = |s: f32, weight: f32| {
        let h = 6.0 * weight;
        if h < 0.5 {
            return;
        }
        let t = map(center + Vec2::new(s, h));
        let b = map(center + Vec2::new(s, -h));
        gizmos.line_2d(t, b, color);
    };

    draw_at(0.0, 1.0);

    for power in -3..7 {
        let size = 10.0f32.powi(power);
        let ds = size / cam.actual_scale;
        let weight = (ds * 10.0 / width).min(1.0);
        let mut s = 0.0;
        s += ds;
        for _ in 0..100 {
            if s > width {
                break;
            }
            draw_at(s, weight);
            draw_at(-s, weight);
            s += ds;
        }
    }

    gizmos.line_2d(u1, u2, color);
}

fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let stamp = state.sim_time;

    draw_scale_indicator(&mut gizmos, &state.camera);

    for p in &state.control_points {
        draw_circle(
            &mut gizmos,
            *p,
            6.0 * state.camera.actual_scale,
            alpha(GRAY, 0.4),
        );
    }

    if let Some((parent, orbit)) = state.target_orbit() {
        if let Some(pv) = state
            .scenario
            .lup(parent, state.sim_time)
            .map(|lup| lup.pv())
        {
            draw_orbit(&mut gizmos, &orbit, pv.pos, alpha(RED, 0.2));
        }
    }

    for id in state.scenario.all_ids() {
        if let Some(lup) = state.scenario.lup(id, stamp) {
            let center = lup.pv().pos;

            if state.track_list.contains(&id) {
                if let Some(body) = lup.body() {
                    for r in [1.2, 1.25, 1.3] {
                        draw_circle(&mut gizmos, center, body.radius * r, alpha(ORANGE, 0.4));
                    }
                }
            }

            let padding = 40.0 * state.camera.actual_scale.min(1.0);
            let w = if let Some(body) = lup.body() {
                2.0 * body.radius + padding
            } else {
                padding
            };
            let aabb = AABB::new(center, Vec2::splat(w));
            if state
                .camera
                .mouse_pos()
                .map(|p| aabb.contains(p))
                .unwrap_or(false)
            {
                draw_aabb(&mut gizmos, aabb, GRAY);
            }
        }
    }

    for ctrl in &state.controllers {
        if state.track_list.contains(&ctrl.target) {
            draw_controller(&mut gizmos, state.sim_time, ctrl, state.camera.actual_scale);
        }
    }

    if state.show_animations && state.track_list.len() < 6 {
        for id in &state.track_list {
            draw_event_animation(
                &mut gizmos,
                &state.scenario,
                *id,
                state.sim_time,
                state.camera.actual_scale,
                state.duty_cycle_high,
            );
        }
    }

    draw_camera_controls(&mut gizmos, &state);

    draw_scenario(
        &mut gizmos,
        &state.scenario,
        stamp,
        state.camera.actual_scale,
        state.show_orbits,
        &state.track_list,
        state.duty_cycle_high,
    );

    draw_highlighted_objects(&mut gizmos, &state);
}

fn draw_mouse_state(
    mouse: Single<&MouseState>,
    cam: Single<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    let points = [
        (mouse.current_world(*cam), RED),
        (mouse.left_world(*cam), BLUE),
        (mouse.right_world(*cam), GREEN),
        (mouse.middle_world(*cam), YELLOW),
    ];

    for (p, c) in points {
        if let Some(p) = p {
            draw_circle(&mut gizmos, p, 8.0 * cam.1.scale().z, c);
        }
    }
}
