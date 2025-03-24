#![allow(dead_code)]

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;

use std::collections::HashSet;

use starling::prelude::*;

use crate::camera_controls::CameraState;
use crate::graph::*;
use crate::mouse::MouseState;
use crate::notifications::*;
use crate::planetary::{GameState, ShowOrbitsState};

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

fn draw_region(gizmos: &mut Gizmos, region: Region, color: Srgba, origin: Vec2) {
    match region {
        Region::AABB(aabb) => draw_aabb(gizmos, aabb, color),
        Region::AltitudeRange(a, b) => {
            draw_circle(gizmos, Vec2::ZERO, a, color);
            draw_circle(gizmos, Vec2::ZERO, b, color);
        }
        Region::OrbitRange(a, b) => {
            draw_orbit(gizmos, &a, origin, color);
            draw_orbit(gizmos, &b, origin, color);
            for angle in linspace(0.0, 2.0 * PI, 40) {
                let u = rotate(Vec2::X, angle);
                let p1 = origin + u * a.radius_at_angle(angle);
                let p2 = origin + u * b.radius_at_angle(angle);
                gizmos.line_2d(p1, p2, color.with_alpha(color.alpha * 0.2));
            }
        }
        Region::NearOrbit(orbit, dist) => {
            draw_orbit(gizmos, &orbit, origin, color);
            for angle in linspace(0.0, 2.0 * PI, 40) {
                let u = rotate(Vec2::X, angle);
                let r = orbit.radius_at_angle(angle);
                let p1 = (r + dist) * u;
                let p2 = (r - dist) * u;
                gizmos.line_2d(p1, p2, color.with_alpha(color.alpha * 0.2));
            }
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

fn draw_planets(gizmos: &mut Gizmos, planet: &PlanetarySystem, stamp: Nanotime, origin: Vec2) {
    draw_circle(gizmos, origin, planet.body.radius, GRAY.with_alpha(0.1));
    for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
        draw_circle(gizmos, origin, planet.body.soi * ds, ORANGE.with_alpha(a));
    }

    for (orbit, pl) in &planet.subsystems {
        if let Some(pv) = orbit.pv(stamp).ok() {
            draw_orbit(gizmos, orbit, origin, GRAY.with_alpha(0.4));
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
    let (_, parent_pv, _, _) = planets.lookup(prop.parent(), stamp)?;

    draw_orbit(gizmos, &prop.orbit.1, parent_pv.pos, color);
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
                WHITE.with_alpha(0.02)
            } else {
                TEAL.with_alpha((1.0 - i as f32 * 0.3).max(0.0))
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
                GRAY.with_alpha(0.02),
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
    track_list: &HashSet<ObjectId>,
    duty_cycle: bool,
) {
    draw_planets(gizmos, scenario.planets(), stamp, Vec2::ZERO);

    for belt in scenario.belts() {
        let origin = match scenario.lup(belt.parent(), stamp).map(|lup| lup.pv().pos) {
            Some(p) => p,
            None => continue,
        };

        let region = belt.region();
        draw_region(gizmos, region, GRAY.with_alpha(0.1), origin);
    }

    _ = scenario
        .orbiter_ids()
        .into_iter()
        .filter_map(|id| {
            let obj = scenario.lup(id, stamp)?.orbiter()?;
            let is_tracked = track_list.contains(&obj.id());
            draw_object(
                gizmos,
                scenario.planets(),
                obj,
                stamp,
                scale,
                show_orbits,
                is_tracked,
                duty_cycle,
            )
        })
        .collect::<Vec<_>>();

    for GlobalOrbit(id, orbit) in scenario.debris() {
        let lup = match scenario.lup(*id, stamp) {
            Some(lup) => lup,
            None => continue,
        };

        let pv = match orbit.pv(stamp).ok() {
            Some(pv) => pv,
            None => continue,
        };

        draw_circle(gizmos, pv.pos + lup.pv().pos, 2.0 * scale, WHITE);
    }
}

fn draw_scalar_field_cell(
    gizmos: &mut Gizmos,
    scalar_field: &impl Fn(Vec2) -> f32,
    center: Vec2,
    step: f32,
    levels: &[i32],
) {
    // draw_square(gizmos, center, step as f32, WHITE.with_alpha(0.001));

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

        gizmos.linestrip_2d(pts, RED.with_alpha(0.03));
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

    draw_circle(gizmos, p, 15.0 * scale, color.with_alpha(0.8));
    draw_circle(gizmos, p, 6.0 * scale, color.with_alpha(0.8));
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
        draw_circle(gizmos, pv.pos, body.soi, ORANGE.with_alpha(0.2));
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

fn draw_controller(
    gizmos: &mut Gizmos,
    stamp: Nanotime,
    ctrl: &Controller,
    scenario: &Scenario,
    scale: f32,
    actual_time: Nanotime,
    tracked: bool,
) -> Option<()> {
    let craft = scenario.lup(ctrl.target, stamp)?.pv().pos;

    let secs = 2;
    let t_start = actual_time.floor(Nanotime::PER_SEC * secs);
    let dt = (actual_time - t_start).to_secs();
    let r = (8.0 + dt * 30.0) * scale;
    let a = (1.0 - dt / secs as f32).powi(3);

    draw_circle(gizmos, craft, r, GRAY.with_alpha(a));

    if tracked {
        let origin = scenario.lup(ctrl.parent()?, stamp)?.pv().pos;
        let plan = ctrl.plan()?;
        draw_maneuver_plan(gizmos, stamp, plan, origin, scale)?;
    }

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
        let pv = obj.pv(t, scenario.planets())?;
        draw_diamond(gizmos, pv.pos, 11.0 * scale.min(1.0), WHITE.with_alpha(0.6));
        t += dt;
    }
    for prop in obj.props() {
        if let Some((t, e)) = prop.stamped_event() {
            let pv = obj.pv(t, scenario.planets())?;
            draw_event_marker_at(gizmos, &e, pv.pos, scale, duty_cycle);
        }
    }
    if let Some(t) = p.end() {
        let pv = obj.pv(t, scenario.planets())?;
        draw_square(gizmos, pv.pos, 13.0 * scale.min(1.0), RED.with_alpha(0.8));
    }
    Some(())
}

fn draw_maneuver_plan(
    gizmos: &mut Gizmos,
    stamp: Nanotime,
    plan: &ManeuverPlan,
    origin: Vec2,
    scale: f32,
) -> Option<()> {
    let color = YELLOW;
    for segment in &plan.segments {
        draw_orbit(gizmos, &segment.orbit, origin, color);
        if segment.end > stamp {
            let pv = plan.pv(segment.end)?;
            draw_diamond(gizmos, origin + pv.pos, 10.0 * scale, color);
        }
    }
    draw_orbit(gizmos, &plan.terminal, origin, color);
    Some(())
}

fn draw_timeline(gizmos: &mut Gizmos, state: &GameState) {
    if !state.controllers.iter().any(|c| !c.is_idle()) {
        return;
    }

    let tmin = state.sim_time - Nanotime::secs(1);
    let tmax = state.sim_time + Nanotime::secs(120);

    let b = state.camera.world_bounds();
    let c = state.camera.viewport_bounds();

    let width = state.camera.window_dims.x * 0.5;
    let y_root = state.camera.window_dims.y - 40.0;
    let row_height = 5.0;
    let x_center = state.camera.window_dims.x / 2.0;
    let x_min = x_center - width / 2.0;

    let map = |p: Vec2| c.map(b, p);

    let p_at = |t: Nanotime, row: usize| -> Vec2 {
        let y = y_root - row as f32 * row_height;
        let pmin = map(Vec2::new(x_min, y));
        let pmax = map(Vec2::new(x_min + width, y));
        let s = (t - tmin).to_secs() / (tmax - tmin).to_secs();
        pmin.lerp(pmax, s)
    };

    // gizmos.line_2d(p_at(tmin, 0), p_at(tmax, 0), WHITE.with_alpha(0.3));

    let mut draw_tick_mark = |t: Nanotime, row: usize, scale: f32, color: Srgba| {
        let p = p_at(t, row);
        let h = Vec2::Y * state.camera.actual_scale * row_height * scale / 2.0;
        gizmos.line_2d(p + h, p - h, color);
    };

    draw_tick_mark(state.sim_time, 0, 1.0, WHITE);

    // let mut t = tmin.ceil(Nanotime::PER_SEC);
    // while t < tmax {
    //     draw_tick_mark(t, 0, 0.3, WHITE.with_alpha(0.3));
    //     t += tick_dur;
    // }

    for (i, ctrl) in state.controllers.iter().enumerate() {
        let plan = match ctrl.plan() {
            Some(p) => p,
            None => continue,
        };

        let alpha = if state.track_list.contains(&ctrl.target) {
            1.0
        } else {
            0.2 / (i + 1) as f32
        };

        for segment in &plan.segments {
            if segment.end < tmin || segment.start > tmax {
                continue;
            }
            let p1 = p_at(segment.start.max(tmin), i);
            let p2 = p_at(segment.end.min(tmax), i);
            gizmos.line_2d(p1, p2, BLUE.with_alpha(alpha * 0.3));
            let size = state.camera.actual_scale * row_height * 0.5;
            for t in [segment.start, segment.end] {
                if t < tmin || t > tmax {
                    continue;
                }
                let p = p_at(t, i);
                draw_diamond(gizmos, p, size, WHITE.with_alpha(alpha));
            }
        }
    }
}

fn draw_scale_indicator(gizmos: &mut Gizmos, cam: &CameraState) {
    let width = 300.0;
    let center = Vec2::new(cam.window_dims.x / 2.0, cam.window_dims.y - 20.0);

    let p1 = center + Vec2::X * width;
    let p2 = center - Vec2::X * width;

    let b = cam.world_bounds();
    let c = cam.viewport_bounds();

    let u1 = c.map(b, p1);
    let u2 = c.map(b, p2);

    let map = |p: Vec2| c.map(b, p);

    let color = WHITE.with_alpha(0.3);

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

pub fn draw_counter(gizmos: &mut Gizmos, val: u64, pos: Vec2, scale: f32, color: Srgba) {
    if val == 0 {
        return;
    }

    let h = 10.0 * scale;
    let r = h * 0.8;

    let mut val = val;

    let mut y = 0.0;

    gizmos.line_2d(pos, pos + Vec2::X * h * 10.0, color);

    while val > 0 {
        let nth_digit = val % 10;
        for xn in 0..nth_digit {
            let p = Vec2::new(xn as f32 * h, y);
            draw_circle(gizmos, pos + p + Vec2::splat(h / 2.0), r / 2.0, color);
        }
        val /= 10;
        y += h;
    }
}

fn draw_belt_orbits(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let cursor_orbit = state.right_cursor_orbit();
    for belt in state.scenario.belts() {
        let lup = match state.scenario.lup(belt.parent(), state.sim_time) {
            Some(lup) => lup,
            None => continue,
        };

        let origin = lup.pv().pos;

        if let Some(orbit) = cursor_orbit {
            if orbit.0 == belt.parent() && belt.contains_orbit(&orbit.1) {
                draw_orbit(gizmos, &orbit.1, origin, YELLOW);
                draw_diamond(gizmos, orbit.1.periapsis(), 10.0, YELLOW);
                draw_diamond(gizmos, orbit.1.apoapsis(), 10.0, YELLOW);
            }
        }

        let count: u64 = state
            .scenario
            .orbiter_ids()
            .filter_map(|id| {
                let lup = state.scenario.lup(id, state.sim_time)?;
                let orbiter = lup.orbiter()?;
                let orbit = orbiter.propagator_at(state.sim_time)?.orbit;
                if orbit.0 != belt.parent() {
                    return None;
                }
                if belt.contains_orbit(&orbit.1) {
                    Some(1)
                } else {
                    Some(0)
                }
            })
            .sum();

        let (_, corner) = belt.position(0.8);

        if state.camera.actual_scale < 2.0 {
            draw_counter(
                gizmos,
                count,
                origin + corner * 1.1,
                state.camera.actual_scale,
                WHITE,
            );
        }
    }
    Some(())
}

pub fn draw_notifications(gizmos: &mut Gizmos, state: &GameState) {
    for notif in &state.notifications {
        let p = match state.scenario.lup(notif.parent, state.sim_time) {
            Some(lup) => lup.pv().pos + notif.offset + notif.jitter,
            None => continue,
        };

        let size = 20.0 * state.camera.actual_scale;
        let s = (state.actual_time - notif.wall_time).to_secs() / notif.duration().to_secs();
        let a = (1.0 - 2.0 * s).max(0.2);

        match notif.kind {
            NotificationType::OrbiterCrashed(_) => {
                draw_diamond(gizmos, p, size, RED.with_alpha(a));
            }
            NotificationType::OrbiterDeleted(_) => {
                draw_x(gizmos, p, size, RED.with_alpha(a));
            }
            NotificationType::ManeuverStarted(_) => {
                draw_diamond(gizmos, p, size, ORANGE.with_alpha(a));
            }
            NotificationType::ManeuverComplete(_) => {
                // TODO fix circle size
                draw_circle(gizmos, p, size / 2.0, GREEN.with_alpha(a));
            }
            NotificationType::ManeuverFailed(_) => {
                draw_square(gizmos, p, size, RED.with_alpha(a));
            }
            NotificationType::Following(_) => {
                let a = 0.7 * (1.0 - s);
                let size = 2.0 * size * (1.0 - s);
                draw_circle(gizmos, p, size, ORANGE.with_alpha(a));
            }
            NotificationType::OrbitChanged(_) => draw_square(gizmos, p, size, TEAL.with_alpha(a)),
        }
    }
}

fn draw_graph(gizmos: &mut Gizmos, graph: &Graph, state: &GameState) -> Option<()> {
    let map = |p: Vec2| state.camera.world_bounds().from_normalized(p);

    for p in graph.points() {
        draw_x(
            gizmos,
            map(p),
            40.0 * state.camera.actual_scale,
            RED.with_alpha(0.2),
        );
    }

    {
        // axes
        let origin = graph.origin();
        let d = origin.with_y(0.0);
        let u = origin.with_y(1.0);
        let l = origin.with_x(0.0);
        let r = origin.with_x(1.0);
        gizmos.line_2d(map(l), map(r), GRAY.with_alpha(0.2));
        gizmos.line_2d(map(d), map(u), GRAY.with_alpha(0.2));
    }

    for signal in graph.signals() {
        let p = signal.points().map(|p| map(p)).collect::<Vec<_>>();
        gizmos.linestrip_2d(p, signal.color());
    }
    Some(())
}

pub fn draw_orbit_spline(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    if !state.show_graph {
        return None;
    }

    if let Some(graph) = state
        .right_cursor_orbit()
        .map(|o| get_lut_error_graph(&o.1))
        .flatten()
    {
        draw_graph(gizmos, &graph, state);
    } else {
        draw_graph(gizmos, get_lut_graph(), state);
    };

    Some(())
}

pub fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let stamp = state.sim_time;

    draw_scale_indicator(&mut gizmos, &state.camera);

    draw_timeline(&mut gizmos, &state);

    draw_orbit_spline(&mut gizmos, &state);

    if let Some(a) = state.selection_region() {
        draw_region(&mut gizmos, a, RED, Vec2::ZERO);
    }

    for p in &state.control_points() {
        draw_circle(
            &mut gizmos,
            *p,
            6.0 * state.camera.actual_scale,
            GRAY.with_alpha(0.4),
        );
    }

    let mut draw_orbit_with_parent = |parent: ObjectId, orbit: &SparseOrbit| {
        if let Some(pv) = state
            .scenario
            .lup(parent, state.sim_time)
            .map(|lup| lup.pv())
        {
            let color = match orbit.is_retrograde() {
                true => TEAL,
                false => RED,
            };
            draw_orbit(&mut gizmos, &orbit, pv.pos, color.with_alpha(0.3));
        }
    };

    for GlobalOrbit(parent, orbit) in &state.queued_orbits {
        draw_orbit_with_parent(*parent, orbit);
    }

    if let Some(GlobalOrbit(parent, orbit)) = state.right_cursor_orbit() {
        draw_orbit_with_parent(parent, &orbit);
    }

    for ctrl in &state.controllers {
        let tracked = state.track_list.contains(&ctrl.target);
        draw_controller(
            &mut gizmos,
            state.sim_time,
            ctrl,
            &state.scenario,
            state.camera.actual_scale,
            state.actual_time,
            tracked,
        );
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

    draw_notifications(&mut gizmos, &state);

    draw_belt_orbits(&mut gizmos, &state);

    if !state.hide_debug {
        draw_mouse_state(&state.mouse, &mut gizmos);
    }

    if let Some(p) = state.mouse.current_world() {
        draw_counter(
            &mut gizmos,
            state.highlighted().len() as u64,
            p,
            state.camera.actual_scale,
            WHITE,
        );
    }
}

fn draw_mouse_state(mouse: &MouseState, gizmos: &mut Gizmos) {
    let points = [
        (mouse.current_world(), RED),
        (mouse.left_world(), BLUE),
        (mouse.right_world(), GREEN),
        (mouse.middle_world(), YELLOW),
        // (mouse.current(), RED),
        // (mouse.left(), BLUE),
        // (mouse.right(), GREEN),
        // (mouse.middle(), YELLOW),
    ];

    for (p, c) in points {
        if let Some(p) = p {
            draw_circle(gizmos, p, 8.0 * mouse.scale(), c);
        }
    }

    // draw_aabb(&mut gizmos, mouse.viewport_bounds(), GREEN);
    // draw_aabb(&mut gizmos, mouse.world_bounds(), GREEN);
}
