#![allow(dead_code)]

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::gizmos::cross;
use bevy::prelude::*;

use std::collections::HashSet;

use starling::prelude::*;

use crate::camera_controls::CameraState;
use crate::graph::*;
use crate::mouse::{FrameId, MouseButt, MouseState};
use crate::notifications::*;
use crate::planetary::{GameMode, GameState, ShowOrbitsState};
use crate::scenes::{OrbitalScene, Scene, SceneType, TelescopeScene};

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

fn draw_triangle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size;
    let pts =
        [0.0, 1.0 / 3.0, 2.0 / 3.0, 0.0].map(|a| p + rotate(Vec2::X * s, a * 2.0 * PI + PI / 2.0));
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

fn fill_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    for t in linspace(0.0, 1.0, 10) {
        let s = aabb.from_normalized(Vec2::new(t, 0.0));
        let n = aabb.from_normalized(Vec2::new(t, 1.0));
        let w = aabb.from_normalized(Vec2::new(0.0, t));
        let e = aabb.from_normalized(Vec2::new(1.0, t));

        gizmos.line_2d(w, e, color);
        gizmos.line_2d(s, n, color);
    }
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
    // draw_cross(gizmos, obb.0.center, 30.0, color);
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

fn draw_global_orbit(
    gizmos: &mut Gizmos,
    orbit: &GlobalOrbit,
    state: &GameState,
    color: Srgba,
) -> Option<()> {
    let pv = state
        .scenario
        .lup_planet(orbit.0, state.sim_time)
        .map(|lup| lup.pv())?;
    draw_orbit(gizmos, &orbit.1, pv.pos, color);
    Some(())
}

fn draw_orbit_between(
    gizmos: &mut Gizmos,
    orb: &SparseOrbit,
    origin: Vec2,
    color: Srgba,
    start: Nanotime,
    end: Nanotime,
) -> Option<()> {
    let points: Vec<_> = orb.sample_pos(start, end, 10.0, origin)?;
    gizmos.linestrip_2d(points, color);
    Some(())
}

fn draw_planets(
    gizmos: &mut Gizmos,
    planet: &PlanetarySystem,
    stamp: Nanotime,
    origin: Vec2,
    mode: GameMode,
) {
    let a = match mode {
        GameMode::Default => 0.1,
        _ => 0.8,
    };
    draw_circle(gizmos, origin, planet.body.radius, GRAY.with_alpha(a));

    if mode == GameMode::Default {
        draw_circle(gizmos, origin, planet.body.soi, GRAY.with_alpha(a));
    } else {
        for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
            draw_circle(gizmos, origin, planet.body.soi * ds, ORANGE.with_alpha(a));
        }
    }

    for (orbit, pl) in &planet.subsystems {
        if let Some(pv) = orbit.pv(stamp).ok() {
            draw_orbit(gizmos, orbit, origin, GRAY.with_alpha(a / 2.0));
            draw_planets(gizmos, pl, stamp, origin + pv.pos, mode)
        }
    }
}

fn draw_propagator(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    prop: &Propagator,
    stamp: Nanotime,
    wall_time: Nanotime,
    scale: f32,
    with_event: bool,
    color: Srgba,
) -> Option<()> {
    let (_, parent_pv, _, _) = planets.lookup(prop.parent(), stamp)?;

    draw_orbit(gizmos, &prop.orbit.1, parent_pv.pos, color);
    if with_event {
        if let Some((t, e)) = prop.stamped_event() {
            let pv_end = parent_pv + prop.pv(t)?;
            draw_event(gizmos, planets, &e, t, wall_time, pv_end.pos, scale);
        }
    }
    Some(())
}

fn draw_vehicle(gizmos: &mut Gizmos, vehicle: &Vehicle, pos: Vec2, scale: f32) {
    for poly in vehicle.body() {
        let corners: Vec<_> = poly.iter_closed().map(|p| p * scale + pos).collect();
        gizmos.linestrip_2d(corners, WHITE);
    }

    for thruster in vehicle.thrusters() {
        let p1 = rotate(thruster.pos, vehicle.angle()) * scale + pos;
        let u = rotate(-Vec2::X, thruster.angle + vehicle.angle());
        let v = rotate(u, PI / 2.0);
        let p2 = p1 + (u * thruster.length + v * thruster.length / 5.0) * scale;
        let p3 = p1 + (u * thruster.length - v * thruster.length / 5.0) * scale;
        gizmos.linestrip_2d([p1, p2, p3, p1], WHITE);

        if thruster.is_active {
            let p4 = p2 + (u * 0.7 + v * 0.2) * thruster.length * scale;
            let p5 = p3 + (u * 0.7 - v * 0.2) * thruster.length * scale;
            let color = if thruster.is_rcs { TEAL } else { ORANGE };
            for s in linspace(0.0, 1.0, 13) {
                let u = p2.lerp(p3, s);
                let v = p4.lerp(p5, s);
                gizmos.line_2d(u, v, color);
            }
        }
    }
}

fn draw_piloting_overlay(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let piloting = state.piloting()?;

    let lup = state.scenario.lup_orbiter(piloting, state.sim_time)?;
    let orbiter = lup.orbiter()?;

    let rb = orbiter.vehicle.bounding_radius();
    let r = state.camera.window_dims.y * 0.2;

    let zoom = 0.8 * r / rb;

    let center = Vec2::new(state.camera.window_dims.x - r * 1.2, r * 1.2);

    let b = state.camera.world_bounds();
    let c = state.camera.viewport_bounds();

    let map = |p: Vec2| c.map(b, p);

    draw_vehicle(
        gizmos,
        &orbiter.vehicle,
        map(center),
        state.camera.actual_scale * zoom,
    );

    let mut draw_clock = |stamp: Nanotime, color: Srgba| {
        let angle = (stamp % Nanotime::secs_f32(2.0 * PI)).to_secs();
        let u = rotate(Vec2::X, angle);
        gizmos.line_2d(map(center), map(center + u * r), color);
    };

    draw_clock(state.sim_time, PURPLE.with_alpha(0.2));
    draw_clock(state.wall_time, RED.with_alpha(0.2));

    draw_counter(
        gizmos,
        rb as u64,
        map(center + Vec2::Y * r),
        state.camera.actual_scale,
        WHITE,
    );

    {
        draw_circle(
            gizmos,
            map(center),
            rb * state.camera.actual_scale * zoom,
            RED.with_alpha(0.02),
        );

        let mut rc = 10.0;
        while rc < rb {
            draw_circle(
                gizmos,
                map(center),
                rc * state.camera.actual_scale * zoom,
                GRAY.with_alpha(0.05),
            );
            rc += 10.0;
        }

        draw_cross(
            gizmos,
            map(center),
            3.0 * state.camera.actual_scale,
            RED.with_alpha(0.1),
        );
    }

    {
        let s = 20.0;
        let c1 = state.camera.window_dims * 0.5 + s * rotate(Vec2::X, 3.0 * PI / 6.0);
        let c2 = state.camera.window_dims * 0.5 + s * rotate(Vec2::X, 5.0 * PI / 4.0);
        let u1 = center + r * rotate(Vec2::X, 3.0 * PI / 6.0);
        let u2 = center + r * rotate(Vec2::X, 5.0 * PI / 4.0);
        gizmos.line_2d(map(c1), map(u1), GRAY.with_alpha(0.1));
        gizmos.line_2d(map(c2), map(u2), GRAY.with_alpha(0.1));
        draw_circle(
            gizmos,
            lup.pv().pos,
            s * state.camera.actual_scale,
            GRAY.with_alpha(0.1),
        );
    }

    let mut draw_pointing_vector = |u: Vec2, color: Srgba| {
        let triangle_width = 13.0;
        let v = rotate(u, PI / 2.0);
        let p1 = center + u * r * 0.7;
        let p2 = p1 + (v - u) * triangle_width;
        let p3 = p2 - v * triangle_width * 2.0;
        gizmos.linestrip_2d([map(p1), map(p2), map(p3), map(p1)], color);
    };

    draw_pointing_vector(orbiter.vehicle.pointing(), LIME);
    draw_pointing_vector(orbiter.vehicle.target_pointing(), LIME.with_alpha(0.4));

    let r = r * state.camera.actual_scale;
    draw_circle(gizmos, map(center), r, GRAY);

    let p = orbiter.fuel_percentage();
    let iso = Isometry2d::from_translation(map(center));

    if orbiter.low_fuel() {
        if is_blinking(state.wall_time, None) {
            draw_triangle(
                gizmos,
                map(center),
                30.0 * state.camera.actual_scale,
                YELLOW,
            );
        }
    }

    let mut arc = |percent: f32, s: f32, color: Srgba| {
        gizmos
            .arc_2d(iso, percent * 2.0 * PI, s * r, color)
            .resolution(200);
    };

    for s in linspace(0.95, 0.97, 3) {
        arc(p, s, RED);
    }

    arc(orbiter.vehicle.angular_velocity() / 10.0, 0.93, TEAL);

    Some(())
}

fn draw_orbiter(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    obj: &Orbiter,
    stamp: Nanotime,
    wall_time: Nanotime,
    scale: f32,
    show_orbits: ShowOrbitsState,
    tracked: bool,
    piloting: bool,
) -> Option<()> {
    let pv = obj.pv(stamp, planets)?;

    let blinking = is_blinking(wall_time, pv.pos);

    let size = (4.0 * scale).min(10.0);
    if blinking && obj.will_collide() {
        draw_circle(gizmos, pv.pos, size + 10.0 * scale, RED);
        draw_circle(gizmos, pv.pos, size + 16.0 * scale, RED);
    } else if blinking && obj.has_error() {
        draw_circle(gizmos, pv.pos, size + 10.0 * scale, YELLOW);
        draw_circle(gizmos, pv.pos, size + 16.0 * scale, YELLOW);
    } else if blinking && obj.will_change() {
        draw_circle(gizmos, pv.pos, size + 7.0 * scale, TEAL);
    } else if blinking && obj.low_fuel() {
        draw_triangle(gizmos, pv.pos, size + 20.0 * scale, BLUE);
    }

    let show_orbits = match show_orbits {
        ShowOrbitsState::All => true,
        ShowOrbitsState::Focus => tracked || piloting,
        ShowOrbitsState::None => false,
    };

    if tracked || piloting {
        for (i, prop) in obj.props().iter().enumerate() {
            let color = if i == 0 {
                WHITE.with_alpha(0.02)
            } else {
                TEAL.with_alpha((1.0 - i as f32 * 0.3).max(0.0))
            };
            if show_orbits {
                draw_propagator(gizmos, planets, &prop, stamp, wall_time, scale, true, color);
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
                wall_time,
                scale,
                false,
                GRAY.with_alpha(0.02),
            );
        }
    }
    Some(())
}

fn draw_scenario(
    gizmos: &mut Gizmos,
    scenario: &Scenario,
    stamp: Nanotime,
    wall_time: Nanotime,
    scale: f32,
    show_orbits: ShowOrbitsState,
    track_list: &HashSet<OrbiterId>,
    piloting: Option<OrbiterId>,
    mode: GameMode,
) {
    draw_planets(gizmos, scenario.planets(), stamp, Vec2::ZERO, mode);

    for belt in scenario.belts() {
        let origin = match scenario
            .lup_planet(belt.parent(), stamp)
            .map(|lup| lup.pv().pos)
        {
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
            let obj = scenario.lup_orbiter(id, stamp)?.orbiter()?;
            let is_tracked = track_list.contains(&obj.id());
            let is_piloting = piloting == Some(obj.id());
            draw_orbiter(
                gizmos,
                scenario.planets(),
                obj,
                stamp,
                wall_time,
                scale,
                show_orbits,
                is_tracked,
                is_piloting,
            )
        })
        .collect::<Vec<_>>();

    for GlobalOrbit(id, orbit) in scenario.debris() {
        let lup = match scenario.lup_planet(*id, stamp) {
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
    wall_time: Nanotime,
    event: &EventType,
    p: Vec2,
    scale: f32,
) {
    let blinking = is_blinking(wall_time, p);

    if !blinking {
        return;
    }

    if !blinking {
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
    wall_time: Nanotime,
    p: Vec2,
    scale: f32,
) -> Option<()> {
    if let EventType::Encounter(id) = event {
        let (body, pv, _, _) = planets.lookup(*id, stamp)?;
        draw_circle(gizmos, pv.pos, body.soi, ORANGE.with_alpha(0.2));
    }
    draw_event_marker_at(gizmos, wall_time, event, p, scale);
    Some(())
}

fn draw_highlighted_objects(gizmos: &mut Gizmos, state: &GameState) {
    _ = state
        .highlighted()
        .into_iter()
        .filter_map(|id| {
            let pv = state.scenario.lup_orbiter(id, state.sim_time)?.pv();
            draw_circle(gizmos, pv.pos, 20.0 * state.camera.actual_scale, GRAY);
            Some(())
        })
        .collect::<Vec<_>>();
}

fn draw_controller(
    gizmos: &mut Gizmos,
    stamp: Nanotime,
    wall_time: Nanotime,
    ctrl: &Controller,
    scenario: &Scenario,
    scale: f32,
    tracked: bool,
) -> Option<()> {
    let lup = scenario.lup_orbiter(ctrl.target(), stamp)?;
    let parent = lup.parent(stamp)?;
    let craft = lup.pv().pos;

    let parent_lup = scenario.lup_planet(parent, stamp)?;
    let origin = parent_lup.pv().pos;

    let secs = 2;
    let t_start = wall_time.floor(Nanotime::PER_SEC * secs);
    let dt = (wall_time - t_start).to_secs();
    let r = (8.0 + dt * 30.0) * scale;
    let a = 0.03 * (1.0 - dt / secs as f32).powi(3);

    draw_circle(gizmos, craft, r, GRAY.with_alpha(a));

    if tracked {
        let plan = ctrl.plan()?;
        draw_maneuver_plan(gizmos, stamp, plan, origin, scale, wall_time)?;
    }

    Some(())
}

fn is_blinking(wall_time: Nanotime, pos: impl Into<Option<Vec2>>) -> bool {
    let r = pos.into().unwrap_or(Vec2::ZERO).length();
    let clock = (wall_time % Nanotime::secs(1)).to_secs();
    let offset = (r / 5000. - clock * 2.0 * PI).sin();
    offset >= 0.0
}

fn draw_event_animation(
    gizmos: &mut Gizmos,
    scenario: &Scenario,
    id: OrbiterId,
    stamp: Nanotime,
    scale: f32,
    wall_time: Nanotime,
) -> Option<()> {
    let obj = scenario.lup_orbiter(id, stamp)?.orbiter()?;
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
            draw_event_marker_at(gizmos, wall_time, &e, pv.pos, scale);
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
    wall_time: Nanotime,
) -> Option<()> {
    let anim_dur = Nanotime::secs(2);
    let s = (wall_time % anim_dur).to_secs() / anim_dur.to_secs();

    for s in [s - 1.0, s - 0.5, s, s + 0.5, s + 1.0] {
        let t_anim = plan.start() + plan.duration() * s;
        let t_end: Nanotime = t_anim + plan.duration() * 0.2;
        let positions: Vec<_> = tspace(t_anim, t_end, 30)
            .iter()
            .filter_map(|t| (*t >= stamp).then(|| plan.pv(*t)).flatten())
            .map(|p| p.pos + origin)
            .collect();

        gizmos.linestrip_2d(positions, YELLOW);
    }

    let color = YELLOW.with_alpha(0.03);
    for segment in &plan.segments {
        draw_orbit_between(
            gizmos,
            &segment.orbit,
            origin,
            color,
            segment.start.max(stamp),
            segment.end,
        );
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

        let alpha = if state.track_list.contains(&ctrl.target()) {
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
        if nth_digit == 0 {
            let p = Vec2::new(0.0, y);
            draw_x(gizmos, pos + p + Vec2::splat(h / 2.0), r, color);
        }
        val /= 10;
        y += h;
    }
}

fn draw_belt_orbits(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let cursor_orbit = state.right_cursor_orbit();
    for belt in state.scenario.belts() {
        let lup = match state.scenario.lup_planet(belt.parent(), state.sim_time) {
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
                let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
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
        let p = match notif.parent {
            ObjectId::Orbiter(id) => match state.scenario.lup_orbiter(id, state.sim_time) {
                Some(lup) => lup.pv().pos + notif.offset + notif.jitter,
                None => continue,
            },
            ObjectId::Planet(id) => match state.scenario.lup_planet(id, state.sim_time) {
                Some(lup) => lup.pv().pos + notif.offset + notif.jitter,
                None => continue,
            },
        };

        let size = 20.0 * state.camera.actual_scale;
        let s = (state.wall_time - notif.wall_time).to_secs() / notif.duration().to_secs();
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
                // draw_circle(gizmos, p, size / 2.0, GREEN.with_alpha(a));
            }
            NotificationType::ManeuverFailed(_) => {
                draw_square(gizmos, p, size, RED.with_alpha(a));
            }
            NotificationType::NotControllable => {}
            NotificationType::Following(_) => {
                let a = 0.7 * (1.0 - s);
                let size = 2.0 * size * (1.0 - s);
                draw_circle(gizmos, p, size, ORANGE.with_alpha(a));
            }
            NotificationType::OrbitChanged(_) => (), // draw_square(gizmos, p, size, TEAL.with_alpha(a)),
        }
    }
}

fn draw_graph(
    gizmos: &mut Gizmos,
    graph: &Graph,
    state: &GameState,
    center: Vec2,
    scale: Vec2,
) -> Option<()> {
    let wb = state.camera.world_bounds();
    let vb = AABB::new(wb.center + wb.span * center, wb.span * scale);

    let map = |p: Vec2| vb.from_normalized(p);

    draw_aabb(gizmos, vb, GRAY.with_alpha(0.05));

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

    for p in graph.points() {
        if !AABB::unit().contains(p) {
            continue;
        }
        let size = 15.0 * state.camera.actual_scale;
        draw_x(gizmos, map(p), size, WHITE.with_alpha(0.6));
    }

    Some(())
}

pub fn draw_ui_layout(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let scene = state.current_scene();

    let vb = state.camera.viewport_bounds();
    let wb = state.camera.world_bounds();
    let map = |aabb: AABB| vb.map_box(wb, aabb);

    let fm = |aabb: AABB| {
        let mut aabb = aabb.flip_y_about(0.0);
        aabb.center += Vec2::Y * vb.span.y;
        map(aabb)
    };

    if let Some(n) = state
        .mouse
        .position(MouseButt::Hover, FrameId::Current)
        .map(|p| Vec2::new(p.x, vb.span.y - p.y))
        .map(|p| scene.ui().at(p))
        .flatten()
    {
        if n.text_content().is_some() {
            draw_aabb(gizmos, fm(n.aabb()), RED);
        }
    }

    Some(())
}

pub fn draw_orbit_spline(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    if !state.show_graph {
        return None;
    }

    let scale = Vec2::splat(0.4);
    let c1 = Vec2::new(-0.26, 0.0);
    let c2 = Vec2::new(0.26, 0.0);

    let g = state
        .right_cursor_orbit()
        .map(|o: GlobalOrbit| get_orbit_info_graph(&o.1))
        .unwrap_or(Graph::blank());

    draw_graph(gizmos, &g, state, c2, scale);

    draw_graph(gizmos, get_lut_graph(), state, c1, scale);

    Some(())
}

pub fn draw_orbital_view(gizmos: &mut Gizmos, state: &GameState, scene: &OrbitalScene) {
    draw_scale_indicator(gizmos, &state.camera);

    draw_piloting_overlay(gizmos, &state);

    // draw_timeline(gizmos, &state);

    draw_orbit_spline(gizmos, &state);

    if let Some(a) = state.selection_region() {
        draw_region(gizmos, a, RED, Vec2::ZERO);
    }

    if let Some((m1, m2, corner)) = state.measuring_tape() {
        draw_x(gizmos, m1, 12.0 * state.camera.actual_scale, GRAY);
        draw_x(gizmos, m2, 12.0 * state.camera.actual_scale, GRAY);
        gizmos.line_2d(m1, m2, GRAY);
        gizmos.line_2d(m1, corner, GRAY.with_alpha(0.3));
        gizmos.line_2d(m2, corner, GRAY.with_alpha(0.3));
    }

    for (t, pos, vel, l) in &state.particles {
        let age = (state.wall_time - *t).to_secs();
        let p = pos + vel * age * state.camera.actual_scale;
        let a = 1.0 - (age / l.to_secs());
        draw_circle(
            gizmos,
            p,
            1.2 * state.camera.actual_scale,
            WHITE.with_alpha(a),
        );
    }

    for orbit in &state.queued_orbits {
        draw_global_orbit(gizmos, orbit, &state, RED);
    }

    if let Some(orbit) = state
        .current_hover_ui()
        .map(|id| {
            if let crate::ui::OnClick::GlobalOrbit(i) = *id {
                state.queued_orbits.get(i)
            } else {
                None
            }
        })
        .flatten()
    {
        let mut go = *orbit;
        let sparse = &orbit.1;
        let anim_dur = 2.0;
        let max_radius = 20.0;

        let mut draw_with_offset = |s: f32| {
            let alpha = if s == 0.0 { 1.0 } else { (1.0 - s.abs()) * 0.4 };
            if let Some(o) = SparseOrbit::new(
                sparse.apoapsis_r() + s * max_radius,
                sparse.periapsis_r() + s * max_radius,
                sparse.arg_periapsis,
                sparse.body,
                sparse.epoch,
                sparse.is_retrograde(),
            ) {
                go.1 = o;
                draw_global_orbit(gizmos, &go, &state, YELLOW.with_alpha(alpha));
            }
        };

        draw_with_offset(0.0);
        let dt = (state.wall_time % Nanotime::secs_f32(anim_dur)).to_secs();
        for off in linspace(0.0, 1.0, 3) {
            let s = (dt / anim_dur + off) % 1.0;
            draw_with_offset(-s);
            draw_with_offset(s);
        }
    }

    if let Some(orbit) = state.right_cursor_orbit() {
        draw_global_orbit(gizmos, &orbit, &state, ORANGE);
    }

    if let Some(orbit) = state.current_orbit() {
        draw_global_orbit(gizmos, &orbit, &state, TEAL);
    }

    for ctrl in &state.controllers {
        let tracked = state.track_list.contains(&ctrl.target());
        draw_controller(
            gizmos,
            state.sim_time,
            state.wall_time,
            ctrl,
            &state.scenario,
            state.camera.actual_scale,
            tracked,
        );
    }

    if state.show_animations && state.track_list.len() < 6 {
        for id in &state.track_list {
            draw_event_animation(
                gizmos,
                &state.scenario,
                *id,
                state.sim_time,
                state.camera.actual_scale,
                state.wall_time,
            );
        }
    }

    draw_scenario(
        gizmos,
        &state.scenario,
        state.sim_time,
        state.wall_time,
        state.camera.actual_scale,
        state.show_orbits,
        &state.track_list,
        state.piloting(),
        state.game_mode,
    );

    draw_x(
        gizmos,
        state.light_source(),
        20.0 * state.camera.actual_scale,
        RED.with_alpha(0.2),
    );

    draw_highlighted_objects(gizmos, &state);

    draw_notifications(gizmos, &state);

    draw_belt_orbits(gizmos, &state);

    if let Some(p) = state
        .mouse
        .world_position(MouseButt::Hover, FrameId::Current)
    {
        draw_counter(
            gizmos,
            state.highlighted().len() as u64,
            p,
            state.camera.actual_scale,
            WHITE,
        );
    }
}

pub fn draw_docking_scenario(
    gizmos: &mut Gizmos,
    state: &GameState,
    _id: &OrbiterId,
) -> Option<()> {
    draw_x(gizmos, Vec2::ZERO, 3.0, RED);

    let scale = 3.0;
    let mut x = 0.0;
    for id in &state.track_list {
        if let Some(orbiter) = state.scenario.orbiter(*id) {
            let r = orbiter.vehicle.bounding_radius();
            x += r * scale;
            draw_vehicle(gizmos, &orbiter.vehicle, Vec2::X * x, scale);
            draw_circle(gizmos, Vec2::X * x, r * scale, GRAY);
            x += r * scale + 10.0 * scale;
        }
    }

    Some(())
}

fn orthographic_camera_map(p: Vec3, center: Vec3, normal: Vec3, x: Vec3, y: Vec3) -> Vec2 {
    let p = p - center;
    let p = p.reject_from(normal);
    Vec2::new(p.dot(x), p.dot(y))
}

fn draw_telescope_view(gizmos: &mut Gizmos, state: &GameState, scene: &TelescopeScene) {
    let center = Vec3::new(0.0, 0.0, 0.0);
    let normal = Vec3::new(
        state.sim_time.to_secs().cos(),
        state.sim_time.to_secs().sin(),
        -0.4,
    )
    .normalize_or_zero();

    let x = normal.cross(Vec3::Z).normalize_or_zero();
    let y = normal.cross(x);

    let map = |p: Vec3| -> Vec2 { orthographic_camera_map(p, center, normal, x, y) };

    draw_cross(gizmos, scene.center, 5.0 * state.camera.actual_scale, GRAY);
    for (star, color, radius) in &state.starfield {
        let star_zero = star.with_z(0.0);
        let p = map(*star);
        let q = map(star_zero);
        draw_circle(gizmos, p, *radius, *color);
        draw_circle(gizmos, q, 2.0, WHITE.with_alpha(0.03));
        gizmos.line_2d(p, q, WHITE.with_alpha(0.01));
    }
}

pub fn draw_scene(gizmos: &mut Gizmos, state: &GameState, scene: &Scene) {
    match scene.kind() {
        SceneType::OrbitalView(scene) => draw_orbital_view(gizmos, state, scene),
        SceneType::DockingView(id) => _ = draw_docking_scenario(gizmos, state, id),
        SceneType::TelescopeView(ti) => draw_telescope_view(gizmos, &state, ti),
        SceneType::MainMenu => {}
    }
}

pub fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let gizmos = &mut gizmos;

    draw_scene(gizmos, &state, state.current_scene());

    draw_ui_layout(gizmos, &state);

    // draw_mouse_state(gizmos, &state.mouse, state.wall_time);
}

fn draw_mouse_state(gizmos: &mut Gizmos, mouse: &MouseState, wall_time: Nanotime) {
    let points = [
        (MouseButt::Left, BLUE),
        (MouseButt::Right, GREEN),
        (MouseButt::Middle, YELLOW),
    ];

    if let Some(p) = mouse.world_position(MouseButt::Hover, FrameId::Current) {
        draw_circle(gizmos, p, 8.0 * mouse.scale(), RED);
    }

    for (b, c) in points {
        let p1 = mouse.world_position(b, FrameId::Down);
        let p2 = mouse
            .world_position(b, FrameId::Current)
            .or(mouse.world_position(b, FrameId::Up));

        if let Some((p1, p2)) = p1.zip(p2) {
            gizmos.line_2d(p1, p2, c);
        }

        for fid in [FrameId::Down, FrameId::Up] {
            let age = mouse.age(b, fid, wall_time);
            let p = mouse.world_position(b, fid);
            if let Some((p, age)) = p.zip(age) {
                let dt = age.to_secs();
                let a = (1.0 - dt).max(0.0);
                draw_circle(
                    gizmos,
                    p,
                    50.0 * mouse.scale() * age.to_secs(),
                    c.with_alpha(a),
                );
            }
        }
    }
}
