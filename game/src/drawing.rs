use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::ORANGE;
use bevy::prelude::*;
use starling::aabb::AABB;
use starling::core::*;
use starling::orbit::*;
use starling::orbiter::*;
use starling::planning::*;

use crate::planetary::GameState;

pub fn alpha(color: Srgba, a: f32) -> Srgba {
    Srgba { alpha: a, ..color }
}

pub fn draw_x(gizmos: &mut Gizmos, p: Vec2, size: f32, color: Srgba) {
    let dx = Vec2::new(size, 0.0);
    let dy = Vec2::new(0.0, size);
    gizmos.line_2d(p - dx, p + dx, color);
    gizmos.line_2d(p - dy, p + dy, color);
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

pub fn draw_aabb(gizmos: &mut Gizmos, aabb: AABB, color: Srgba) {
    gizmos.rect_2d(
        Isometry2d::from_translation(aabb.center()),
        aabb.span(),
        color,
    );
}

pub fn draw_orbit(
    origin: Vec2,
    stamp: Nanotime,
    orb: &Orbit,
    gizmos: &mut Gizmos,
    a: f32,
    color: Srgba,
    detailed: bool,
) {
    if orb.eccentricity >= 1.0 {
        let n_points = 60;
        let range = 0.999 * hyperbolic_range_ta(orb.eccentricity);
        let points: Vec<_> = (0..n_points)
            .map(|i| {
                let t = (i as f32 / (n_points - 1) as f32) * 2.0 - 1.0;
                origin + orb.position_at(t * range)
            })
            .collect();
        gizmos.linestrip_2d(points, alpha(color, a));
    } else {
        let b = orb.semi_major_axis * (1.0 - orb.eccentricity.powi(2)).sqrt();
        let center: Vec2 = origin + (orb.periapsis() + orb.apoapsis()) / 2.0;
        let iso = Isometry2d::new(center, orb.arg_periapsis.into());

        let res = if detailed {
            1000
        } else {
            orb.semi_major_axis.clamp(40.0, 300.0) as u32
        };

        gizmos
            .ellipse_2d(iso, Vec2::new(orb.semi_major_axis, b), alpha(color, a))
            .resolution(res);
    }

    if !detailed {
        return;
    }

    if orb.eccentricity >= 1.0 {
        let focii = orb.focii();
        draw_x(gizmos, focii[0], 20.0, WHITE);
        draw_circle(gizmos, focii[1], 15.0, WHITE);
        if let Some((ua, la)) = orb.asymptotes() {
            let c = orb.center();
            for asym in [ua, la] {
                gizmos.line_2d(c, c + asym * 100.0, alpha(WHITE, 0.04));
            }
        }
    }

    let ta = orb.ta_at_time(stamp).as_f32();
    let root = orb.position_at(ta) + origin;
    let t1 = root + orb.normal_at(ta) * 60.0;
    let t2 = root + orb.tangent_at(ta) * 60.0;
    let t3 = root + orb.velocity_at(ta) * 3.0;
    gizmos.line_2d(root, t1, GREEN);
    gizmos.line_2d(root, origin, alpha(GREEN, 0.4));
    gizmos.line_2d(root, t2, GREEN);
    gizmos.line_2d(root, t3, PURPLE);

    gizmos.circle_2d(
        Isometry2d::from_translation(origin + orb.periapsis()),
        4.0,
        alpha(RED, a),
    );

    if orb.eccentricity < 1.0 {
        gizmos.circle_2d(
            Isometry2d::from_translation(origin + orb.apoapsis()),
            4.0,
            alpha(WHITE, a),
        );
    }
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

pub fn draw_planets(gizmos: &mut Gizmos, planet: &Planet, stamp: Nanotime, origin: Vec2) {
    draw_shadows(gizmos, origin, planet.primary.radius, stamp);
    draw_globe(gizmos, origin, planet.primary.radius, WHITE);
    for (a, ds) in [(1.0, 1.0), (0.3, 0.98), (0.1, 0.95)] {
        draw_circle(gizmos, origin, planet.primary.soi * ds, alpha(ORANGE, a));
    }

    for (orbit, pl) in &planet.subsystems {
        let pv = orbit.pv_at_time(stamp);
        draw_orbit(origin, stamp, orbit, gizmos, 0.4, GRAY, false);
        draw_planets(gizmos, pl, stamp, origin + pv.pos)
    }
}

const NUM_FUTURE_ORBITS: usize = 4;

const ORBIT_COLORS: [Srgba; NUM_FUTURE_ORBITS] = [TEAL, RED, GREEN, YELLOW];

pub fn draw_object(
    gizmos: &mut Gizmos,
    planets: &Planet,
    obj: &Object,
    stamp: Nanotime,
    scale: f32,
    show_orbits: bool,
    tracked: bool,
) -> Option<()> {
    let (_, parent_pv, _, _) = planets.lookup(obj.parent, stamp)?;
    let color = orbit_color_mapping(&obj.prop.orbit, stamp);
    let pv = parent_pv + obj.prop.orbit.pv_at_time(stamp);
    draw_circle(gizmos, pv.pos, (4.0 * scale).min(10.0), color);
    let (a, color) = if tracked { (0.2, ORANGE) } else { (0.05, GRAY) };
    if show_orbits {
        draw_orbit(parent_pv.pos, stamp, &obj.prop.orbit, gizmos, a, color, false);
    }
    if tracked {
        draw_square(gizmos, pv.pos, (70.0 * scale).min(70.0), alpha(color, 0.7));
    }

    if tracked {
        let mut cursor = obj.clone();
        for i in 0..NUM_FUTURE_ORBITS {
            let next = cursor.next(&planets);
            match next {
                Ok((e, nobj)) => {
                    let color = ORBIT_COLORS[i];
                    let (_, pv, _, _) = planets.lookup(nobj.parent, stamp)?;
                    let p = nobj.prop.orbit.pv_at_time(e.stamp).pos;
                    draw_event(gizmos, &e, p, scale);
                    draw_orbit(pv.pos, stamp, &nobj.prop.orbit, gizmos, 0.6, color, false);
                    cursor = nobj;
                }
                _ => break,
            };
        }
    }

    Some(())
}

fn orbit_color_mapping(orbit: &Orbit, stamp: Nanotime) -> Srgba {
    let near_parabolic = (orbit.eccentricity - 1.0).abs() < 0.01;
    if !orbit.is_consistent(stamp) {
        if near_parabolic {
            PURPLE
        } else {
            RED
        }
    } else {
        if near_parabolic {
            YELLOW
        } else {
            WHITE
        }
    }
}

pub fn draw_orbital_system(
    gizmos: &mut Gizmos,
    sys: &OrbitalTree,
    stamp: Nanotime,
    scale: f32,
    show_orbits: bool,
    track_list: &Vec<ObjectId>,
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
            );
        })
        .collect::<Vec<_>>();
}

pub fn draw_scalar_field_cell(
    gizmos: &mut Gizmos,
    planet: &Planet,
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

pub fn draw_scalar_field_v2(gizmos: &mut Gizmos, planet: &Planet, stamp: Nanotime, levels: &[i32]) {
    let step = 250;
    for y in (-4000..=4000).step_by(step) {
        for x in (-4000..=4000).step_by(step) {
            let p = Vec2::new(x as f32, y as f32);
            draw_scalar_field_cell(gizmos, planet, stamp, p, step as f32, levels, false);
        }
    }
}

pub fn draw_shadows(gizmos: &mut Gizmos, origin: Vec2, radius: f32, stamp: Nanotime) {
    let angle = as_seconds(stamp) / 1000.0;
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

pub fn draw_event(gizmos: &mut Gizmos, event: &OrbitalEvent, p: Vec2, scale: f32) {
    let color = match event.etype {
        EventType::Collide => RED,
        EventType::Encounter(_) => GREEN,
        EventType::Escape => TEAL,
        EventType::Maneuver(_) => PURPLE,
    };
    draw_circle(gizmos, p, 10.0 * scale, alpha(color, 0.5));
    draw_circle(gizmos, p, 3.0 * scale, alpha(color, 0.5));
}

pub fn draw_highlighted_objects(gizmos: &mut Gizmos, state: &GameState) {
    _ = state
        .highlighted_list
        .iter()
        .filter_map(|id| {
            let pv = state.system.lookup(*id, state.sim_time)?.pv();
            draw_circle(gizmos, pv.pos, 20.0 * state.actual_scale, GRAY);
            Some(())
        })
        .collect::<Vec<_>>();
}

pub fn draw_game_state(mut gizmos: Gizmos, state: Res<GameState>) {
    let stamp = state.sim_time;

    for p in &state.control_points {
        draw_circle(&mut gizmos, *p, 6.0 * state.actual_scale, alpha(GRAY, 0.4));
    }

    if let Some(o) = state.target_orbit() {
        draw_orbit(Vec2::ZERO, stamp, &o, &mut gizmos, 0.1, RED, false);
    }

    if state.show_potential_field {
        draw_scalar_field_v2(&mut gizmos, &state.system.system, stamp, &state.draw_levels);
    }

    if let Some(p) = state.mouse_pos() {
        draw_circle(&mut gizmos, p, 3.0 * state.actual_scale, RED);
    }

    if let Some(p) = state.mouse_down_pos() {
        draw_circle(&mut gizmos, p, 2.0 * state.actual_scale, RED);
    }

    if let Some(a) = state.selection_region() {
        draw_aabb(&mut gizmos, a, RED);
    }

    draw_orbital_system(
        &mut gizmos,
        &state.system,
        stamp,
        state.actual_scale,
        state.show_orbits,
        &state.track_list,
    );

    draw_highlighted_objects(&mut gizmos, &state);
}
