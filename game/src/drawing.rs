#![allow(dead_code)]

use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use starling::prelude::*;
use std::collections::HashSet;

use crate::canvas::Canvas;
use crate::game::GameState;
use crate::graph::*;
use crate::input::*;
use crate::notifications::*;
use crate::onclick::OnClick;
use crate::scenes::*;
use crate::z_index::*;

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

pub fn draw_aabb(canvas: &mut Canvas, aabb: AABB, color: Srgba) {
    canvas
        .gizmos
        .rect_2d(Isometry2d::from_translation(aabb.center), aabb.span, color);
}

pub fn fill_aabb(canvas: &mut Canvas, aabb: AABB, color: Srgba) {
    // TODO get rid of this
    for t in linspace(0.0, 1.0, 10) {
        let s = aabb.from_normalized(Vec2::new(t, 0.0));
        let n = aabb.from_normalized(Vec2::new(t, 1.0));
        let w = aabb.from_normalized(Vec2::new(0.0, t));
        let e = aabb.from_normalized(Vec2::new(1.0, t));

        canvas.gizmos.line_2d(w, e, color);
        canvas.gizmos.line_2d(s, n, color);
    }
}

pub fn draw_and_fill_aabb(canvas: &mut Canvas, aabb: AABB, color: Srgba) {
    fill_aabb(canvas, aabb, color);
    draw_aabb(canvas, aabb, color);
}

fn draw_region(
    canvas: &mut Canvas,
    region: Region,
    ctx: &impl CameraProjection,
    color: Srgba,
    origin: DVec2,
) {
    match region {
        Region::AABB(aabb) => {
            let p1 = ctx.w2c(aabb.lower().as_dvec2());
            let p2 = ctx.w2c(aabb.upper().as_dvec2());
            draw_aabb(canvas, AABB::from_arbitrary(p1, p2), color)
        }
        Region::OrbitRange(a, b) => {
            draw_orbit(canvas, &a, origin, color, ctx);
            draw_orbit(canvas, &b, origin, color, ctx);
            for angle in linspace_f64(0.0, 2.0 * PI_64, 40) {
                let u = rotate_f64(DVec2::X, angle);
                let p1 = origin + u * a.radius_at_angle(angle);
                let p2 = origin + u * b.radius_at_angle(angle);
                canvas.gizmos.line_2d(
                    graphics_cast(p1),
                    graphics_cast(p2),
                    color.with_alpha(color.alpha * 0.2),
                );
            }
        }
        Region::NearOrbit(orbit, dist) => {
            draw_orbit(canvas, &orbit, origin, color, ctx);
            for angle in linspace_f64(0.0, 2.0 * PI_64, 40) {
                let u = rotate_f64(DVec2::X, angle);
                let r = orbit.radius_at_angle(angle);
                let p1 = (r + dist) * u;
                let p2 = (r - dist) * u;
                let p1 = ctx.w2c(p1);
                let p2 = ctx.w2c(p2);
                canvas
                    .gizmos
                    .line_2d(p1, p2, color.with_alpha(color.alpha * 0.2));
            }
        }
    }
}

pub fn draw_obb(gizmos: &mut Gizmos, obb: &OBB, color: Srgba) {
    // draw_cross(gizmos, obb.0.center, 30.0, color);
    let mut corners = obb.corners().to_vec();
    corners.push(*corners.get(0).expect("Expected a corner"));
    gizmos.linestrip_2d(corners, color);
}

fn fill_obb(gizmos: &mut Gizmos, obb: &OBB, color: Srgba, pct: f32) {
    let mut obb2 = *obb;

    let n = (pct * 20.0).round() as usize;

    for s in linspace(0.0, pct, n) {
        obb2.0.span = obb.0.span * s;
        draw_obb(gizmos, &obb2, color)
    }
}

pub fn draw_orbit(
    canvas: &mut Canvas,
    orb: &SparseOrbit,
    origin: DVec2,
    color: Srgba,
    ctx: &impl CameraProjection,
) {
    if orb.ecc() > 0.99 {
        // orb.will_escape() {
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
                Some(ctx.w2c(origin + p))
            })
            .collect();
        canvas.gizmos.linestrip_2d(points, color);
    } else {
        let b = orb.semi_minor_axis();
        let center = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
        let center = ctx.w2c(center);
        let rot = Quat::from_rotation_z(orb.arg_periapsis as f32);
        canvas
            .painter
            .set_translation(center.extend(ZOrdering::Orbit.as_f32()));
        canvas.painter.set_rotation(rot);
        canvas
            .painter
            .set_scale(Vec3::new(1.0, (b / orb.semi_major_axis) as f32, 1.0));
        canvas.painter.hollow = true;
        canvas.painter.thickness = 2.0;
        canvas.painter.set_color(color);
        canvas
            .painter
            .circle((orb.semi_major_axis * ctx.scale()) as f32);
        canvas.painter.reset();
    }
}

fn draw_global_orbit(
    canvas: &mut Canvas,
    orbit: &GlobalOrbit,
    state: &GameState,
    color: Srgba,
) -> Option<()> {
    let pv = state
        .universe
        .lup_planet(orbit.0, state.universe.stamp())
        .map(|lup: ObjectLookup<'_>| lup.pv())?;
    draw_orbit(canvas, &orbit.1, pv.pos, color, &state.orbital_context);
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
    let points: Vec<_> = orb
        .sample_pos(start, end, 100.0, origin)?
        .into_iter()
        .map(|p| ctx.w2c(p))
        .collect();
    gizmos.linestrip_2d(points, color);
    Some(())
}

fn draw_planets(
    canvas: &mut Canvas,
    planet: &PlanetarySystem,
    stamp: Nanotime,
    origin: DVec2,
    ctx: &OrbitalContext,
) {
    let a = match ctx.draw_mode {
        DrawMode::Default => 0.1,
        _ => 0.8,
    };

    let screen_origin = ctx.w2c(origin);

    canvas.sprite(
        screen_origin,
        0.0,
        planet.name.clone(),
        ZOrdering::Planet,
        graphics_cast(DVec2::splat(planet.body.radius) * 2.0 * ctx.scale()),
    );

    draw_circle(
        &mut canvas.gizmos,
        screen_origin,
        gcast(planet.body.radius * ctx.scale()),
        GRAY.with_alpha(a),
    );

    if ctx.draw_mode == DrawMode::Default {
        draw_circle(
            &mut canvas.gizmos,
            screen_origin,
            gcast(planet.body.soi * ctx.scale()),
            GRAY.with_alpha(a),
        );
    } else {
        for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
            draw_circle(
                &mut canvas.gizmos,
                screen_origin,
                gcast(planet.body.soi * ds * ctx.scale()),
                ORANGE.with_alpha(a),
            );
        }
    }

    for (orbit, pl) in &planet.subsystems {
        if let Some(pv) = orbit.pv(stamp).ok() {
            draw_orbit(canvas, orbit, origin, GRAY.with_alpha(a / 2.0), ctx);
            draw_planets(canvas, pl, stamp, origin + pv.pos, ctx)
        }
    }
}

fn draw_propagator(
    canvas: &mut Canvas,
    state: &GameState,
    prop: &Propagator,
    with_event: bool,
    color: Srgba,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let (_, parent_pv, _, _) = state
        .universe
        .planets
        .lookup(prop.parent(), state.universe.stamp())?;

    draw_orbit(canvas, &prop.orbit.1, parent_pv.pos, color, ctx);
    if with_event {
        if let Some((t, e)) = prop.stamped_event() {
            let pv_end = parent_pv + prop.pv(t)?;
            draw_event(
                canvas,
                &state.universe.planets,
                &e,
                t,
                state.wall_time,
                pv_end.pos,
                ctx,
            );
        }
    }
    Some(())
}

pub fn draw_thruster(
    gizmos: &mut Gizmos,
    thruster: &ThrusterModel,
    data: &ThrusterInstanceData,
    part_dims: Vec2,
    center: Vec2,
    scale: f32,
    angle: f32,
) {
    // along-plume direction
    let u = rotate(-Vec2::X, angle);

    // cross-plume direction
    let v = rotate(u, PI / 2.0);

    // corners of the business end of the thruster
    let p2 = center + (u * part_dims.x / 2.0 + v * part_dims.y / 2.0) * scale;
    let p3 = center + (u * part_dims.x / 2.0 - v * part_dims.y / 2.0) * scale;

    let c1 = crate::scenes::surface::to_srgba(thruster.primary_color);
    let c2 = crate::scenes::surface::to_srgba(thruster.secondary_color);

    if data.is_thrusting(thruster) {
        let ul = rotate(u, thruster.plume_angle);
        let ur = rotate(u, -thruster.plume_angle);

        for s in linspace(0.0, 1.0, 13) {
            let length = thruster.plume_length
                * rand(0.6, 1.0)
                * data.throttle()
                * ((s - 0.5) * PI).abs().cos();

            let p4 = p2 + ul * length * scale;
            let p5 = p3 + ur * length * scale;

            let color = c1.mix(&c2, rand(0.0, 1.0));
            let u = p2.lerp(p3, s);
            let v = p4.lerp(p5, s);
            gizmos.line_2d(u, v, color);
        }
    }
}

pub fn vehicle_sprite_path(disc: u64) -> String {
    format!("vehicle-{}", disc)
}

pub fn draw_vehicle(
    canvas: &mut Canvas,
    vehicle: &Vehicle,
    pos: Vec2,
    scale: f32,
    angle: f32,
    outline: bool,
    thrusters: bool,
) {
    if outline {
        for (_, part) in vehicle.parts() {
            let color = diagram_color(&part.prototype());
            let color = Srgba::from_f32_array(color);
            let dims = part.dims_meters();
            let center = rotate(part.center_meters(), angle) * scale;
            let obb = OBB::new(
                AABB::from_arbitrary(scale * -dims / 2.0, scale * dims / 2.0),
                angle,
            )
            .offset(center + pos);
            draw_obb(&mut canvas.gizmos, &obb, color);
        }
    }

    let geo = vehicle.aabb().center;

    canvas.sprite(
        pos + rotate(geo, angle) * scale,
        angle,
        vehicle_sprite_path(vehicle.discriminator()),
        ZOrdering::Vehicle,
        vehicle.aabb().span * scale,
    );

    if thrusters {
        for (_, part) in vehicle.parts() {
            let dims = part.prototype().dims_meters();
            if let Some((thruster, data)) = part.as_thruster() {
                draw_thruster(
                    &mut canvas.gizmos,
                    thruster,
                    data,
                    dims,
                    pos + rotate(part.center_meters() * scale, angle),
                    scale,
                    gcast(part.rotation().to_angle()) + angle,
                );
            }
        }
    }
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
) -> (Graph, Graph, Vec<DVec2>) {
    // t is in hours!
    let mut g = Graph::linspace(0.0, 48.0, 100);
    let mut v = Graph::linspace(0.0, 48.0, 100);

    let teval = tspace(now, now + Nanotime::hours(16), 300);

    let pv = apply(&teval, |t| {
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.pos - q.pos
    });

    let sep = |hours| {
        let t = now + Nanotime::secs_f64(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.pos.distance(q.pos)
    };

    let rvelx = |hours| {
        let t = now + Nanotime::secs_f64(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.vel.x - q.vel.x
    };

    let rvely = |hours| {
        let t = now + Nanotime::secs_f64(hours * 3600.0);
        let p = src.pv(t).ok().unwrap_or(PV::NAN);
        let q = dst.pv(t).ok().unwrap_or(PV::NAN);
        p.vel.y - q.vel.y
    };

    g.add_func(sep, WHITE);
    g.add_point(0.0, 0.0, true);
    g.add_point(0.0, 50.0, true);

    v.add_func(rvelx, ORANGE);
    v.add_func(rvely, TEAL);
    v.add_point(0.0, 0.0, true);

    (g, v, pv)
}

pub fn draw_pinned(canvas: &mut Canvas, state: &GameState) -> Option<()> {
    let dims = state.input.screen_bounds.span;
    let leftmost = dims / 2.0 - Vec2::splat(180.0);

    for (i, id) in state.pinned.iter().enumerate() {
        if i > 5 {
            continue;
        }

        let ov = match state.universe.orbital_vehicles.get(id) {
            Some(ov) => ov,
            None => continue,
        };

        let size = 200.0;
        let rb = ov.vehicle().bounding_radius();

        let pos = leftmost - Vec2::X * i as f32 * size * 1.1;

        draw_vehicle(
            canvas,
            &ov.vehicle(),
            pos,
            size / gcast(rb * 2.0),
            gcast(ov.body().angle),
            false,
            true,
        );
        let color = if Some(*id) == state.piloting() {
            TEAL.with_alpha(0.3)
        } else {
            GRAY.with_alpha(0.2)
        };
        draw_circle(&mut canvas.gizmos, pos, size / 2.0, color);
    }
    Some(())
}

pub fn draw_pointing_vector(gizmos: &mut Gizmos, center: Vec2, r: f32, u: Vec2, color: Srgba) {
    let triangle_width = 13.0;
    let v = rotate(u, PI / 2.0);
    let p1 = center + u * r * 0.7;
    let p2 = p1 + (v - u) * triangle_width;
    let p3 = p2 - v * triangle_width * 2.0;
    gizmos.linestrip_2d([p1, p2, p3, p1], color);
}

pub fn draw_arc(
    painter: &mut ShapePainter,
    pos: Vec2,
    z: f32,
    color: Srgba,
    r: f32,
    start: f32,
    end: f32,
) {
    painter.reset();
    painter.set_translation(pos.extend(z));
    painter.set_color(color);
    painter.hollow = true;
    painter.thickness = 12.0;
    painter.cap = Cap::Square;
    painter.arc(r + 6.0, start, end);
}

pub fn draw_piloting_overlay(
    canvas: &mut Canvas,
    state: &GameState,
    pilot: Option<EntityId>,
) -> Option<()> {
    let piloting = pilot?;

    let (vehicle, body, orbit) = if let Some(os) = state.universe.orbital_vehicles.get(&piloting) {
        (os.vehicle(), os.body(), os.current_orbit())
    } else if let Some(sv) = state.universe.surface_vehicles.get(&piloting) {
        (&sv.vehicle, &sv.body, sv.current_orbit())
    } else {
        return None;
    };

    let window_dims = state.input.screen_bounds.span;
    let rb = gcast(vehicle.bounding_radius());
    let r = window_dims.y * 0.2;

    let zoom = 0.8 * r / rb;

    let center = Vec2::new(
        window_dims.x / 2.0 - r * 1.2,
        -window_dims.y / 2.0 + r * 1.2,
    );

    canvas.sprite(
        center,
        0.0,
        "shipscope",
        ZOrdering::Shipscope,
        Vec2::splat(r * 2.0) * 1.1,
    );

    draw_vehicle(
        canvas,
        vehicle,
        center,
        zoom,
        gcast(body.angle),
        false,
        true,
    );

    {
        canvas.painter.reset();
        canvas
            .painter
            .set_translation(center.extend(ZOrdering::HudAngularMomentum.as_f32()));
        canvas.painter.set_color(RED);
        canvas.painter.hollow = true;
        canvas.painter.thickness = 6.0;

        let angles = |am: f32| {
            let am = am.clamp(-1.0, 1.0);
            if am > 0.0 {
                (2.0 * PI - 1.9 * PI * am, 2.0 * PI)
            } else {
                (0.0, -1.9 * PI * am)
            }
        };

        let am_body = (body.angular_velocity / MAX_ANGULAR_VELOCITY) as f32;
        let am_gyro = (vehicle.gyro.angular_velocity / vehicle.gyro.max_angular_velocity) as f32;

        let (start, end) = angles(am_body);

        draw_arc(
            &mut canvas.painter,
            center,
            ZOrdering::HudAngularMomentum.as_f32(),
            RED,
            r,
            start,
            end,
        );

        let (start, end) = angles(am_gyro);

        draw_arc(
            &mut canvas.painter,
            center,
            ZOrdering::HudAngularMomentum.as_f32(),
            GREEN,
            r - 15.0,
            start,
            end,
        );
    }

    // prograde markers, etc
    {
        // let pv = lup.pv();
        // let angle = gcast(pv.vel.to_angle());
        // let p = center + rotate(Vec2::X * r * 0.8, angle);
        // draw_prograde_marker(&mut canvas.gizmos, p, 20.0, GREEN);
    }

    {
        draw_circle(&mut canvas.gizmos, center, rb * zoom, RED.with_alpha(0.02));

        let mut rc = 10.0;
        while rc < rb {
            draw_circle(&mut canvas.gizmos, center, rc * zoom, GRAY.with_alpha(0.05));
            rc += 10.0;
        }

        draw_cross(&mut canvas.gizmos, center, 3.0, RED.with_alpha(0.1));
    }

    draw_circle(&mut canvas.gizmos, center, r, GRAY);

    canvas
        .text(
            format!("{}-type vessel", vehicle.model().to_uppercase()),
            center + Vec2::new(r * 0.4, r + 90.0),
            0.8,
        )
        .anchor_right();

    canvas
        .text(
            format!("{} {}", vehicle.name(), piloting.0),
            center + Vec2::new(r * 0.4, r + 60.0),
            1.2,
        )
        .anchor_right();

    let orbit_str = orbit.map(|o| format!("{}", o));

    canvas
        .text(
            format!("{}\n{:?}", body.pv, orbit_str),
            center + Vec2::new(r * 0.4, r + 150.0),
            0.8,
        )
        .anchor_right();

    let dash_icons = [
        ("low-fuel", "low-fuel-dim", vehicle.low_fuel(), true),
        ("radar", "radar-dim", vehicle.has_radar(), false),
        ("ctrl", "ctrl-dim", !vehicle.is_controllable(), true),
    ];

    let mut icon_pos = center + Vec2::new(r * 0.9, r * 1.1);
    let icon_size = 75.0;
    for (pa, pb, cond, blink) in dash_icons {
        let path = if cond && (!blink || is_blinking(state.wall_time)) {
            pa
        } else {
            pb
        };
        canvas.sprite(
            icon_pos,
            0.0,
            path,
            ZOrdering::HudIcon,
            Vec2::splat(icon_size),
        );
        icon_pos += Vec2::Y * icon_size;
    }

    Some(())
}

fn draw_orbiter(canvas: &mut Canvas, state: &GameState, id: EntityId) -> Option<()> {
    let ctx = &state.orbital_context;
    let tracked = state.orbital_context.selected.contains(&id);
    let piloting = state.piloting() == Some(id);
    let targeting = state.targeting() == Some(id);

    let (parent, vehicle, body, orbit) = if let Some(os) = state.universe.orbital_vehicles.get(&id)
    {
        (os.parent(), os.vehicle(), os.body(), os.current_orbit())
    } else if let Some(sv) = state.universe.surface_vehicles.get(&id) {
        (sv.planet_id, &sv.vehicle, &sv.body, sv.current_orbit())
    } else {
        println!("Failed to draw orbiter {}", id);
        return None;
    };

    let low_fuel = vehicle.low_fuel();
    let is_thrusting = vehicle.is_thrusting();
    let has_radar = vehicle.has_radar();

    let parent_pv = state
        .universe
        .lup_planet(parent, state.universe.stamp())?
        .pv();

    let pv = body.pv + parent_pv;

    let blinking = is_blinking(state.wall_time);

    let screen_pos = ctx.w2c(pv.pos);

    canvas.painter.set_translation(screen_pos.extend(12.0));
    canvas.painter.set_color(WHITE);
    canvas.painter.circle(4.0);

    // let size = 12.0;
    // if blinking && obj.will_collide() {
    //     draw_circle(&mut canvas.gizmos, screen_pos, size, RED);
    //     draw_circle(&mut canvas.gizmos, screen_pos, size + 3.0, RED);
    // } else if blinking && obj.has_error() {
    //     draw_circle(&mut canvas.gizmos, screen_pos, size, YELLOW);
    //     draw_circle(&mut canvas.gizmos, screen_pos, size + 3.0, YELLOW);
    // } else if blinking && obj.will_change() {
    //     draw_circle(&mut canvas.gizmos, screen_pos, size, TEAL);
    // } else if blinking && low_fuel {
    //     draw_triangle(&mut canvas.gizmos, screen_pos, size, BLUE);
    // }

    if has_radar {
        draw_circle(
            &mut canvas.gizmos,
            screen_pos,
            gcast((10.0 * ctx.scale()).max(20.0)),
            TEAL.with_alpha(0.4),
        );
    }

    if is_thrusting {
        draw_diamond(&mut canvas.gizmos, screen_pos, 16.0, RED);
    }

    let show_orbits = match ctx.show_orbits {
        ShowOrbitsState::All => true,
        ShowOrbitsState::Focus => tracked || piloting || targeting,
        ShowOrbitsState::None => false,
    };

    if let Some(orbit) = orbit {
        draw_global_orbit(canvas, &orbit, state, GRAY);
    }

    // if tracked || piloting || targeting {
    //     for (i, prop) in obj.props().iter().enumerate() {
    //         let color = if i == 0 {
    //             if piloting {
    //                 GRAY.with_alpha(0.4)
    //             } else if targeting {
    //                 TEAL.with_alpha(0.4)
    //             } else {
    //                 WHITE.with_alpha(0.02)
    //             }
    //         } else {
    //             TEAL.with_alpha((1.0 - i as f32 * 0.3).max(0.0))
    //         };
    //         if show_orbits {
    //             draw_propagator(canvas, state, &prop, true, color, ctx);
    //         }

    //         if piloting {
    //             if let Some(o) = ov.current_orbit(state.universe.stamp()) {
    //                 draw_global_orbit(canvas, &o, state, ORANGE);
    //             }
    //         }
    //     }
    // } else if show_orbits {
    //     let prop = obj.propagator_at(state.universe.stamp())?;
    //     draw_propagator(canvas, state, prop, false, GRAY.with_alpha(0.02), ctx);
    // }
    Some(())
}

fn draw_scenario(canvas: &mut Canvas, state: &GameState) {
    let stamp = state.universe.stamp();
    let ctx = &state.orbital_context;

    draw_planets(canvas, &state.universe.planets, stamp, DVec2::ZERO, ctx);

    let oids = state.universe.orbital_vehicles.iter().map(|(id, _)| id);
    let sids = state.universe.surface_vehicles.iter().map(|(id, _)| id);

    oids.chain(sids).for_each(|id| {
        draw_orbiter(canvas, state, *id);
    });
}

fn draw_event_marker_at(gizmos: &mut Gizmos, wall_time: Nanotime, event: &EventType, p: Vec2) {
    let blinking = is_blinking(wall_time);

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
    canvas: &mut Canvas,
    planets: &PlanetarySystem,
    event: &EventType,
    stamp: Nanotime,
    wall_time: Nanotime,
    p: DVec2,
    ctx: &impl CameraProjection,
) -> Option<()> {
    if let EventType::Encounter(id) = event {
        let (body, pv, _, _) = planets.lookup(*id, stamp)?;
        draw_circle(
            &mut canvas.gizmos,
            ctx.w2c(pv.pos),
            gcast(body.soi * ctx.scale()),
            ORANGE.with_alpha(0.2),
        );
    }
    draw_event_marker_at(&mut canvas.gizmos, wall_time, event, ctx.w2c(p));
    Some(())
}

fn draw_highlighted_objects(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let ctx = &state.orbital_context;

    let bounds = ctx.selection_bounds?;

    for (_, ov) in &state.universe.orbital_vehicles {
        let pos = ov.pv().pos;
        if bounds.contains(aabb_stopgap_cast(pos)) {
            draw_circle(gizmos, ctx.w2c(pos), 20.0, GRAY);
        }
    }

    Some(())
}

pub fn is_blinking(wall_time: Nanotime) -> bool {
    let clock = (wall_time % Nanotime::secs(1)).to_secs();
    clock >= 0.5
}

fn draw_event_animation(
    gizmos: &mut Gizmos,
    state: &GameState,
    id: EntityId,
    ctx: &impl CameraProjection,
) -> Option<()> {
    let obj = state
        .universe
        .lup_orbiter(id, state.universe.stamp())?
        .orbiter()?;
    let p = obj.props().last()?;
    let dt = Nanotime::hours(1);
    let mut t = state.universe.stamp() + dt;
    while t < p
        .end()
        .unwrap_or(state.universe.stamp() + Nanotime::days(5))
    {
        let pv = obj.pv(t, &state.universe.planets)?;
        draw_circle(gizmos, ctx.w2c(pv.pos), 3.0, WHITE.with_alpha(0.2));
        t += dt;
    }
    for prop in obj.props() {
        if let Some((t, e)) = prop.stamped_event() {
            let pv = obj.pv(t, &state.universe.planets)?;
            draw_event_marker_at(gizmos, state.wall_time, &e, ctx.w2c(pv.pos));
        }
    }
    if let Some(t) = p.end() {
        let pv = obj.pv(t, &state.universe.planets)?;
        draw_square(gizmos, ctx.w2c(pv.pos), 13.0, RED.with_alpha(0.8));
    }
    Some(())
}

fn draw_maneuver_plan(
    canvas: &mut Canvas,
    stamp: Nanotime,
    plan: &ManeuverPlan,
    origin: DVec2,
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
            .map(|p| ctx.w2c(p.pos + origin))
            .collect();

        canvas.gizmos.linestrip_2d(positions, YELLOW);
    }

    for segment in &plan.segments {
        if segment.end > stamp {
            let pv = plan.pv(segment.end)?;
            let p = ctx.w2c(pv.pos + origin);
            draw_circle(&mut canvas.gizmos, p, 20.0, WHITE);
        }
    }
    draw_orbit(canvas, &plan.terminal, origin, PURPLE, ctx);
    Some(())
}

fn draw_scale_indicator(canvas: &mut Canvas, state: &GameState) {
    let window_dims = state.input.screen_bounds.span;
    let width = 300.0;
    let center = Vec2::new(0.0, window_dims.y / 2.0 - 80.0);

    canvas.painter.set_translation(Vec3::ZERO);
    canvas.painter.set_color(GRAY.with_alpha(0.2));
    canvas.painter.circle(10.0);

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
        canvas.gizmos.line_2d(t, b, color);
    };

    draw_at(0.0, 1.0);

    for power in -3..7 {
        let size = 10.0f64.powi(power);
        let ds = gcast(size / state.orbital_context.scale());
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

    canvas.gizmos.line_2d(p1, p2, color);
}

pub fn draw_notifications(gizmos: &mut Gizmos, state: &GameState) {
    let ctx = &state.orbital_context;

    for notif in &state.notifications {
        let p = match notif.parent {
            None => return,
            Some(ObjectId::Orbiter(id)) => {
                match state.universe.lup_orbiter(id, state.universe.stamp()) {
                    Some(lup) => lup.pv().pos + notif.offset + notif.jitter,
                    None => continue,
                }
            }
            Some(ObjectId::Planet(id)) => {
                match state.universe.lup_planet(id, state.universe.stamp()) {
                    Some(lup) => lup.pv().pos + notif.offset + notif.jitter,
                    None => continue,
                }
            }
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

pub fn draw_graph(
    canvas: &mut Canvas,
    graph: &Graph,
    bounds: AABB,
    input: Option<&InputState>,
) -> Option<()> {
    let map = |p: DVec2| bounds.from_normalized(aabb_stopgap_cast(p));

    {
        // axes
        let origin = graph.origin();
        let d = origin.with_y(0.0);
        let u = origin.with_y(1.0);
        let l = origin.with_x(0.0);
        let r = origin.with_x(1.0);
        canvas.gizmos.line_2d(map(l), map(r), GRAY.with_alpha(0.2));
        canvas.gizmos.line_2d(map(d), map(u), GRAY.with_alpha(0.2));
    }

    if let Some(p) = input
        .map(|i| i.position(MouseButt::Hover, FrameId::Current))
        .flatten()
    {
        if bounds.contains(p) {
            canvas.text("Graph!".to_uppercase(), p, 0.7);
        }
    }

    for signal in graph.signals() {
        let p = signal.points().map(|p| map(p)).collect::<Vec<_>>();
        canvas.gizmos.linestrip_2d(p, signal.color());
    }

    for p in graph.points() {
        if !AABB::unit().contains(aabb_stopgap_cast(p)) {
            continue;
        }
        draw_x(&mut canvas.gizmos, map(p), 10.0, WHITE.with_alpha(0.6));
    }

    Some(())
}

pub fn draw_ui_layout(canvas: &mut Canvas, state: &GameState) -> Option<()> {
    let wb = state.input.screen_bounds.span;

    for layout in state.ui.layouts() {
        for n in layout.iter() {
            if n.text_content().map(|t| t.is_empty()).unwrap_or(true) {
                continue;
            }
            let aabb = n.aabb_camera(wb);
            let p = state.input.position(MouseButt::Hover, FrameId::Current);
            if p.map(|p| aabb.contains(p)).unwrap_or(false) {
                draw_aabb(canvas, n.aabb_camera(wb), RED);
            }
        }
    }

    Some(())
}

pub fn draw_orbit_spline(canvas: &mut Canvas, state: &GameState) -> Option<()> {
    if !state.input.is_pressed(KeyCode::KeyP) {
        return None;
    }

    let g = state
        .cursor_orbit_if_mode()
        .map(|o: GlobalOrbit| get_orbit_info_graph(&o.1))
        .unwrap_or(Graph::blank());

    let bounds = state.input.screen_bounds.with_center(Vec2::ZERO);

    draw_graph(canvas, &g, bounds, Some(&state.input));
    draw_graph(canvas, get_lut_graph(), bounds, Some(&state.input));

    Some(())
}

fn highlight_targeted_vehicle(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
    let id = state.targeting()?;
    let lup = state.universe.lup_orbiter(id, state.universe.stamp())?;
    let pos = lup.pv().pos;
    let c = state.orbital_context.w2c(pos);
    draw_circle(gizmos, c, 10.0, TEAL);
    for km in 1..=5 {
        let r = state.orbital_context.scale() * km as f64;
        draw_circle(gizmos, c, gcast(r), GRAY);
    }
    Some(())
}

fn draw_rendezvous_info(canvas: &mut Canvas, state: &GameState) -> Option<()> {
    let ctx = &state.orbital_context;
    let pilot = state.piloting()?;
    let target = state.targeting()?;
    let po = state
        .universe
        .orbital_vehicles
        .get(&pilot)?
        .current_orbit()?;
    let to = state
        .universe
        .orbital_vehicles
        .get(&target)?
        .current_orbit()?;
    let vb = state.input.screen_bounds.span / 2.0;

    let (g, v, relpos) = revisit(make_separation_graph(&po.1, &to.1, state.universe.stamp()));

    let h = 140.0;

    draw_graph(
        canvas,
        &g,
        AABB::from_arbitrary(
            vb - Vec2::new(vb.x * 0.7, 200.0),
            vb - Vec2::new(20.0, 200.0 - h),
        ),
        Some(&state.input),
    );
    draw_graph(
        canvas,
        &v,
        AABB::from_arbitrary(
            vb - Vec2::new(vb.x * 0.7, 220.0 + h),
            vb - Vec2::new(20.0, 220.0),
        ),
        Some(&state.input),
    );

    {
        let world_radius = ctx.rendezvous_scope_radius.value; // km
        let screen_radius = 150.0;
        let screen_center = vb - Vec2::new(200.0, 550.0);
        let current_world = relpos.first().cloned();

        let relpos_screen: Vec<_> = relpos
            .into_iter()
            .map(|p| {
                let sep = p.length();
                if sep > world_radius {
                    Vec2::NAN
                } else {
                    screen_center + graphics_cast(p / world_radius) * screen_radius
                }
            })
            .collect();

        draw_circle(&mut canvas.gizmos, screen_center, screen_radius, GRAY);
        canvas.gizmos.linestrip_2d(relpos_screen, WHITE);
        draw_x(&mut canvas.gizmos, screen_center, 6.0, RED);
        if let Some(p) = current_world {
            let p = screen_center + graphics_cast(p / world_radius) * screen_radius;
            draw_x(&mut canvas.gizmos, p, 7.0, TEAL);
        }
    }

    if let Ok(Some((t, pv))) = get_next_intersection(state.universe.stamp(), &po.1, &to.1) {
        let p = ctx.w2c(pv.pos);
        draw_circle(&mut canvas.gizmos, p, 20.0, WHITE);
        if let Some(q) = to.1.pv(t).ok() {
            let q = ctx.w2c(q.pos);
            draw_circle(&mut canvas.gizmos, q, 20.0, ORANGE);
        }
    }

    Some(())
}

fn draw_landing_sites(gizmos: &mut Gizmos, state: &GameState) {
    let ctx = &state.orbital_context;
    for (_, site) in &state.universe.landing_sites {
        if let Some(pos) = landing_site_position(&state.universe, site.planet, site.angle) {
            let p = ctx.w2c(pos);
            draw_diamond(gizmos, p, 12.0, WHITE.with_alpha(0.7))
        }
    }
}

pub fn draw_bezier(gizmos: &mut Gizmos, bezier: &Bezier, color: Srgba) {
    let points: Vec<_> = linspace(0.0, 1.0, 20)
        .into_iter()
        .map(|t| bezier.eval(t))
        .collect();
    gizmos.linestrip_2d(points, color);
}

pub fn draw_factory(canvas: &mut Canvas, factory: &Factory, _aabb: AABB, _stamp: Nanotime) {
    // draw_aabb(&mut canvas.gizmos, aabb, WHITE.with_alpha(0.3));

    // let mut text_pos = aabb.top_center() + Vec2::Y * 20.0;

    // canvas.text(format!("{}", factory.stamp().to_date()), text_pos, 0.7);

    // for (_, plant) in factory.plants() {
    //     text_pos += Vec2::Y * 24.0;
    //     canvas.text(format!("{}", plant.recipe()), text_pos, 0.7);
    // }

    // for storage in factory.storage() {
    //     text_pos += Vec2::Y * 24.0;
    //     canvas.text(format!("{:?}", storage), text_pos, 0.7);
    // }

    if factory.storage_count() + factory.plant_count() == 0 {
        return;
    }

    // canvas.text(
    //     format!("{:?}", factory.get_next_relevant_plant()),
    //     Vec2::ZERO,
    //     1.3,
    // );

    let n = factory.storage_count() + factory.plant_count();

    let storage_width = 50.0;
    let plant_width = 70.0;

    let id_to_pos = |id: u64| {
        let angle = id as f32 * 2.0 * PI / n as f32;
        rotate(Vec2::X * 300.0, angle)
    };

    for (id, storage) in factory.storage() {
        let center = id_to_pos(id);
        let aabb = AABB::new(center, Vec2::splat(storage_width));
        let color = crate::sprites::hashable_to_color(&storage.item());
        draw_aabb(canvas, aabb, color.into());

        canvas.text(
            format!(
                "{:?} {} / {}",
                storage.item(),
                Mass::grams(storage.count()),
                Mass::grams(storage.capacity())
            ),
            center + Vec2::Y * storage_width,
            0.6,
        );

        let filled = storage.fill_percent();
        let aabb_fill = AABB::from_arbitrary(
            aabb.lower(),
            aabb.bottom_right() + Vec2::Y * aabb.span.y * filled,
        );
        canvas
            .sprite(
                aabb_fill.center,
                0.0,
                "error",
                ZOrdering::Factory,
                aabb_fill.span,
            )
            .set_color(color);

        canvas.sprite(
            aabb.center,
            0.0,
            format!("item-{}", storage.item().to_sprite_name()),
            ZOrdering::Factory,
            Vec2::splat(storage_width),
        );
    }

    // let input_port_center = |id: u64, i: usize| {
    //     let center = id_to_pos(id);
    //     let aabb = AABB::new(center, Vec2::splat(plant_width));
    //     let bl = aabb.lower();
    // };

    for (plant_id, plant) in factory.plants() {
        let center = id_to_pos(plant_id);
        let aabb = AABB::new(center, Vec2::splat(plant_width));
        draw_aabb(canvas, aabb, WHITE);

        canvas.text(plant.name().to_uppercase(), aabb.center, 0.6);

        {
            let progress = plant.progress();
            let bl = aabb.bottom_right();
            let tr = bl + Vec2::new(plant_width * 0.15, progress * plant_width);
            canvas.rect(AABB::from_arbitrary(bl, tr), ZOrdering::Factory, RED);
        }

        {
            let d = plant_width * 0.2;
            let lc = if plant.is_enabled() { YELLOW } else { GRAY };
            let bc = if plant.is_blocked() { ORANGE } else { GRAY };
            let sc = if plant.is_starved() { BLUE } else { GRAY };
            let wc = if plant.is_working() { GREEN } else { RED };

            let mut tr = aabb.top_left() - Vec2::new(d, 0.0);
            for color in [lc, bc, sc, wc] {
                let bl = tr - Vec2::splat(d);
                canvas.rect(AABB::from_arbitrary(bl, tr), ZOrdering::Factory, color);
                tr -= Vec2::Y * d * 1.4;
            }
        }

        let recipe = plant.recipe();

        // draw inputs
        let input_count = recipe.input_count();
        if input_count > 0 {
            for (i, (item, _)) in recipe.inputs().enumerate() {
                let color = crate::sprites::hashable_to_color(&item);
                let width = plant_width / input_count as f32;
                let height = plant_width / 4.0;
                let bl = aabb.lower() + Vec2::X * i as f32 * width;
                let tr = bl + Vec2::new(width, height);
                let aabb = AABB::from_arbitrary(bl, tr);
                canvas.rect(aabb, ZOrdering::Factory, color);
            }
        }

        // draw outputs
        let output_count = recipe.output_count();
        if output_count > 0 {
            for (i, (item, _)) in recipe.outputs().enumerate() {
                let color = crate::sprites::hashable_to_color(&item);
                let width = plant_width / output_count as f32;
                let height = plant_width / 4.0;
                let bl = aabb.lower() + Vec2::new(i as f32 * width, plant_width * 0.75);
                let tr = bl + Vec2::new(width, height);
                let aabb = AABB::from_arbitrary(bl, tr);
                canvas.rect(aabb, ZOrdering::Factory, color);
            }
        }

        for port in plant.input_ports() {
            let conn_id = match port.connected_to() {
                Some(id) => id,
                None => continue,
            };
            let color = crate::sprites::hashable_to_color(&port.item());
            let start = center - Vec2::Y * plant_width / 2.5;
            let end = id_to_pos(conn_id);
            let bezier = Bezier::new(vec![start, start - Vec2::Y * 200.0, Vec2::ZERO, end]);
            draw_bezier(&mut canvas.gizmos, &bezier, color.into());
        }

        for port in plant.output_ports() {
            let conn_id = match port.connected_to() {
                Some(id) => id,
                None => continue,
            };
            let color = crate::sprites::hashable_to_color(&port.item());
            let start = center + Vec2::Y * plant_width / 2.5;
            let end = id_to_pos(conn_id);
            let bezier = Bezier::new(vec![start, start + Vec2::Y * 200.0, Vec2::ZERO, end]);
            draw_bezier(&mut canvas.gizmos, &bezier, color.into());
        }
    }

    // bar graph representation

    // let column_width = aabb.span.x / n as f32;
    // let sprite_size = 50.0;

    // let mut bl = aabb.lower();

    // for (_, storage) in factory.storage() {
    //     let item = storage.item();
    //     let color = crate::sprites::hashable_to_color(&item);
    //     let dims = Vec2::new(
    //         column_width,
    //         aabb.span.y * storage.count() as f32 / storage.capacity() as f32,
    //     );
    //     let aabb = AABB::from_arbitrary(bl, bl + dims);
    //     canvas
    //         .sprite(aabb.center, 0.0, "error", 0.0, aabb.span)
    //         .set_color(color);

    //     let mut bottom = aabb.bottom_center() - Vec2::Y * 15.0;
    //     canvas.text(format!("{:?}", item), bottom, 0.7);
    //     bottom -= Vec2::Y * 20.0;
    //     canvas.text(format!("{}", Mass::grams(storage.count())), bottom, 0.7);
    //     bottom -= Vec2::Y * 20.0;
    //     canvas.text(format!("{}", Mass::grams(storage.capacity())), bottom, 0.7);

    //     let sprite_name = item.to_string().to_lowercase();
    //     bottom -= Vec2::Y * sprite_size;
    //     canvas.sprite(bottom, 0.0, sprite_name, 0.0, Vec2::splat(sprite_size));

    //     bl += Vec2::X * column_width;
    // }

    // for (_, plant) in factory.plants() {
    //     let color = crate::sprites::hashable_to_color(plant.recipe());
    //     let dims = Vec2::new(column_width, aabb.span.y * plant.progress());
    //     let aabb = AABB::from_arbitrary(bl, bl + dims);
    //     canvas
    //         .sprite(aabb.center, 0.0, "error", 0.0, aabb.span)
    //         .set_color(color);
    //     bl += Vec2::X * column_width;
    // }
}

pub fn draw_orbital_view(canvas: &mut Canvas, state: &GameState) {
    let ctx = &state.orbital_context;

    draw_camera_info(canvas, ctx, state.input.screen_bounds.span);

    draw_transforms(canvas, ctx, &state.universe);

    draw_scale_indicator(canvas, state);

    draw_piloting_overlay(canvas, state, state.piloting());

    draw_pinned(canvas, state);

    highlight_targeted_vehicle(&mut canvas.gizmos, state);

    draw_rendezvous_info(canvas, state);

    draw_orbit_spline(canvas, state);

    draw_landing_sites(&mut canvas.gizmos, state);

    if let Some(bounds) = state.orbital_context.selection_bounds {
        draw_aabb(canvas, ctx.w2c_aabb(bounds), RED);
    }

    if let Some((m1, m2, corner)) = state.measuring_tape() {
        let m1 = ctx.w2c(m1);
        let m2 = ctx.w2c(m2);
        let corner = ctx.w2c(corner);
        draw_x(&mut canvas.gizmos, m1, 12.0, GRAY);
        draw_x(&mut canvas.gizmos, m2, 12.0, GRAY);
        canvas.gizmos.line_2d(m1, m2, GRAY);
        canvas.gizmos.line_2d(m1, corner, GRAY.with_alpha(0.3));
        canvas.gizmos.line_2d(m2, corner, GRAY.with_alpha(0.3));
    }

    if let Some((c, a, b)) = state.protractor() {
        let b = b.unwrap_or(a);
        let c = ctx.w2c(c);
        let a = ctx.w2c(a);
        let b = ctx.w2c(b);
        let r1 = c.distance(a);
        let r2 = c.distance(b);
        for p in [a, b, c] {
            draw_x(&mut canvas.gizmos, p, 7.0, WHITE);
        }
        draw_circle(&mut canvas.gizmos, c, r1, WHITE.with_alpha(0.4));
        draw_circle(&mut canvas.gizmos, c, r2, WHITE.with_alpha(0.7));
        canvas.gizmos.line_2d(c, a, RED);
        canvas.gizmos.line_2d(c, b, GREEN);
        canvas.gizmos.line_2d(a, b, GRAY.with_alpha(0.3));
        let angle = (a - c).angle_to(b - c);
        let iso = Isometry2d::new(c, ((a - c).to_angle() - PI / 2.0).into());
        canvas
            .gizmos
            .arc_2d(iso, angle, (r1 * 0.75).min(r2), TEAL)
            .resolution(100);
    }

    for orbit in &state.orbital_context.queued_orbits {
        draw_global_orbit(canvas, orbit, &state, RED);
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
                draw_global_orbit(canvas, &go, &state, YELLOW.with_alpha(alpha));
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
        draw_global_orbit(canvas, &orbit, &state, ORANGE);
    }

    if let Some(orbit) = state.current_orbit() {
        draw_global_orbit(canvas, &orbit, &state, TEAL);
    }

    if state.orbital_context.show_animations && state.orbital_context.selected.len() < 6 {
        for id in &state.orbital_context.selected {
            draw_event_animation(&mut canvas.gizmos, &state, *id, ctx);
        }
    }

    draw_scenario(canvas, state);

    draw_x(
        &mut canvas.gizmos,
        state.light_source(),
        20.0,
        RED.with_alpha(0.2),
    );

    draw_highlighted_objects(&mut canvas.gizmos, &state);

    draw_notifications(&mut canvas.gizmos, &state);
}

fn orthographic_camera_map(p: Vec3, center: Vec3, normal: Vec3, x: Vec3, y: Vec3) -> Vec2 {
    let p = p - center;
    let p = p.reject_from(normal);
    Vec2::new(p.dot(x), p.dot(y))
}

pub fn draw_game_state(gizmos: Gizmos, mut state: ResMut<GameState>, painter: ShapePainter) {
    // draw_input_state(&mut gizmos, &state);

    let mut canvas = Canvas::new(gizmos, painter);

    GameState::draw(&mut canvas, &state);

    state.text_labels = canvas.text_labels;
    state.sprites = canvas.sprites;
}

pub fn draw_cells(canvas: &mut Canvas, state: &GameState) -> Option<()> {
    let ctx = &state.orbital_context;

    let scale_factor = 3500.0;

    let mut idxs = HashSet::new();

    for id in state.universe.orbiter_ids() {
        let pos = state
            .universe
            .lup_orbiter(id, state.universe.stamp())?
            .pv()
            .pos;

        let idx = vfloor_f64(pos / scale_factor);
        idxs.insert(idx);
    }

    for idx in idxs {
        let p = idx.as_dvec2() * scale_factor;
        let q = p + DVec2::splat(scale_factor);

        let aabb = AABB::from_arbitrary(aabb_stopgap_cast(p), aabb_stopgap_cast(q));
        let aabb = ctx.w2c_aabb(aabb);
        draw_aabb(canvas, aabb, ORANGE.with_alpha(0.3));
        fill_aabb(canvas, aabb, GRAY.with_alpha(0.03));
    }

    Some(())
}

pub fn draw_transforms(canvas: &mut Canvas, ctx: &impl CameraProjection, universe: &Universe) {
    for (_, ov) in &universe.orbital_vehicles {
        let pv = ov.pv();
        let parent = ov.parent();
        let parent_pv = match universe.lup_planet(parent, universe.stamp()) {
            Some(p) => p.pv(),
            None => continue,
        };

        let p1 = ctx.w2c(pv.pos + parent_pv.pos);
        let p2 = ctx.w2c(parent_pv.pos);

        canvas.painter.reset();
        canvas.painter.thickness = 2.0;
        canvas.painter.set_color(GREEN);
        let z = ZOrdering::Transforms.as_f32();
        canvas.painter.line(p1.extend(z), p2.extend(z));
    }
}

pub fn draw_camera_info(canvas: &mut Canvas, ctx: &impl CameraProjection, window_span: Vec2) {
    let meters = window_span.as_dvec2() / ctx.scale();
    let lower_bound = ctx.origin() - meters / 2.0;
    let upper_bound = ctx.origin() + meters / 2.0;

    let xl = lower_bound.x.ceil() as i64;
    let xu = upper_bound.x.floor() as i64;

    let yl = lower_bound.y.ceil() as i64;
    let yu = upper_bound.y.floor() as i64;

    let dist = (xu - xl).max(yu - yl);

    let mut step: i64 = 1;
    while dist / step > 100 {
        step *= 10;
    }

    let xl = (xl / step) * step;
    let xu = (xu / step) * step;

    let yl = (yl / step) * step;
    let yu = (yu / step) * step;

    let step = step.try_into().unwrap();

    canvas.painter.reset();
    canvas.painter.thickness = 3.0;

    for x in (xl..=xu).step_by(step) {
        let wp = lower_bound.with_x(x as f64);
        let p = ctx.w2c(wp);
        let q = p + Vec2::Y * 10.0;
        canvas.painter.line(
            p.extend(ZOrdering::ScaleIndicator.as_f32()),
            q.extend(ZOrdering::ScaleIndicator.as_f32()),
        );
    }

    for y in (yl..=yu).step_by(step) {
        let wp = upper_bound.with_y(y as f64);
        let p = ctx.w2c(wp);
        let q = p - Vec2::X * 10.0;
        canvas.painter.line(
            p.extend(ZOrdering::ScaleIndicator.as_f32()),
            q.extend(ZOrdering::ScaleIndicator.as_f32()),
        );
    }
}

fn draw_input_state(canvas: &mut Canvas, state: &GameState) {
    let points = [
        (MouseButt::Left, BLUE),
        (MouseButt::Right, GREEN),
        (MouseButt::Middle, YELLOW),
    ];

    let offset = state.input.screen_bounds.span / 2.0;
    draw_aabb(canvas, state.input.screen_bounds.offset(-offset), RED);

    if let Some(p) = state.input.position(MouseButt::Hover, FrameId::Current) {
        draw_circle(&mut canvas.gizmos, p, 8.0, RED);
    }

    for (b, c) in points {
        let p1 = state.input.position(b, FrameId::Down);
        let p2 = state
            .input
            .position(b, FrameId::Current)
            .or(state.input.position(b, FrameId::Up));

        if let Some((p1, p2)) = p1.zip(p2) {
            canvas.gizmos.line_2d(p1, p2, c);
        }

        for fid in [FrameId::Down, FrameId::Up] {
            let age = state.input.age(b, fid, state.wall_time);
            let p = state.input.position(b, fid);
            if let Some((p, age)) = p.zip(age) {
                let dt = age.to_secs();
                let a = (1.0 - dt).max(0.0);
                draw_circle(&mut canvas.gizmos, p, 50.0 * age.to_secs(), c.with_alpha(a));
            }
        }
    }
}
