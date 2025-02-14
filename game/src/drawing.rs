use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use starling::aabb::{AABB, OBB};
use starling::control::Controller;
use starling::core::*;
use starling::orbiter::*;
use starling::orbits::sparse_orbit::*;
use starling::planning::*;
use starling::pv::PV;

use crate::camera_controls::CameraState;
use crate::planetary::GameState;

pub fn alpha(color: Srgba, a: f32) -> Srgba {
    Srgba { alpha: a, ..color }
}

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

pub fn draw_circle(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    gizmos
        .circle_2d(Isometry2d::from_translation(p), size, color)
        .resolution(200);
}

pub fn draw_velocity_vec(gizmos: &mut Gizmos, pv: PV, length: f32, color: Srgba) {
    let p1 = pv.pos;
    let p2 = pv.pos + pv.vel.normalize_or_zero() * length;
    gizmos.line_2d(p1, p2, color);
}

pub fn draw_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    gizmos.rect_2d(Isometry2d::from_translation(aabb.center), aabb.span, color);
}

pub fn draw_obb(gizmos: &mut Gizmos, obb: &OBB, color: Srgba) {
    draw_cross(gizmos, obb.0.center, 30.0, color);
    let mut corners = obb.corners().to_vec();
    corners.push(*corners.get(0).unwrap());
    gizmos.linestrip_2d(corners, color);
}

pub fn draw_orbit(gizmos: &mut Gizmos, orb: &SparseOrbit, origin: Vec2, color: Srgba) {
    if orb.eccentricity >= 1.0 {
        let n_points = 60;
        let range = 0.999 * hyperbolic_range_ta(orb.eccentricity);
        let points: Vec<_> = (0..n_points)
            .map(|i| {
                let t = (i as f32 / (n_points - 1) as f32) * 2.0 - 1.0;
                origin + orb.position_at(t * range)
            })
            .collect();
        gizmos.linestrip_2d(points, color);
    } else {
        let b = orb.semi_major_axis * (1.0 - orb.eccentricity.powi(2)).sqrt();
        let center: Vec2 = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
        let iso = Isometry2d::new(center, orb.arg_periapsis.into());

        let res = orb.semi_major_axis.clamp(40.0, 300.0) as u32;

        gizmos
            .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), color)
            .resolution(res);
    }

    // if !detailed {
    //     return;
    // }

    // if orb.eccentricity >= 1.0 {
    //     let focii = orb.focii();
    //     draw_cross(gizmos, focii[0], 20.0, WHITE);
    //     draw_circle(gizmos, focii[1], 15.0, WHITE);
    //     if let Some((ua, la)) = orb.asymptotes() {
    //         let c = orb.center();
    //         for asym in [ua, la] {
    //             gizmos.line_2d(c, c + asym * 100.0, alpha(WHITE, 0.04));
    //         }
    //     }
    // }

    // let ta = orb.ta_at_time(stamp).as_f32();
    // let root = orb.position_at(ta) + origin;
    // let t1 = root + orb.normal_at(ta) * 60.0;
    // let t2 = root + orb.tangent_at(ta) * 60.0;
    // let t3 = root + orb.velocity_at(ta) * 3.0;
    // gizmos.line_2d(root, t1, GREEN);
    // gizmos.line_2d(root, origin, alpha(GREEN, 0.4));
    // gizmos.line_2d(root, t2, GREEN);
    // gizmos.line_2d(root, t3, PURPLE);
}

pub fn draw_globe(gizmos: &mut Gizmos, p: Vec2, radius: f32, color: Srgba) {
    draw_circle(gizmos, p, radius, color);
    let c = alpha(color, 0.15);
    let iso = Isometry2d::from_translation(p);
    for s in [0.9, 0.6, 0.3] {
        gizmos.ellipse_2d(iso, Vec2::new(radius, radius * s), c);
        gizmos.ellipse_2d(iso, Vec2::new(radius * s, radius), c);
    }
    for (p1, p2) in [
        ((-radius, 0.0), (radius, 0.0)),
        ((0.0, -radius), (0.0, radius)),
    ] {
        gizmos.line_2d(p + Vec2::from(p1), p + Vec2::from(p2), c);
    }
}

pub fn draw_planets(gizmos: &mut Gizmos, planet: &PlanetarySystem, stamp: Nanotime, origin: Vec2) {
    draw_shadows(gizmos, origin, planet.body.radius, stamp);
    draw_globe(gizmos, origin, planet.body.radius, WHITE);
    for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
        draw_circle(gizmos, origin, planet.body.soi * ds, alpha(ORANGE, a));
    }

    for (orbit, pl) in &planet.subsystems {
        let pv = orbit.pv_at_time(stamp);
        draw_orbit(gizmos, orbit, origin, alpha(GRAY, 0.4));
        draw_planets(gizmos, pl, stamp, origin + pv.pos)
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
        let pv_end = parent_pv + prop.pv(prop.end)?;
        if let Some(e) = prop.event {
            draw_event(gizmos, planets, &e, prop.end, pv_end.pos, scale, duty_cycle);
        }
    }
    Some(())
}

pub fn draw_object(
    gizmos: &mut Gizmos,
    planets: &PlanetarySystem,
    obj: &Orbiter,
    stamp: Nanotime,
    scale: f32,
    show_orbits: bool,
    tracked: bool,
    duty_cycle: bool,
) -> Option<()> {
    let pv = obj.pv(stamp, planets)?;

    let size = (4.0 * scale).min(10.0);
    draw_circle(gizmos, pv.pos, size, WHITE);
    if tracked {
        draw_square(gizmos, pv.pos, (70.0 * scale).min(70.0), alpha(WHITE, 0.7));
    }
    if duty_cycle && obj.will_collide() {
        draw_circle(gizmos, pv.pos, size + 10.0 * scale, RED);
        draw_circle(gizmos, pv.pos, size + 16.0 * scale, RED);
    }
    if duty_cycle && obj.has_error() {
        draw_circle(gizmos, pv.pos, size + 10.0 * scale, YELLOW);
        draw_circle(gizmos, pv.pos, size + 16.0 * scale, YELLOW);
    }

    if tracked {
        if let Some((pvl, pv)) = obj.pvl(stamp).zip(obj.pv(stamp, planets)) {
            let prograde = pvl.vel.normalize_or(Vec2::ZERO) * 200.0 * scale.min(2.0);
            gizmos.line_2d(pv.pos, pv.pos + prograde, RED);
        }

        for (i, prop) in obj.props().iter().enumerate() {
            let color = if i == 0 {
                alpha(ORANGE, 0.3)
            } else {
                alpha(TEAL, (1.0 - i as f32 * 0.3).max(0.0))
            };
            draw_propagator(
                gizmos, planets, &prop, stamp, scale, true, color, duty_cycle,
            );
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

pub fn draw_orbital_system(
    gizmos: &mut Gizmos,
    sys: &OrbitalTree,
    stamp: Nanotime,
    scale: f32,
    show_orbits: bool,
    track_list: &Vec<ObjectId>,
    duty_cycle: bool,
) {
    draw_planets(gizmos, &sys.system, stamp, Vec2::ZERO);

    _ = sys
        .objects
        .iter()
        .map(|obj| {
            let is_tracked = track_list.contains(&obj.id);
            draw_object(
                gizmos,
                &sys.system,
                obj,
                stamp,
                scale,
                show_orbits,
                is_tracked,
                duty_cycle,
            );
        })
        .collect::<Vec<_>>();
}

pub fn draw_scalar_field_cell(
    gizmos: &mut Gizmos,
    planet: &PlanetarySystem,
    stamp: Nanotime,
    center: Vec2,
    step: f32,
    levels: &[i32],
    expanded: bool,
) {
    if !expanded {
        let d = planet
            .subsystems
            .iter()
            .map(|(orbit, _)| (orbit.pv_at_time(stamp).pos.distance(center) * 1000.0) as u32)
            .min()
            .unwrap_or(10000000);
        if d < 600000 || center.length() < 600.0 {
            let n: i32 = 4;
            let substep = step / n as f32;
            for i in 0..n {
                for j in 0..n {
                    let x = (i - 1) as f32 / n as f32 * substep * n as f32 - substep * 0.5;
                    let y = (j - 1) as f32 / n as f32 * substep * n as f32 - substep * 0.5;
                    let p = center + Vec2::new(x, y);
                    draw_scalar_field_cell(gizmos, planet, stamp, p, substep, levels, true);
                }
            }
            return;
        }
    }

    draw_square(gizmos, center, step as f32, alpha(WHITE, 0.01));

    let bl = center + Vec2::new(-step / 2.0, -step / 2.0);
    let br = center + Vec2::new(step / 2.0, -step / 2.0);
    let tl = center + Vec2::new(-step / 2.0, step / 2.0);
    let tr = center + Vec2::new(step / 2.0, step / 2.0);

    let pot: Vec<(Vec2, f32)> = [bl, br, tr, tl]
        .iter()
        .map(|p| (*p, potential_at(&planet, *p, stamp)))
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

        gizmos.linestrip_2d(pts, GREEN);
    }
}

pub fn draw_scalar_field_v2(
    gizmos: &mut Gizmos,
    planet: &PlanetarySystem,
    stamp: Nanotime,
    levels: &[i32],
) {
    let step = 250;
    for y in (-4000..=4000).step_by(step) {
        for x in (-4000..=4000).step_by(step) {
            let p = Vec2::new(x as f32, y as f32);
            draw_scalar_field_cell(gizmos, planet, stamp, p, step as f32, levels, false);
        }
    }
}

pub fn draw_shadows(gizmos: &mut Gizmos, origin: Vec2, radius: f32, stamp: Nanotime) {
    let angle = stamp.to_secs() / 1000.0;
    let u = rotate(Vec2::X, angle);
    let steps = radius.ceil() as u32;
    let jmax = 50;
    for i in 0..steps {
        let y = (i as f32 / (steps - 1) as f32) * 2.0 - 1.0;
        let xoff = Vec2::X * radius * (1.0 - y.powi(2)).sqrt();
        let yoff = Vec2::Y * y * radius;
        let start = origin + rotate(xoff + yoff, angle);
        let delta = u * 2000.0;
        for j in 0..jmax {
            let s = start + delta * j as f32;
            let e = start + delta * (j + 1) as f32;
            let a = 0.25 * ((jmax - j) as f32 / jmax as f32).powi(4);
            gizmos.line_2d(s, e, alpha(BLACK, a));
        }
    }
}

pub fn draw_event_marker_at(
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
        EventType::Maneuver(_) => PURPLE,
    };

    draw_circle(gizmos, p, 15.0 * scale, alpha(color, 0.8));
    draw_circle(gizmos, p, 6.0 * scale, alpha(color, 0.8));
}

pub fn draw_event(
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

pub fn draw_highlighted_objects(gizmos: &mut Gizmos, state: &GameState) {
    _ = state
        .highlighted_list
        .iter()
        .filter_map(|id| {
            let pv = state.system.orbiter_lookup(*id, state.sim_time)?.pv();
            draw_circle(gizmos, pv.pos, 20.0 * state.camera.actual_scale, GRAY);
            Some(())
        })
        .collect::<Vec<_>>();
}

pub fn draw_camera_controls(gizmos: &mut Gizmos, cam: &CameraState) {
    if let Some(p) = cam.mouse_pos() {
        draw_circle(gizmos, p, 3.0 * cam.actual_scale, RED);
    }

    if let Some(p) = cam.mouse_down_pos() {
        draw_circle(gizmos, p, 2.0 * cam.actual_scale, RED);
    }

    if let Some(a) = cam.selection_region() {
        draw_aabb(gizmos, a, RED);
    }
}

pub fn draw_controllers(
    gizmos: &mut Gizmos,
    system: &OrbitalTree,
    ctrl: &Controller,
    stamp: Nanotime,
) -> Option<()> {
    if !ctrl.last().is_some() {
        return None;
    }
    let obj = system.objects.iter().find(|o| o.id == ctrl.target())?;
    let pv = obj.pv(stamp, &system.system)?;
    draw_circle(gizmos, pv.pos, 60.0, TEAL);
    Some(())
}

pub fn draw_event_animation(
    gizmos: &mut Gizmos,
    system: &OrbitalTree,
    id: ObjectId,
    stamp: Nanotime,
    scale: f32,
    duty_cycle: bool,
) -> Option<()> {
    let obj = system.objects.iter().find(|o| o.id == id)?;
    let p = obj.props().last()?;
    let mut t = stamp;
    while t < p.end {
        let pv = obj.pv(t, &system.system)?;
        draw_square(gizmos, pv.pos, 11.0 * scale.min(1.0), alpha(WHITE, 0.6));
        t += Nanotime::secs(1);
    }
    for prop in obj.props() {
        let pv = obj.pv(prop.end, &system.system)?;
        if let Some(e) = prop.event {
            draw_event_marker_at(gizmos, &e, pv.pos, scale, duty_cycle);
        }
    }
    let pv = obj.pv(p.end, &system.system)?;
    draw_square(gizmos, pv.pos, 13.0 * scale.min(1.0), alpha(RED, 0.8));
    Some(())
}

pub fn draw_maneuver_plan(gizmos: &mut Gizmos, state: &GameState, id: ObjectId) -> Option<()> {
    let to = state.target_orbit()?;
    let pr = state
        .system
        .objects
        .iter()
        .find(|o| o.id == id)?
        .propagator_at(state.sim_time)?
        .orbit;

    let plan = generate_maneuver_plan(&pr, &to, state.sim_time)?;

    for node in &plan.nodes {
        for pv in [node.before, node.after] {
            draw_circle(gizmos, pv.pos, 10.0 * state.camera.actual_scale, YELLOW);
            draw_velocity_vec(gizmos, pv, 60.0 * state.camera.actual_scale, PURPLE);
        }
        draw_orbit(gizmos, &node.orbit, Vec2::ZERO, alpha(YELLOW, 0.2));
    }

    Some(())
}

pub fn draw_game_state(mut gizmos: Gizmos, state: &GameState) {
    let stamp = state.sim_time;

    for p in &state.control_points {
        draw_circle(
            &mut gizmos,
            *p,
            6.0 * state.camera.actual_scale,
            alpha(GRAY, 0.4),
        );
    }

    if let Some(aabb) = state.tracked_aabb() {
        draw_aabb(&mut gizmos, aabb.padded(50.0), alpha(GRAY, 0.03));
    }

    if let Some(o) = state.target_orbit() {
        draw_orbit(&mut gizmos, &o, Vec2::ZERO, alpha(RED, 0.2));
        if let Some(test) = state.camera.mouse_pos() {
            for (p, d) in [o.nearest_along_track(test), o.nearest(test)] {
                let color = alpha(if d >= 0.0 { RED } else { TEAL }, 0.5);
                let size = 7.0 * state.camera.actual_scale.min(1.0);
                draw_circle(&mut gizmos, p.pos, size, color);
                draw_velocity_vec(&mut gizmos, p, 40.0 * state.camera.actual_scale, color);
                gizmos.line_2d(Vec2::ZERO, p.pos, color);
            }
        }
    }

    if state.show_potential_field {
        draw_scalar_field_v2(&mut gizmos, &state.system.system, stamp, &state.draw_levels);
    }

    for ctrl in &state.controllers {
        draw_controllers(&mut gizmos, &state.system, ctrl, state.sim_time);
    }

    if state.show_animations {
        for id in &state.track_list {
            draw_event_animation(
                &mut gizmos,
                &state.system,
                *id,
                state.sim_time,
                state.camera.actual_scale,
                state.duty_cycle_high,
            );
        }
    }

    draw_camera_controls(&mut gizmos, &state.camera);

    draw_orbital_system(
        &mut gizmos,
        &state.system,
        stamp,
        state.camera.actual_scale,
        state.show_orbits,
        &state.track_list,
        state.duty_cycle_high,
    );

    draw_highlighted_objects(&mut gizmos, &state);

    for id in &state.track_list {
        draw_maneuver_plan(&mut gizmos, &state, *id);
    }
}
