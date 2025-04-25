use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;

use crate::camera_controls::*;
use crate::debug::*;
use crate::drawing::*;

use starling::prelude::*;

pub struct CraftPlugin;

impl Plugin for CraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(
            Update,
            (
                update_keys,
                handle_viewport_input,
                update,
                log_system_info,
                draw,
            )
                .chain(),
        );
    }
}

#[derive(Resource)]
struct CraftState {
    bodies: Vec<RigidBody>,
    collisions: Vec<CollisionInfo>,
    camera: OrbitalCameraState,
    track_list: Vec<ObjectId>,
    highlighted: Vec<ObjectId>,
}

impl CraftState {
    fn lookup(&self, id: ObjectId) -> Option<&RigidBody> {
        self.bodies.iter().find(|b| b.id == id)
    }

    fn lookup_mut(&mut self, id: ObjectId) -> Option<&mut RigidBody> {
        self.bodies.iter_mut().find(|b| b.id == id)
    }
}

impl Default for CraftState {
    fn default() -> Self {
        CraftState {
            bodies: vec![
                RigidBody::new(
                    ObjectId(0),
                    ((-1500.0, 20.0), (0.0, 0.0)),
                    satellite_body(4.0),
                ),
                RigidBody::new(
                    ObjectId(1),
                    ((1500.0, -30.0), (0.0, 0.0)),
                    satellite_body(3.6),
                ),
                RigidBody::new(
                    ObjectId(2),
                    PV::zero(),
                    vec![AABB::from_arbitrary((-200.0, -200.0), (200.0, 200.0))],
                ),
                RigidBody::new(
                    ObjectId(3),
                    ((0.0, 900.0), (0.0, 0.0)),
                    vec![AABB::from_arbitrary((-150.0, -150.0), (150.0, 150.0))],
                ),
            ],
            collisions: vec![],
            camera: OrbitalCameraState::default(),
            track_list: vec![],
            highlighted: vec![],
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(CraftState::default());
}

#[derive(Debug, Clone, Copy)]
struct CollisionInfo {
    location: Vec2,
    b1: (ObjectId, usize, PV),
    b2: (ObjectId, usize, PV),
}

fn collision_info(r1: &RigidBody, r2: &RigidBody) -> Option<CollisionInfo> {
    for (i, b1) in r1.body().into_iter().enumerate() {
        for (j, b2) in r2.body().into_iter().enumerate() {
            if let Some(p) = b1.intersects(b2) {
                let p1 = b1.0.center;
                let p2 = b2.0.center;
                let v1 = r1.vel(p);
                let v2 = r2.vel(p);

                let d = p1 - p2;
                let v = v1 - v2;

                if d.dot(v) > 0.0 {
                    continue;
                }

                return Some(CollisionInfo {
                    location: p,
                    b1: (r1.id, i, PV::new(p1, v1)),
                    b2: (r2.id, j, PV::new(p2, v2)),
                });
            }
        }
    }
    None
}

fn update(mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();

    for body in &mut state.bodies {
        body.update(dt);
    }

    let mut cols = vec![];

    for (i, b1) in state.bodies.iter().enumerate() {
        for (j, b2) in state.bodies.iter().enumerate() {
            if i <= j {
                continue;
            }
            if let Some(ci) = collision_info(b1, b2) {
                cols.push(ci);
            }
        }
    }

    for col in &cols {
        let bid1 = col.b1.0;
        let bid2 = col.b2.0;
        let dv = col.b1.2.vel.distance(col.b2.2.vel);
        let f = (col.b1.2.pos - col.b2.2.pos).normalize_or_zero() * dv * 20.0;
        apply_world_force(state.lookup_mut(bid1).unwrap(), dt, f, col.location);
        apply_world_force(state.lookup_mut(bid2).unwrap(), dt, -f, col.location);
    }

    state.collisions = cols;

    state.highlighted.clear();
    if let Some(a) = state.selection_region {
        let oa = OBB::new(a, 0.0);
        state.highlighted = state
            .bodies
            .iter()
            .filter_map(|b| {
                let inter = b.body().into_iter().any(|ob| ob._intersects(oa));
                inter.then(|| b.id)
            })
            .collect();
    }

    for id in state.highlighted.clone() {
        if !state.track_list.contains(&id) {
            state.track_list.push(id);
        }
    }
}

fn draw(mut gizmos: Gizmos, state: Res<CraftState>) {
    // draw_camera_controls(&mut gizmos, &state.camera);

    for b in &state.bodies {
        draw_rigid_body(&mut gizmos, &b, WHITE);
    }

    for ci in &state.collisions {
        let b1 = &state.lookup(ci.b1.0).unwrap();
        let b2 = &state.lookup(ci.b2.0).unwrap();

        let o1 = b1.body()[ci.b1.1];
        let o2 = b2.body()[ci.b2.1];

        draw_obb(&mut gizmos, &o1, RED);
        draw_obb(&mut gizmos, &o2, RED);

        draw_circle(&mut gizmos, ci.location, 100.0, RED);
    }

    for id in &state.highlighted {
        if let Some(obj) = state.lookup(*id) {
            draw_rigid_body(&mut gizmos, obj, ORANGE);
        }
    }
}

fn apply_world_force(craft: &mut RigidBody, dt: f32, force: Vec2, location: Vec2) {
    let lever = location - craft.pv.pos;
    let torque = lever.extend(0.0).cross(force.extend(0.0)).z;
    craft.pv.vel += force / craft.mass() * dt;
    craft.angular_rate += torque / craft.moi() * dt;
}

struct KeyMapping {
    forward: KeyCode,
    back: KeyCode,
    left: KeyCode,
    right: KeyCode,
    turn_left: KeyCode,
    turn_right: KeyCode,
}

fn do_rcs_commands(
    craft: &mut RigidBody,
    dt: f32,
    keys: &ButtonInput<KeyCode>,
    keymapping: &KeyMapping,
) {
    let dv = rotate(Vec2::X, craft.angle) * 700.0 * dt;
    let ds = rotate(dv, PI / 2.0);
    let da = 2.0 * dt;

    if keys.pressed(keymapping.forward) {
        craft.pv.vel += dv;
    }
    if keys.pressed(keymapping.left) {
        craft.pv.vel += ds;
    }
    if keys.pressed(keymapping.back) {
        craft.pv.vel -= dv;
    }
    if keys.pressed(keymapping.right) {
        craft.pv.vel -= ds;
    }
    if keys.pressed(keymapping.turn_left) {
        craft.angular_rate += da;
    }
    if keys.pressed(keymapping.turn_right) {
        craft.angular_rate -= da;
    }
}

fn update_keys(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();

    if keys.just_pressed(KeyCode::KeyR) {
        *state = CraftState::default();
    }

    do_rcs_commands(
        &mut state.bodies[1],
        dt,
        &keys,
        &KeyMapping {
            forward: KeyCode::KeyI,
            back: KeyCode::KeyK,
            left: KeyCode::KeyU,
            right: KeyCode::KeyO,
            turn_left: KeyCode::KeyJ,
            turn_right: KeyCode::KeyL,
        },
    );
}

fn handle_viewport_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut state: ResMut<CraftState>,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    query: Query<&mut Transform, With<Camera>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let scroll_events = scroll.read().collect::<Vec<_>>();
    if !keys.pressed(KeyCode::ShiftLeft) {
        state.camera.on_scroll(&scroll_events);
    }
    state.camera.on_mouse_click(&buttons);
    state.camera.on_mouse_move(windows);
}

fn log_system_info(state: Res<CraftState>, mut evt: EventWriter<DebugLog>) {
    send_log(&mut evt, &format!("Tracked: {:?}", state.track_list));
    send_log(&mut evt, &format!("Highlighted: {:?}", state.highlighted));
}
