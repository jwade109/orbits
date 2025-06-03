#![allow(dead_code)]

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use starling::prelude::*;

use crate::graph::*;
use crate::mouse::{FrameId, MouseButt};
use crate::notifications::*;
use crate::onclick::OnClick;
use crate::planetary::GameState;
use crate::scenes::*;

pub fn draw_cross(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let dx = Vec2::new(size, 0.0);
    let dy = Vec2::new(0.0, size);
    gizmos.line_2d(p - dx, p + dx, color);
    gizmos.line_2d(p - dy, p + dy, color);
}

pub fn draw_x(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size / 2.0;
    gizmos.line_2d(p + Vec2::new(-s, -s), p + Vec2::new(s, s), color);
    gizmos.line_2d(p + Vec2::new(s, -s), p + Vec2::new(-s, s), color);
}

pub fn draw_square(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos.rect_2d(
        Isometry2d::from_translation(p),
        Vec2::new(size, size),
        color,
    );
}

pub fn draw_diamond(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size / 2.0;
    let pts = [0.0, PI / 2.0, PI, -PI / 2.0, 0.0].map(|a| p + rotate(Vec2::X * s, a));
    gizmos.linestrip_2d(pts, color);
}

pub fn draw_triangle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let s = size;
    let pts =
        [0.0, 1.0 / 3.0, 2.0 / 3.0, 0.0].map(|a| p + rotate(Vec2::X * s, a * 2.0 * PI + PI / 2.0));
    gizmos.linestrip_2d(pts, color);
}

pub fn draw_circle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos
        .circle_2d(Isometry2d::from_translation(p), size, color)
        .resolution(200);
}

pub fn draw_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    gizmos.rect_2d(Isometry2d::from_translation(aabb.center), aabb.span, color);
}

pub fn fill_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    for t in linspace(0.0, 1.0, 10) {
        let s = aabb.from_normalized(Vec2::new(t, 0.0));
        let n = aabb.from_normalized(Vec2::new(t, 1.0));
        let w = aabb.from_normalized(Vec2::new(0.0, t));
        let e = aabb.from_normalized(Vec2::new(1.0, t));

        gizmos.line_2d(w, e, color);
        gizmos.line_2d(s, n, color);
    }
}

pub fn draw_and_fill_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    fill_aabb(gizmos, aabb, color);
    draw_aabb(gizmos, aabb, color);
}

fn draw_region(
    gizmos: &mut Gizmos,
    region: Region,
    ctx: &impl CameraProjection,
    color: Srgba,
    origin: Vec2,
) {
    match region {
        Region::AABB(aabb) => {
            let p1 = ctx.w2c(aabb.lower());
            let p2 = ctx.w2c(aabb.upper());
            draw_aabb(gizmos, AABB::from_arbitrary(p1, p2), color)
        }
        Region::OrbitRange(a, b) => {
            draw_orbit(gizmos, &a, origin, color, ctx);
            draw_orbit(gizmos, &b, origin, color, ctx);
            for angle in linspace(0.0, 2.0 * PI, 40) {
                let u = rotate(Vec2::X, angle);
                let p1 = origin + u * a.radius_at_angle(angle as f64) as f32;
                let p2 = origin + u * b.radius_at_angle(angle as f64) as f32;
                gizmos.line_2d(p1, p2, color.with_alpha(color.alpha * 0.2));
            }
        }
        Region::NearOrbit(orbit, dist) => {
            draw_orbit(gizmos, &orbit, origin, color, ctx);
            for angle in linspace(0.0, 2.0 * PI, 40) {
                let u = rotate(Vec2::X, angle);
                let r = orbit.radius_at_angle(angle as f64) as f32;
                let p1 = (r + dist) * u;
                let p2 = (r - dist) * u;
                let p1 = ctx.w2c(p1);
                let p2 = ctx.w2c(p2);
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

fn draw_orbit(
    gizmos: &mut Gizmos,
    orb: &SparseOrbit,
    origin: Vec2,
    color: Srgba,
    ctx: &impl CameraProjection,
) {
    if orb.will_escape() {
        let ta = if orb.is_hyperbolic() {
            let hrta = hyperbolic_range_ta(orb.ecc() as f32);
            linspace(-0.999 * hrta, 0.999 * hrta, 1000)
        } else {
            linspace(-PI, PI, 1000)
        };

        let points: Vec<_> = ta
            .iter()
            .filter_map(|t| {
                let p = orb.position_at(*t as f64);
                if p.length() > orb.body.soi as f64 {
                    return None;
                }
                Some(ctx.w2c(origin + p.as_vec2()))
            })
            .collect();
        gizmos.linestrip_2d(points, color);
    } else {
        let b = orb.semi_minor_axis();
        let center = origin + (orb.periapsis() + orb.apoapsis()).as_vec2() / 2.0;
        let center = ctx.w2c(center);
        let iso = Isometry2d::new(center, (orb.arg_periapsis as f32).into());

        let res = orb.semi_major_axis.clamp(40.0, 300.0) as u32;

        gizmos
            .ellipse_2d(
                iso,
                Vec2::new(orb.semi_major_axis as f32, b as f32) * ctx.scale(),
                color,
            )
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
    draw_orbit(
        gizmos,
        &orbit.1,
        pv.pos_f32(),
        color,
        &state.orbital_context,
    );
    Some(())
}

fn draw_orbit_between(
    gizmos: &mut Gizmos,
    orb: &SparseOrbit,
    origin: Vec2,
    color: Srgba,
    start: Nanotime,
    end: Nanotime,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let mut points: Vec<_> = orb.sample_pos(start, end, 100.0, origin)?;
    points.iter_mut().for_each(|p| {
        *p = ctx.w2c(*p);
    });
    gizmos.linestrip_2d(points, color);
    Some(())
}

fn draw_planets(
    gizmos: &mut Gizmos,
    planet: &PlanetarySystem,
    stamp: Nanotime,
    origin: Vec2,
    ctx: &OrbitalContext,
) {
    let a = match ctx.draw_mode {
        DrawMode::Default => 0.1,
        _ => 0.8,
    };

    let screen_origin = ctx.w2c(origin);

    draw_circle(
        gizmos,
        screen_origin,
        planet.body.radius * ctx.scale(),
        GRAY.with_alpha(a),
    );

    if ctx.draw_mode == DrawMode::Default {
        draw_circle(
            gizmos,
            screen_origin,
            planet.body.soi * ctx.scale(),
            GRAY.with_alpha(a),
        );
    } else {
        for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
            draw_circle(
                gizmos,
                screen_origin,
                planet.body.soi * ds * ctx.scale(),
                ORANGE.with_alpha(a),
            );
        }
    }

    for (orbit, pl) in &planet.subsystems {
        if let Some(pv) = orbit.pv(stamp).ok() {
            draw_orbit(gizmos, orbit, origin, GRAY.with_alpha(a / 2.0), ctx);
            draw_planets(gizmos, pl, stamp, origin + pv.pos_f32(), ctx)
        }
    }
}

fn draw_propagator(
    gizmos: &mut Gizmos,
    state: &GameState,
    prop: &Propagator,
    with_event: bool,
    color: Srgba,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let (_, parent_pv, _, _) = state
        .scenario
        .planets()
        .lookup(prop.parent(), state.sim_time)?;

    draw_orbit(gizmos, &prop.orbit.1, parent_pv.pos_f32(), color, ctx);
    if with_event {
        if let Some((t, e)) = prop.stamped_event() {
            let pv_end = parent_pv + prop.pv(t)?;
            draw_event(
                gizmos,
                state.scenario.planets(),
                &e,
                t,
                state.wall_time,
                pv_end.pos_f32(),
                ctx,
            );
        }
    }
    Some(())
}

pub fn draw_vehicle(gizmos: &mut Gizmos, vehicle: &Vehicle, pos: Vec2, scale: f32, angle: f32) {
    for (p, rot, part) in &vehicle.parts {
        let dims = meters_with_rotation(*rot, part);
        let center = rotate(p.as_vec2() / PIXELS_PER_METER + dims / 2.0, angle) * scale;
        let obb = OBB::new(
            AABB::from_arbitrary(scale * -dims / 2.0, scale * dims / 2.0),
            angle,
        )
        .offset(center + pos);
        let color = match part.data.layer {
            PartLayer::Exterior => YELLOW,
            PartLayer::Internal => GRAY,
            PartLayer::Structural => WHITE,
        };
        draw_obb(gizmos, &obb, color);
    }

    for thruster in vehicle.thrusters() {
        let p1 = pos + rotate(thruster.pos * scale, angle);
        let u = rotate(-Vec2::X, thruster.angle + angle);
        let v = rotate(u, PI / 2.0);
        let p2 = p1 + (u * thruster.proto.length + v * thruster.proto.length / 5.0) * scale;
        let p3 = p1 + (u * thruster.proto.length - v * thruster.proto.length / 5.0) * scale;
        gizmos.linestrip_2d([p1, p2, p3, p1], ORANGE);

        if thruster.is_thrusting() {
            for s in linspace(0.0, 1.0, 13) {
                let length = thruster.proto.length * rand(1.3, 2.5);
                let p4 = p2 + (u * 0.7 + v * 0.4) * length * scale;
                let p5 = p3 + (u * 0.7 - v * 0.4) * length * scale;
                let color = if thruster.proto.is_rcs { TEAL } else { RED };
                let u = p2.lerp(p3, s);
                let v = p4.lerp(p5, s);
                gizmos.line_2d(u, v, color);
            }
        }
    }
}

fn draw_rpo(gizmos: &mut Gizmos, state: &GameState, id: OrbiterId, rpo: &RPO) -> Option<()> {
    let ctx = &state.orbital_context;

    let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
    let pv = lup.pv();

    let screen_pos = ctx.w2c(pv.pos_f32());

    draw_circle(gizmos, screen_pos, 15.0, TEAL);

    for km in 1..=5 {
        let r = state.orbital_context.scale() * km as f32;
        draw_circle(gizmos, screen_pos, r, GRAY);
    }

    let d = pv.pos_f32() - ctx.origin();

    for (vpv, vehicle) in &rpo.vehicles {
        let p = (d + vpv.pos_f32() / 1000.0) * ctx.scale();
        draw_square(gizmos, p, 7.0, RED);

        draw_vehicle(gizmos, vehicle, p, ctx.scale() / 1000.0, vehicle.angle());
    }
    Some(())
}

fn draw_prograde_marker(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let mut draw_notch = |a: f32| {
        let start = p + rotate(Vec2::X * 0.5 * size, a);
        let end = p + rotate(Vec2::X * size, a);
        gizmos.line_2d(start, end, color);
    };

    draw_notch(0.0);
    draw_notch(PI / 2.0);
    draw_notch(PI);

    draw_circle(gizmos, p, size * 0.5, color);
}

pub fn make_separation_graph(
    src: &SparseOrbit,
    dst: &SparseOrbit,
    now: Nanotime,
) -> (Graph, Graph, Vec<Vec2>) {
    // t is in hours!
    let mut g = Graph::linspace(0.0, 48.0, 100);
    let mut v = Graph::linspace(0.0, 48.0, 100);

    let teval = tspace(now, now + Nanotime::hours(16), 300)
        .iter()
        .map(|t| t.floor(Nanotime::PER_SEC))
        .collect();

    let pv = apply(&teval, |t| {
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        (p.pos - q.pos).as_vec2()
    });

    let sep = |hours: f32| {
        let t = now + Nanotime::secs_f32(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.pos_f32().distance(q.pos_f32())
    };

    let rvelx = |hours: f32| {
        let t = now + Nanotime::secs_f32(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.vel_f32().x - q.vel_f32().x
    };

    let rvely = |hours: f32| {
        let t = now + Nanotime::secs_f32(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.vel_f32().y - q.vel_f32().y
    };

    g.add_func(sep, WHITE);
    g.add_point(0.0, 0.0, true);
    g.add_point(0.0, 50.0, true);

    v.add_func(rvelx, ORANGE);
    v.add_func(rvely, TEAL);
    v.add_point(0.0, 0.0, true);

    (g, v, pv)
}

pub fn draw_piloting_overlay(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let ctx = &state.orbital_context;

    let piloting = state.piloting()?;

    let lup = state.scenario.lup_orbiter(piloting, state.sim_time)?;

    let vehicle = state.vehicles.get(&piloting)?;

    let window_dims = state.input.screen_bounds.span;
    let rb = vehicle.bounding_radius();
    let r = window_dims.y * 0.2;

    let zoom = 0.8 * r / rb;

    let center = Vec2::new(
        window_dims.x / 2.0 - r * 1.2,
        -window_dims.y / 2.0 + r * 1.2,
    );

    draw_vehicle(gizmos, &vehicle, center, zoom, vehicle.angle());

    draw_counter(gizmos, rb as u64, center + Vec2::Y * r, WHITE);

    // prograde markers, etc
    {
        let pv = lup.pv();
        let angle = pv.vel_f32().to_angle();
        let p = center + rotate(Vec2::X * r * 0.8, angle);
        draw_prograde_marker(gizmos, p, 20.0, GREEN);
    }

    {
        draw_circle(gizmos, center, rb * zoom, RED.with_alpha(0.02));

        let mut rc = 10.0;
        while rc < rb {
            draw_circle(gizmos, center, rc * zoom, GRAY.with_alpha(0.05));
            rc += 10.0;
        }

        draw_cross(gizmos, center, 3.0, RED.with_alpha(0.1));
    }

    let vehicle_screen = ctx.w2c(lup.pv().pos_f32());

    {
        let alpha = -(center - vehicle_screen).to_angle();
        let s = 20.0;
        let x1 = center.x;
        let x2 = vehicle_screen.x;
        let y1 = center.y;
        let y2 = vehicle_screen.y;
        for sign in [-1.0, 1.0] {
            let p1 = Vec2::new(x1 + r * alpha.sin() * sign, y1 + r * alpha.cos() * sign);
            let p2 = Vec2::new(x2 + s * alpha.sin() * sign, y2 + s * alpha.cos() * sign);
            gizmos.line_2d(p1, p2, GREEN.with_alpha(0.2));
        }
        draw_circle(gizmos, vehicle_screen, s, GREEN.with_alpha(0.2));
    }

    let mut draw_pointing_vector = |u: Vec2, color: Srgba| {
        let triangle_width = 13.0;
        let v = rotate(u, PI / 2.0);
        let p1 = center + u * r * 0.7;
        let p2 = p1 + (v - u) * triangle_width;
        let p3 = p2 - v * triangle_width * 2.0;
        gizmos.linestrip_2d([p1, p2, p3, p1], color);
    };

    draw_pointing_vector(vehicle.pointing(), LIME);
    draw_pointing_vector(vehicle.target_pointing(), LIME.with_alpha(0.4));

    draw_circle(gizmos, center, r, GRAY);
    let p = vehicle.fuel_percentage();
    let iso = Isometry2d::from_translation(center);

    if vehicle.low_fuel() {
        if is_blinking(state.wall_time, None) {
            draw_triangle(gizmos, center, 30.0, YELLOW);
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

    arc(vehicle.angular_velocity() / 10.0, 0.93, TEAL);

    Some(())
}

fn draw_orbiter(gizmos: &mut Gizmos, state: &GameState, id: OrbiterId) -> Option<()> {
    let ctx = &state.orbital_context;
    let tracked = state.orbital_context.selected.contains(&id);
    let piloting = state.piloting() == Some(id);
    let targeting = state.targeting() == Some(id);

    let low_fuel = match state.vehicles.get(&id) {
        Some(v) => v.low_fuel(),
        None => false,
    };

    let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
    let pv = lup.pv();
    let obj = lup.orbiter()?;

    let blinking = is_blinking(state.wall_time, pv.pos_f32());

    let screen_pos = ctx.w2c(pv.pos_f32());

    let size = 12.0;
    if blinking && obj.will_collide() {
        draw_circle(gizmos, screen_pos, size, RED);
        draw_circle(gizmos, screen_pos, size + 3.0, RED);
    } else if blinking && obj.has_error() {
        draw_circle(gizmos, screen_pos, size, YELLOW);
        draw_circle(gizmos, screen_pos, size + 3.0, YELLOW);
    } else if blinking && obj.will_change() {
        draw_circle(gizmos, screen_pos, size, TEAL);
    } else if blinking && low_fuel {
        draw_triangle(gizmos, screen_pos, size, BLUE);
    }

    let show_orbits = match ctx.show_orbits {
        ShowOrbitsState::All => true,
        ShowOrbitsState::Focus => tracked || piloting || targeting,
        ShowOrbitsState::None => false,
    };

    if tracked || piloting || targeting {
        for (i, prop) in obj.props().iter().enumerate() {
            let color = if i == 0 {
                if piloting {
                    ORANGE.with_alpha(0.4)
                } else if targeting {
                    TEAL.with_alpha(0.4)
                } else {
                    WHITE.with_alpha(0.02)
                }
            } else {
                TEAL.with_alpha((1.0 - i as f32 * 0.3).max(0.0))
            };
            if show_orbits {
                draw_propagator(gizmos, state, &prop, true, color, ctx);
            }
        }
    } else if show_orbits {
        let prop = obj.propagator_at(state.sim_time)?;
        draw_propagator(gizmos, state, prop, false, GRAY.with_alpha(0.02), ctx);
    }
    Some(())
}

fn draw_scenario(gizmos: &mut Gizmos, state: &GameState) {
    let stamp = state.sim_time;
    let scenario = &state.scenario;
    let ctx = &state.orbital_context;

    draw_planets(gizmos, scenario.planets(), stamp, Vec2::ZERO, ctx);

    _ = scenario
        .orbiter_ids()
        .into_iter()
        .filter_map(|id| draw_orbiter(gizmos, state, id))
        .collect::<Vec<_>>();
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

fn draw_event_marker_at(gizmos: &mut Gizmos, wall_time: Nanotime, event: &EventType, p: Vec2) {
    let blinking = is_blinking(wall_time, p);

    if !blinking {
        match event {
            EventType::NumericalError => return,
            EventType::Collide(_) => return,
            _ => (),
        }
    }

    let color = match event {
        EventType::Collide(_) => {
            draw_x(gizmos, p, 40.0, RED);
            return;
        }
        EventType::NumericalError => YELLOW,
        EventType::Encounter(_) => GREEN,
        EventType::Escape(_) => TEAL,
        EventType::Impulse(_) => PURPLE,
    };

    draw_circle(gizmos, p, 15.0, color.with_alpha(0.8));
    draw_circle(gizmos, p, 6.0, color.with_alpha(0.8));
}

fn draw_event(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    event: &EventType,
    stamp: Nanotime,
    wall_time: Nanotime,
    p: Vec2,
    ctx: &impl CameraProjection,
) -> Option<()> {
    if let EventType::Encounter(id) = event {
        let (body, pv, _, _) = planets.lookup(*id, stamp)?;
        draw_circle(
            gizmos,
            ctx.w2c(pv.pos_f32()),
            body.soi * ctx.scale(),
            ORANGE.with_alpha(0.2),
        );
    }
    draw_event_marker_at(gizmos, wall_time, event, ctx.w2c(p));
    Some(())
}

fn draw_highlighted_objects(gizmos: &mut Gizmos, state: &GameState) {
    let ctx = &state.orbital_context;
    _ = ctx
        .highlighted
        .iter()
        .filter_map(|id| {
            let pv = state.scenario.lup_orbiter(*id, state.sim_time)?.pv();
            draw_circle(gizmos, ctx.w2c(pv.pos_f32()), 20.0, GRAY);
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
    tracked: bool,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let lup = scenario.lup_orbiter(ctrl.target(), stamp)?;
    let parent = lup.parent(stamp)?;
    let craft = lup.pv().pos_f32();

    let parent_lup = scenario.lup_planet(parent, stamp)?;
    let origin = parent_lup.pv().pos_f32();

    let secs = 2;
    let t_start = wall_time.floor(Nanotime::PER_SEC * secs);
    let dt = (wall_time - t_start).to_secs();
    let r = (8.0 + dt * 30.0) * ctx.scale();
    let a = 0.03 * (1.0 - dt / secs as f32).powi(3);

    draw_circle(gizmos, craft, r, GRAY.with_alpha(a));

    if tracked {
        let plan = ctrl.plan()?;
        draw_maneuver_plan(gizmos, stamp, plan, origin, wall_time, ctx)?;
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
    wall_time: Nanotime,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let obj = scenario.lup_orbiter(id, stamp)?.orbiter()?;
    let p = obj.props().last()?;
    let dt = Nanotime::hours(1);
    let mut t = stamp + dt;
    while t < p.end().unwrap_or(stamp + Nanotime::days(5)) {
        let pv = obj.pv(t, scenario.planets())?;
        draw_diamond(gizmos, ctx.w2c(pv.pos_f32()), 11.0, WHITE.with_alpha(0.6));
        t += dt;
    }
    for prop in obj.props() {
        if let Some((t, e)) = prop.stamped_event() {
            let pv = obj.pv(t, scenario.planets())?;
            draw_event_marker_at(gizmos, wall_time, &e, ctx.w2c(pv.pos_f32()));
        }
    }
    if let Some(t) = p.end() {
        let pv = obj.pv(t, scenario.planets())?;
        draw_square(gizmos, ctx.w2c(pv.pos_f32()), 13.0, RED.with_alpha(0.8));
    }
    Some(())
}

fn draw_maneuver_plan(
    gizmos: &mut Gizmos,
    stamp: Nanotime,
    plan: &ManeuverPlan,
    origin: Vec2,
    wall_time: Nanotime,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let anim_dur = Nanotime::secs(2);
    let s = (wall_time % anim_dur).to_secs() / anim_dur.to_secs();

    for s in [s - 1.0, s - 0.5, s, s + 0.5, s + 1.0] {
        let t_anim = plan.start() + plan.duration() * s;
        let t_end: Nanotime = t_anim + plan.duration() * 0.2;
        let positions: Vec<_> = tspace(t_anim, t_end, 30)
            .iter()
            .filter_map(|t| (*t >= stamp).then(|| plan.pv(*t)).flatten())
            .map(|p| ctx.w2c(p.pos_f32() + origin))
            .collect();

        gizmos.linestrip_2d(positions, YELLOW);
    }

    for segment in &plan.segments {
        if segment.end > stamp {
            let pv = plan.pv(segment.end)?;
            let p = ctx.w2c(pv.pos_f32() + origin);
            draw_circle(gizmos, p, 20.0, WHITE);
        }
    }
    draw_orbit(gizmos, &plan.terminal, origin, PURPLE, ctx);
    Some(())
}

fn draw_timeline(gizmos: &mut Gizmos, state: &GameState) {
    let ctx = &state.orbital_context;

    if !state.controllers.iter().any(|c| !c.is_idle()) {
        return;
    }

    let tmin = state.sim_time - Nanotime::secs(1);
    let tmax = state.sim_time + Nanotime::secs(120);

    let window_dims = state.input.screen_bounds.span;

    let width = window_dims.x * 0.5;
    let y_root = 0.0;
    let row_height = 5.0;
    let x_center = 0.0;
    let x_min = x_center - width / 2.0;

    let map = |p: Vec2| p;

    let p_at = |t: Nanotime, row: usize| -> Vec2 {
        let y = y_root - row as f32 * row_height;
        let pmin = map(Vec2::new(x_min, y));
        let pmax = map(Vec2::new(x_min + width, y));
        let s = (t - tmin).to_secs() / (tmax - tmin).to_secs();
        pmin.lerp(pmax, s)
    };

    let mut draw_tick_mark = |t: Nanotime, row: usize, scale: f32, color: Srgba| {
        let p = p_at(t, row);
        let h = Vec2::Y * ctx.scale() * row_height * scale / 2.0;
        gizmos.line_2d(p + h, p - h, color);
    };

    draw_tick_mark(state.sim_time, 0, 1.0, WHITE);

    for (i, ctrl) in state.controllers.iter().enumerate() {
        let plan = match ctrl.plan() {
            Some(p) => p,
            None => continue,
        };

        let alpha = if ctx.selected.contains(&ctrl.target()) {
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
            let size = ctx.scale() * row_height * 0.5;
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

fn draw_scale_indicator(gizmos: &mut Gizmos, state: &GameState) {
    let window_dims = state.input.screen_bounds.span;
    let width = 300.0;
    let center = Vec2::new(0.0, window_dims.y / 2.0 - 24.0);

    // let ctx = &state.orbital_context;

    draw_circle(gizmos, Vec2::ZERO, 10.0, GRAY.with_alpha(0.2));

    // for i in 0..=9 {
    //     let r = 10.0f32.powi(i) * ctx.scale();
    //     let color = if i % 3 == 0 { RED } else { WHITE };
    //     draw_circle(gizmos, Vec2::ZERO, r, color.with_alpha(0.04));
    // }

    let p1 = center + Vec2::X * width;
    let p2 = center - Vec2::X * width;

    let map = |p: Vec2| p;

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
        let ds = size / state.orbital_context.scale();
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

    gizmos.line_2d(p1, p2, color);
}

pub fn draw_counter(gizmos: &mut Gizmos, val: u64, pos: Vec2, color: Srgba) {
    if val == 0 {
        return;
    }

    let h = 12.0;
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

#[allow(unused)]
fn draw_belt_orbits(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let ctx = &state.orbital_context;
    let cursor_orbit = state.cursor_orbit_if_mode();
    // for belt in state.scenario.belts() {
    //     let lup = match state.scenario.lup_planet(belt.parent(), state.sim_time) {
    //         Some(lup) => lup,
    //         None => continue,
    //     };

    //     let origin = lup.pv().pos_f32();

    //     if let Some(orbit) = cursor_orbit {
    //         if orbit.0 == belt.parent() && belt.contains_orbit(&orbit.1) {
    //             draw_orbit(gizmos, &orbit.1, origin, YELLOW, ctx);
    //             draw_diamond(gizmos, orbit.1.periapsis().as_vec2(), 10.0, YELLOW);
    //             draw_diamond(gizmos, orbit.1.apoapsis().as_vec2(), 10.0, YELLOW);
    //         }
    //     }

    //     let count: u64 = state
    //         .scenario
    //         .orbiter_ids()
    //         .filter_map(|id| {
    //             let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
    //             let orbiter = lup.orbiter()?;
    //             let orbit = orbiter.propagator_at(state.sim_time)?.orbit;
    //             if orbit.0 != belt.parent() {
    //                 return None;
    //             }
    //             if belt.contains_orbit(&orbit.1) {
    //                 Some(1)
    //             } else {
    //                 Some(0)
    //             }
    //         })
    //         .sum();

    //     let (_, corner) = belt.position(0.8);

    //     if ctx.scale() < 2.0 {
    //         draw_counter(gizmos, count, origin + corner.as_vec2() * 1.1, WHITE);
    //     }
    // }
    Some(())
}

pub fn draw_notifications(gizmos: &mut Gizmos, state: &GameState) {
    let ctx = &state.orbital_context;

    for notif in &state.notifications {
        let p = match notif.parent {
            None => return,
            Some(ObjectId::Orbiter(id)) => match state.scenario.lup_orbiter(id, state.sim_time) {
                Some(lup) => lup.pv().pos_f32() + notif.offset + notif.jitter,
                None => continue,
            },
            Some(ObjectId::Planet(id)) => match state.scenario.lup_planet(id, state.sim_time) {
                Some(lup) => lup.pv().pos_f32() + notif.offset + notif.jitter,
                None => continue,
            },
        };

        let size = 20.0;
        let s = (state.wall_time - notif.wall_time).to_secs() / notif.duration().to_secs();
        let a = (1.0 - 2.0 * s).max(0.2);

        let p = ctx.w2c(p);

        match notif.kind {
            NotificationType::OrbiterCrashed(_) => {
                draw_diamond(gizmos, p, size, RED.with_alpha(a));
            }
            NotificationType::OrbiterEscaped(_) => {
                draw_diamond(gizmos, p, size, TEAL.with_alpha(a));
            }
            NotificationType::NumericalError(_) => {
                draw_diamond(gizmos, p, size, YELLOW.with_alpha(a));
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
            NotificationType::NotControllable(_) => (),
            NotificationType::OrbitChanged(_) => (),
            NotificationType::Notice(_) => (),
        }
    }
}

pub fn draw_graph(gizmos: &mut Gizmos, graph: &Graph, bounds: AABB) -> Option<()> {
    let map = |p: Vec2| bounds.from_normalized(p);

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
        draw_x(gizmos, map(p), 10.0, WHITE.with_alpha(0.6));
    }

    Some(())
}

pub fn draw_ui_layout(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let wb = state.input.screen_bounds.span;

    for layout in state.ui.layouts() {
        for n in layout.iter() {
            if n.text_content().map(|t| t.is_empty()).unwrap_or(true) {
                continue;
            }
            let aabb = n.aabb_camera(wb);
            let p = state.input.position(MouseButt::Hover, FrameId::Current);
            if p.map(|p| aabb.contains(p)).unwrap_or(false) {
                draw_aabb(gizmos, n.aabb_camera(wb), RED);
            }
        }
    }

    Some(())
}

pub fn draw_orbit_spline(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    if !state.input.is_pressed(KeyCode::KeyP) {
        return None;
    }

    let g = state
        .cursor_orbit_if_mode()
        .map(|o: GlobalOrbit| get_orbit_info_graph(&o.1))
        .unwrap_or(Graph::blank());

    let bounds = state.input.screen_bounds.with_center(Vec2::ZERO);

    draw_graph(gizmos, &g, bounds);
    draw_graph(gizmos, get_lut_graph(), bounds);

    Some(())
}

fn highlight_targeted_vehicle(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let id = state.targeting()?;
    let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
    let pos = lup.pv().pos_f32();
    let c = state.orbital_context.w2c(pos);
    draw_circle(gizmos, c, 10.0, TEAL);
    for km in 1..=5 {
        let r = state.orbital_context.scale() * km as f32;
        draw_circle(gizmos, c, r, GRAY);
    }
    Some(())
}

fn draw_rendezvous_info(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let ctx = &state.orbital_context;
    let pilot = state.piloting()?;
    let target = state.targeting()?;
    let po = state.get_orbit(pilot)?;
    let to = state.get_orbit(target)?;
    let vb = state.input.screen_bounds.span / 2.0;

    let (g, v, mut relpos) = make_separation_graph(&po.1, &to.1, state.sim_time);
    let h = 140.0;
    draw_graph(
        gizmos,
        &g,
        AABB::from_arbitrary(
            vb - Vec2::new(vb.x * 0.7, 200.0),
            vb - Vec2::new(20.0, 200.0 - h),
        ),
    );
    draw_graph(
        gizmos,
        &v,
        AABB::from_arbitrary(
            vb - Vec2::new(vb.x * 0.7, 220.0 + h),
            vb - Vec2::new(20.0, 220.0),
        ),
    );

    {
        let world_radius = ctx.rendezvous_scope_radius.value; // km
        let screen_radius = 150.0;
        let screen_center = vb - Vec2::new(200.0, 550.0);
        let current_world = relpos.first().cloned();
        relpos.iter_mut().for_each(|p| {
            let sep = p.length();
            *p = if sep > world_radius {
                Vec2::NAN
            } else {
                screen_center + *p / world_radius * screen_radius
            }
        });
        draw_circle(gizmos, screen_center, screen_radius, GRAY);
        gizmos.linestrip_2d(relpos, WHITE);
        draw_x(gizmos, screen_center, 6.0, RED);
        if let Some(p) = current_world {
            let p = screen_center + p / world_radius * screen_radius;
            draw_x(gizmos, p, 7.0, TEAL);
        }
    }

    if let Ok(Some((t, pv))) = get_next_intersection(state.sim_time, &po.1, &to.1) {
        let p = ctx.w2c(pv.pos_f32());
        draw_circle(gizmos, p, 20.0, WHITE);
        if let Some(q) = to.1.pv(t).ok() {
            let q = ctx.w2c(q.pos_f32());
            draw_circle(gizmos, q, 20.0, ORANGE);
        }
    }

    Some(())
}

pub fn draw_orbital_view(gizmos: &mut Gizmos, state: &GameState) {
    let ctx = &state.orbital_context;

    draw_scale_indicator(gizmos, state);

    draw_piloting_overlay(gizmos, state);

    for (id, rpo) in &state.rpos {
        draw_rpo(gizmos, state, *id, rpo);
    }

    highlight_targeted_vehicle(gizmos, state);

    draw_rendezvous_info(gizmos, state);

    // draw_timeline(gizmos, &state);

    draw_orbit_spline(gizmos, state);

    if let Some(a) = state.selection_region() {
        draw_region(gizmos, a, ctx, RED, Vec2::ZERO);
    }

    if let Some((m1, m2, corner)) = state.measuring_tape() {
        let m1 = ctx.w2c(m1);
        let m2 = ctx.w2c(m2);
        let corner = ctx.w2c(corner);
        draw_x(gizmos, m1, 12.0, GRAY);
        draw_x(gizmos, m2, 12.0, GRAY);
        gizmos.line_2d(m1, m2, GRAY);
        gizmos.line_2d(m1, corner, GRAY.with_alpha(0.3));
        gizmos.line_2d(m2, corner, GRAY.with_alpha(0.3));
    }

    if let Some((c, a, b)) = state.protractor() {
        let b = b.unwrap_or(a);
        let c = ctx.w2c(c);
        let a = ctx.w2c(a);
        let b = ctx.w2c(b);
        let r1 = c.distance(a);
        let r2 = c.distance(b);
        for p in [a, b, c] {
            draw_x(gizmos, p, 7.0, WHITE);
        }
        draw_circle(gizmos, c, r1, WHITE.with_alpha(0.4));
        draw_circle(gizmos, c, r2, WHITE.with_alpha(0.7));
        gizmos.line_2d(c, a, RED);
        gizmos.line_2d(c, b, GREEN);
        gizmos.line_2d(a, b, GRAY.with_alpha(0.3));
        let angle = (a - c).angle_to(b - c);
        let iso = Isometry2d::new(c, ((a - c).to_angle() - PI / 2.0).into());
        gizmos
            .arc_2d(iso, angle, (r1 * 0.75).min(r2), TEAL)
            .resolution(100);
    }

    for orbit in &state.orbital_context.queued_orbits {
        draw_global_orbit(gizmos, orbit, &state, RED);
    }

    if let Some(orbit) = state
        .current_hover_ui()
        .map(|id| {
            if let OnClick::GlobalOrbit(i) = *id {
                state.orbital_context.queued_orbits.get(i)
            } else {
                None
            }
        })
        .flatten()
    {
        let mut go = *orbit;
        let sparse = &orbit.1;
        let anim_dur: f64 = 2.0;
        let max_radius: f64 = 20.0;

        let mut draw_with_offset = |s: f64| {
            let alpha = if s == 0.0 {
                1.0
            } else {
                (1.0 - s.abs() as f32) * 0.4
            };
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
        let dt = (state.wall_time % Nanotime::secs_f64(anim_dur)).to_secs_f64();
        for off in linspace(0.0, 1.0, 3) {
            let off = off as f64;
            let s = (dt / anim_dur + off) % 1.0;
            draw_with_offset(-s);
            draw_with_offset(s);
        }
    }

    if let Some(orbit) = state.cursor_orbit_if_mode() {
        draw_global_orbit(gizmos, &orbit, &state, ORANGE);
    }

    if let Some(orbit) = state.current_orbit() {
        draw_global_orbit(gizmos, &orbit, &state, TEAL);
    }

    for ctrl in &state.controllers {
        let tracked = state.orbital_context.selected.contains(&ctrl.target());
        draw_controller(
            gizmos,
            state.sim_time,
            state.wall_time,
            ctrl,
            &state.scenario,
            tracked,
            ctx,
        );
    }

    if state.orbital_context.show_animations && state.orbital_context.selected.len() < 6 {
        for id in &state.orbital_context.selected {
            draw_event_animation(
                gizmos,
                &state.scenario,
                *id,
                state.sim_time,
                state.wall_time,
                ctx,
            );
        }
    }

    draw_scenario(gizmos, state);

    draw_x(gizmos, state.light_source(), 20.0, RED.with_alpha(0.2));

    draw_highlighted_objects(gizmos, &state);

    draw_notifications(gizmos, &state);

    draw_belt_orbits(gizmos, &state);
}

fn orthographic_camera_map(p: Vec3, center: Vec3, normal: Vec3, x: Vec3, y: Vec3) -> Vec2 {
    let p = p - center;
    let p = p.reject_from(normal);
    Vec2::new(p.dot(x), p.dot(y))
}

pub fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let gizmos = &mut gizmos;
    GameState::draw_gizmos(gizmos, &state);
}

fn draw_input_state(gizmos: &mut Gizmos, state: &GameState) {
    let points = [
        (MouseButt::Left, BLUE),
        (MouseButt::Right, GREEN),
        (MouseButt::Middle, YELLOW),
    ];

    let offset = state.input.screen_bounds.span / 2.0;
    draw_aabb(gizmos, state.input.screen_bounds.offset(-offset), RED);

    if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
        draw_circle(gizmos, p, 8.0, RED);
    }

    for (b, c) in points {
        let p1 = state.input.position(b, FrameId::Down);
        let p2 = state
            .input
            .position(b, FrameId::Current)
            .or(state.input.position(b, FrameId::Up));

        if let Some((p1, p2)) = p1.zip(p2) {
            gizmos.line_2d(p1, p2, c);
        }

        for fid in [FrameId::Down, FrameId::Up] {
            let age = state.input.age(b, fid, state.wall_time);
            let p = state.input.position(b, fid);
            if let Some((p, age)) = p.zip(age) {
                let dt = age.to_secs();
                let a = (1.0 - dt).max(0.0);
                draw_circle(gizmos, p, 50.0 * age.to_secs(), c.with_alpha(a));
            }
        }
    }
}
