use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;

use crate::drawing::*;
use starling::aabb::*;
use starling::orbit::PI;
use starling::pv::PV;

use starling::core::{rand, rotate};

pub struct CraftPlugin;

impl Plugin for CraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(Update, (update_keys, update, draw).chain());
    }
}

#[derive(Default, Debug)]
struct RigidBody {
    pv: PV,
    angle: f32,
    angular_rate: f32,
    body: Vec<AABB>,
}

fn rigid_body_mesh(scale: f32) -> Vec<AABB> {
    vec![
        // body
        AABB::from_arbitrary((-100.0, -100.0), (0.0, 0.0)),
        AABB::from_arbitrary((90.0, -90.0), (0.0, 0.0)),
        AABB::from_arbitrary((0.0, 0.0), (100.0, 100.0)),
        AABB::from_arbitrary((-90.0, 90.0), (0.0, 0.0)),
        // panels
        AABB::from_arbitrary((-300.0, 20.0), (-80.0, -20.0)),
        AABB::from_arbitrary((300.0, 20.0), (80.0, -20.0)),
        // thruster
        AABB::from_arbitrary((-50.0, -90.0), (50.0, -150.0)),
    ]
    .iter()
    .map(|a| a.scale(scale))
    .map(|a| {
        let p1 = rotate(a.center + a.span / 2.0, PI / 2.0);
        let p2 = rotate(a.center - a.span / 2.0, PI / 2.0);
        AABB::from_arbitrary(p1, p2)
    })
    .collect()
}

impl RigidBody {
    fn new(pv: impl Into<PV>) -> Self {
        RigidBody {
            pv: pv.into(),
            angle: rand(0.0, PI * 2.0),
            angular_rate: 0.0,
            body: rigid_body_mesh(6.0),
        }
    }

    fn update(&mut self, dt: f32) {
        self.pv.pos += self.pv.vel.clone() * dt;
        self.angle += self.angular_rate * dt;
        self.pv.vel *= (-dt / 8.0).exp();
        self.angular_rate *= (-dt / 4.0).exp();
    }

    fn mass(&self) -> f32 {
        self.body.len() as f32
    }

    fn vel(&self, pos: Vec2) -> Vec2 {
        let ang = Vec3::new(0.0, 0.0, self.angular_rate);
        ang.cross(pos.extend(0.0)).xy() + self.pv.vel
    }

    fn moi(&self) -> f32 {
        200000.0
    }

    fn body(&self) -> Vec<OBB> {
        self.body
            .iter()
            .map(|e| e.rotate_about(Vec2::ZERO, self.angle).offset(self.pv.pos))
            .collect()
    }

    fn aabb(&self) -> AABB {
        let mut aabb = AABB::new(self.pv.pos, Vec2::new(5.0, 5.0));
        self.body()
            .iter()
            .map(|b| b.corners().into_iter())
            .flatten()
            .for_each(|c| {
                aabb.include(c);
            });
        aabb
    }
}

#[derive(Resource)]
struct CraftState {
    c1: RigidBody,
    c2: RigidBody,
}

impl Default for CraftState {
    fn default() -> Self {
        CraftState {
            c1: RigidBody::new(((-1500.0, 20.0), (0.0, 0.0))),
            c2: RigidBody::new(((1500.0, -30.0), (0.0, 0.0))),
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(CraftState::default());
}

#[derive(Debug)]
struct CollisionInfo {
    part1: usize,
    part2: usize,
    b1: PV,
    b2: PV,
}

fn collision_info(r1: &RigidBody, r2: &RigidBody) -> Option<CollisionInfo> {
    for (i, b1) in r1.body().into_iter().enumerate() {
        for (j, b2) in r2.body().into_iter().enumerate() {
            if b1.intersects(b2) {
                let p1 = b1.0.center;
                let p2 = b2.0.center;
                let v1 = r1.vel(p1 - r1.pv.pos);
                let v2 = r2.vel(p2 - r2.pv.pos);

                let d = p1 - p2;
                let v = v1 - v2;

                if d.dot(v) > 0.05 {
                    continue;
                }

                return Some(CollisionInfo {
                    part1: i,
                    part2: j,
                    b1: PV::new(p1, v1),
                    b2: PV::new(p2, v2),
                });
            }
        }
    }
    None
}

fn update(mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();
    state.c1.update(dt);
    state.c2.update(dt);

    for _ in 0..10 {
        if let Some(ci) = collision_info(&state.c1, &state.c2) {
            let dv = ci.b1.vel.distance(ci.b2.vel);
            let f = (ci.b1.pos - ci.b2.pos).normalize_or_zero() * dv * 5.0;
            apply_world_force(&mut state.c1, dt, f, ci.b1.pos);
            apply_world_force(&mut state.c2, dt, -f, ci.b2.pos);
        }
    }
}

fn integer_lattice_around(p: Vec2, w: i32, step: usize) -> Vec<Vec2> {
    let pf = Vec2::new(
        ((p.x / step as f32) as i32 * step as i32) as f32,
        ((p.y / step as f32) as i32 * step as i32) as f32,
    );

    let mut ret = vec![];
    for x in (-w..=w).step_by(step) {
        for y in (-w..=w).step_by(step) {
            ret.push(Vec2::new(x as f32, y as f32) + pf)
        }
    }
    ret
}

fn draw_rigid_body(gizmos: &mut Gizmos, craft: &RigidBody) {
    let body = craft.body();

    draw_circle(gizmos, craft.pv.pos, 30.0, WHITE);
    gizmos.line_2d(craft.pv.pos, craft.pv.pos + craft.pv.vel * 5.0, PURPLE);
    let u = rotate(Vec2::X, craft.angle);
    gizmos.line_2d(craft.pv.pos, craft.pv.pos + u * 1000.0, GREEN);

    draw_aabb(gizmos, craft.aabb(), alpha(GRAY, 0.1));

    for b in &body {
        draw_obb(gizmos, b, WHITE);
    }

    for b in &body {
        for p in b.corners() {
            let v = craft.vel(p - craft.pv.pos);
            gizmos.line_2d(p, p + v, alpha(ORANGE, 0.2));
        }
    }
}

fn draw_intersections(gizmos: &mut Gizmos, r1: &RigidBody, r2: &RigidBody) {
    for b1 in r1.body() {
        for b2 in r2.body() {
            if b1.intersects(b2) {
                draw_obb(gizmos, &b1, RED);
                draw_obb(gizmos, &b2, PURPLE);
            }
        }
    }
}

fn draw(mut gizmos: Gizmos, state: Res<CraftState>) {
    draw_rigid_body(&mut gizmos, &state.c1);
    draw_rigid_body(&mut gizmos, &state.c2);

    let b1 = state.c1.body();
    let b2 = state.c2.body();

    for p in integer_lattice_around(Vec2::ZERO, 8000, 150) {
        if b1.iter().chain(b2.iter()).all(|b: &OBB| !b.contains(p)) {
            draw_square(&mut gizmos, p, 3.0, GRAY);
        }
    }

    if let Some(info) = collision_info(&state.c1, &state.c2) {
        gizmos.line_2d(info.b1.pos, info.b2.pos, RED);
    }

    draw_intersections(&mut gizmos, &state.c1, &state.c2);
}

fn apply_world_force(craft: &mut RigidBody, dt: f32, force: Vec2, location: Vec2) {
    let lever = location - craft.pv.pos;
    let torque = lever.extend(0.0).cross(force.extend(0.0)).z;
    craft.pv.vel += force / craft.mass() * dt;
    craft.angular_rate += torque / craft.moi() * dt;
}

fn do_rcs_commands(
    craft: &mut RigidBody,
    dt: f32,
    keys: &Res<ButtonInput<KeyCode>>,
    keymapping: [KeyCode; 6],
) {
    let dv = rotate(Vec2::X, craft.angle) * 700.0 * dt;
    let ds = rotate(dv, PI / 2.0);
    let da = 2.0 * dt;

    if keys.pressed(keymapping[0]) {
        craft.pv.vel += dv;
    }
    if keys.pressed(keymapping[1]) {
        craft.pv.vel += ds;
    }
    if keys.pressed(keymapping[2]) {
        craft.pv.vel -= dv;
    }
    if keys.pressed(keymapping[3]) {
        craft.pv.vel -= ds;
    }
    if keys.pressed(keymapping[4]) {
        craft.angular_rate += da;
    }
    if keys.pressed(keymapping[5]) {
        craft.angular_rate -= da;
    }
}

fn update_keys(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();

    do_rcs_commands(
        &mut state.c1,
        dt,
        &keys,
        [
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyS,
            KeyCode::KeyD,
            KeyCode::KeyQ,
            KeyCode::KeyE,
        ],
    );

    do_rcs_commands(
        &mut state.c2,
        dt,
        &keys,
        [
            KeyCode::KeyI,
            KeyCode::KeyJ,
            KeyCode::KeyK,
            KeyCode::KeyL,
            KeyCode::KeyU,
            KeyCode::KeyO,
        ],
    );
}
