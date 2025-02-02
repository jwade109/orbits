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
    .collect()
}

impl RigidBody {
    fn new(pv: impl Into<PV>) -> Self {
        RigidBody {
            pv: pv.into(),
            angle: rand(0.0, PI * 2.0),
            angular_rate: 0.0,
            body: rigid_body_mesh(3.0),
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

    fn moi(&self) -> f32 {
        2000.0
    }

    fn body(&self) -> Vec<OBB> {
        self.body
            .iter()
            .map(|e| {
                e.rotate_about(Vec2::ZERO, self.angle + PI / 2.0)
                    .offset(self.pv.pos)
            })
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
            c1: RigidBody::new(((-1500.0, 20.0), (40.0, -2.0))),
            c2: RigidBody::new(((1500.0, -30.0), (0.0, 0.0))),
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(CraftState::default());
}

fn update(mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();
    state.c1.update(dt);
    state.c2.update(dt);
}

fn draw_rigid_body(gizmos: &mut Gizmos, craft: &RigidBody) {
    draw_circle(gizmos, craft.pv.pos, 30.0, WHITE);
    gizmos.line_2d(craft.pv.pos, craft.pv.pos + craft.pv.vel * 5.0, PURPLE);
    let u = rotate(Vec2::X, craft.angle);
    gizmos.line_2d(craft.pv.pos, craft.pv.pos + u * 1000.0, GREEN);

    draw_aabb(gizmos, craft.aabb(), GRAY);

    for b in &craft.body() {
        draw_obb(gizmos, b, TEAL);
    }
}

fn do_intersection(gizmos: &mut Gizmos, r1: &RigidBody, r2: &RigidBody) {
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

    do_intersection(&mut gizmos, &state.c1, &state.c2);
}

fn apply_vehicle_fixed_force(craft: &mut RigidBody, dt: f32, force: Vec2, lever: Vec2) {
    let force = rotate(force, craft.angle);
    let torque = lever.extend(0.0).cross(force.extend(0.0)).z;

    craft.pv.vel += force / craft.mass() * dt;
    craft.angular_rate += torque / craft.moi() * dt;
}

fn update_keys(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();
    let dv = rotate(Vec2::X, state.c1.angle) * 400.0 * dt;
    let da = 1.3 * dt;

    if keys.pressed(KeyCode::KeyW) {
        state.c1.pv.vel += dv;
    }
    if keys.pressed(KeyCode::KeyA) {
        state.c1.angular_rate += da;
    }
    if keys.pressed(KeyCode::KeyD) {
        state.c1.angular_rate -= da;
    }
    if keys.pressed(KeyCode::KeyS) {
        state.c1.pv.vel -= dv;
    }

    if keys.pressed(KeyCode::KeyT) {
        apply_vehicle_fixed_force(&mut state.c1, dt, Vec2::X * 4000.0, Vec2::ZERO);
    }

    let dv = rotate(Vec2::X, state.c2.angle) * 700.0 * dt;

    if keys.pressed(KeyCode::ArrowUp) {
        state.c2.pv.vel += dv;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        state.c2.angular_rate += da;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        state.c2.angular_rate -= da;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        state.c2.pv.vel -= dv;
    }
}
